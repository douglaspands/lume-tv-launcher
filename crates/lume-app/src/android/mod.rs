//! The Android event loop.
//!
//! The loop blocks in `poll_events` until a key arrives, the window or the activity changes,
//! or the next deadline the launcher reported (an animation, the clock's minute).
//! A frame is drawn only when something changed, and everything that holds GPU memory is
//! released when the window goes away.

mod bridge;
mod device_info;
mod egl;

use android_activity::input::{InputEvent, KeyAction};
use android_activity::{AndroidApp, AndroidAppWaker, InputStatus, MainEvent, PollEvent};
use bridge::{unix_seconds, Bridge};
use egl::Graphics;
use lume_core::app::{Effect, Key};
use lume_core::app_icons as icons;
use lume_core::render::Backend;
use lume_core::shell::protocol::Changed;
use lume_core::shell::{keys, Shell};
use lume_core::theme::{vars, Color};
use std::collections::HashSet;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Set by Java when the Home button is pressed while lume is already open.
static HOME_PRESSED: AtomicBool = AtomicBool::new(false);
static WAKER: Mutex<Option<AndroidAppWaker>> = Mutex::new(None);

/// Called by `LumeActivity.onNewIntent` (Java thread) to say Home was pressed.
#[no_mangle]
pub extern "system" fn Java_app_lume_tv_LumeActivity_nativeHomePressed(
    _env: *mut c_void,
    _class: *mut c_void,
) {
    HOME_PRESSED.store(true, Ordering::SeqCst);
    if let Ok(guard) = WAKER.lock() {
        if let Some(waker) = guard.as_ref() {
            waker.wake();
        }
    }
}

/// Icons fetched per call, so keys are still answered while they load.
const ICONS_PER_TURN: usize = 1;
/// How long a turn of the loop may load icons: while the splash waits for them (nothing moves and
/// nobody sees them come), with the interface at rest (one frame, so a key is not kept waiting),
/// and while a faint fade runs (the splash logo or an icon coming in: a slice of a frame).
const SPLASH_BATCH: Duration = Duration::from_millis(40);
const IDLE_BATCH: Duration = Duration::from_millis(16);
const FADE_BATCH: Duration = Duration::from_millis(6);
/// No icon is started this close to the splash's deadline (one can take this long).
const SPLASH_MARGIN_MS: u64 = 60;

/// `mallopt` option that returns freed heap pages to the system (bionic, Android 10+).
const M_PURGE: libc::c_int = -101;

extern "C" {
    /// Bionic's allocator tuning call.
    fn mallopt(option: libc::c_int, value: libc::c_int) -> libc::c_int;
}

/// Gives freed memory back to the system, so the process stays small while in the background.
fn purge_memory() {
    // SAFETY: mallopt only adjusts allocator behavior; unsupported options are ignored.
    unsafe {
        mallopt(M_PURGE, 0);
    }
}

/// Where the time of loading the icons of a window goes: fetching them from the platform (the
/// bridge) and uploading them to the GPU.
#[derive(Default)]
struct IconTimes {
    /// When the window appeared, until the log line is written.
    from: Option<Instant>,
    count: u32,
    bridge: Duration,
    upload: Duration,
    /// The slowest single fetch.
    slowest: Duration,
}

impl IconTimes {
    fn fetched(&mut self, took: Duration) {
        self.count += 1;
        self.bridge += took;
        self.slowest = self.slowest.max(took);
    }
}

/// The state of the app while its event loop runs.
struct Runtime {
    app: AndroidApp,
    bridge: Bridge,
    shell: Shell,
    graphics: Option<Graphics>,
    /// Apps whose icon could not be loaded in this window, so they are not asked again.
    icons_failed: HashSet<String>,
    /// Apps whose dock icon the platform could not give.
    dock_failed: HashSet<String>,
    /// Icons are still missing: the loop does not wait for events.
    icons_pending: bool,
    started: Instant,
    /// When the current window appeared, until its first frame is drawn and logged.
    first_frame_from: Option<Instant>,
    /// How the icons of the current window were loaded, logged once they all are.
    icon_times: IconTimes,
    /// When the frames of the reveal after the splash were shown, to log how many were late.
    reveal_frames: Vec<Instant>,
    /// When the current window appeared, until the interface is ready (the splash ended) and logged.
    ready_from: Option<Instant>,
    /// Size of the render buffer, once a window exists.
    running: bool,
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("lume"),
    );
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
    log::info!("lume {} starting", lume_version::VERSION);
    *WAKER.lock().expect("waker lock") = Some(app.create_waker());

    let Some(files) = app.internal_data_path() else {
        log::error!("no internal data directory");
        return;
    };
    let bridge = Bridge::new(&app);
    let package = bridge.package_name();
    let mut shell = Shell::start(
        &files,
        &package,
        &bridge.locales(),
        bridge.clock(),
        &mut || bridge.scan_all(None),
    );

    shell.set_inputs_available(bridge.inputs_available());

    let mut runtime = Runtime {
        app: app.clone(),
        bridge,
        shell,
        graphics: None,
        icons_failed: HashSet::new(),
        dock_failed: HashSet::new(),
        icons_pending: false,
        started: Instant::now(),
        first_frame_from: None,
        ready_from: None,
        reveal_frames: Vec::new(),
        icon_times: IconTimes::default(),
        running: true,
    };
    runtime.run();
    log::info!("lume stopped");
}

impl Runtime {
    fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    fn run(&mut self) {
        while self.running {
            let busy = self.shell.is_busy(self.now_ms());
            let timeout = if self.icons_pending && !busy {
                Some(Duration::ZERO)
            } else {
                // The statistics are read only while the settings are on screen.
                [
                    self.shell.next_timeout(self.now_ms()),
                    self.shell.stats_timeout(self.now_ms()),
                ]
                .into_iter()
                .flatten()
                .min()
            };
            let app = self.app.clone();
            app.poll_events(timeout, |event| self.on_event(event));
            self.process_input();
            if HOME_PRESSED.swap(false, Ordering::SeqCst) {
                self.send_key(Key::Home);
            }
            // The clock's seconds move on by themselves; the shell only asks for a frame when the minute turns.
            self.shell.tick_clock(unix_seconds());
            if self.graphics.is_some() && self.shell.frame_due(self.now_ms()) {
                self.draw();
            }
            let now = self.now_ms();
            if self.shell.stats_due(now) {
                if let Some(stats) = self.bridge.stats() {
                    self.shell.set_stats(stats, now);
                } else {
                    // Try again in a while instead of on every turn of the loop.
                    self.shell.stats_failed(now);
                }
            }
            if self.shell.device_info_due(now) {
                let gpu = self.graphics.as_mut().map(|g| g.backend().gpu_strings());
                let info = device_info::read(gpu, self.bridge.display());
                self.shell.set_device_info(info);
            }
            // Loading an icon takes tens of milliseconds: it waits for the animation to end, or the
            // frames would be late and the movement would stutter.
            if !self.shell.is_busy(self.now_ms()) {
                self.prepare_wallpaper();
                self.load_icons_batch();
            }
        }
    }

    fn on_event(&mut self, event: PollEvent<'_>) {
        let PollEvent::Main(event) = event else {
            return;
        };
        match event {
            MainEvent::InitWindow { .. } => self.create_window(),
            MainEvent::TerminateWindow { .. } => self.destroy_window(),
            MainEvent::WindowResized { .. } | MainEvent::RedrawNeeded { .. } => {
                if self.graphics.is_some() {
                    self.create_window_size();
                }
            }
            MainEvent::Resume { .. } => self.on_resume(),
            MainEvent::ConfigChanged { .. } => self.refresh_system_settings(),
            // Anything that keeps GPU memory goes away as soon as lume is not on screen.
            MainEvent::Stop => self.destroy_window(),
            MainEvent::LowMemory => {
                if self.graphics.is_none() {
                    purge_memory();
                }
            }
            MainEvent::Destroy => self.running = false,
            _ => {}
        }
    }

    /// Creates the EGL context and the renderer for the new window.
    fn create_window(&mut self) {
        let Some(window) = self.app.native_window() else {
            return;
        };
        // Render at most 1920x1080; the system scales the buffer up on bigger screens.
        let (width, height) =
            Shell::render_size(window.width().max(1) as u32, window.height().max(1) as u32);
        if let Err(e) = window.set_buffers_geometry(
            width as i32,
            height as i32,
            Some(ndk::hardware_buffer_format::HardwareBufferFormat::R8G8B8A8_UNORM),
        ) {
            log::warn!("cannot set the buffer size: {e}");
        }
        match Graphics::new(window.ptr().as_ptr().cast()) {
            Ok(graphics) => {
                self.graphics = Some(graphics);
                self.first_frame_from = Some(Instant::now());
                self.ready_from = self.first_frame_from;
                self.icon_times = IconTimes {
                    from: self.first_frame_from,
                    ..IconTimes::default()
                };
                self.shell.wake(width, height);
                log::info!("window {width}x{height}");
            }
            Err(e) => log::error!("cannot start the renderer: {e}"),
        }
    }

    /// The window changed size: the buffer size and the scale are recalculated.
    fn create_window_size(&mut self) {
        let Some(window) = self.app.native_window() else {
            return;
        };
        let (width, height) =
            Shell::render_size(window.width().max(1) as u32, window.height().max(1) as u32);
        let _ = window.set_buffers_geometry(
            width as i32,
            height as i32,
            Some(ndk::hardware_buffer_format::HardwareBufferFormat::R8G8B8A8_UNORM),
        );
        self.shell.wake(width, height);
    }

    /// Releases the renderer, the glyph atlas and the heap pages they used.
    fn destroy_window(&mut self) {
        let had_graphics = self.graphics.take().is_some();
        self.icons_failed.clear();
        self.dock_failed.clear();
        self.icons_pending = false;
        self.shell.sleep();
        if had_graphics {
            purge_memory();
            log::info!("window gone: renderer, glyph atlas and fonts released (dormant)");
        }
    }

    fn on_resume(&mut self) {
        self.refresh_system_settings();
        self.sync_catalog();
    }

    /// Language, clock format and time zone may have changed while lume was away.
    fn refresh_system_settings(&mut self) {
        self.shell.set_locales(&self.bridge.locales());
        self.shell.set_clock(self.bridge.clock());
    }

    /// Picks up apps installed, updated or removed since the last time.
    fn sync_catalog(&mut self) {
        let now = unix_seconds();
        match self.bridge.changed() {
            Changed::Unknown => {
                let discovered = self.bridge.scan_all(Some(&self.shell.app().catalog));
                self.shell.rescan(&discovered, now);
            }
            Changed::Packages(packages) if packages.is_empty() => {}
            Changed::Packages(packages) => {
                let (mut changed, mut removed) = (Vec::new(), Vec::new());
                for package in packages {
                    let entries = self.bridge.query(&package);
                    if entries.is_empty() {
                        removed.push(package);
                    } else {
                        changed.extend(entries);
                    }
                }
                self.shell.apply_changes(&changed, &removed, now);
            }
        }
    }

    fn process_input(&mut self) {
        let mut pressed = Vec::new();
        match self.app.input_events_iter() {
            Ok(mut iter) => loop {
                let more = iter.next(|event| match event {
                    InputEvent::KeyEvent(key) => {
                        let code: u32 = key.key_code().into();
                        match keys::from_android(code as i32) {
                            Some(mapped) => {
                                if key.action() == KeyAction::Down {
                                    pressed.push(mapped);
                                }
                                InputStatus::Handled
                            }
                            None => {
                                // Which code a remote sends for a button lume does not use, for
                                // mapping remotes that differ (see docs/device-notes.md).
                                if key.action() == KeyAction::Down {
                                    log::info!("key {code} not used");
                                }
                                InputStatus::Unhandled
                            }
                        }
                    }
                    _ => InputStatus::Unhandled,
                });
                if !more {
                    break;
                }
            },
            Err(e) => log::warn!("no input events: {e:?}"),
        }
        for key in pressed {
            self.send_key(key);
        }
    }

    fn send_key(&mut self, key: Key) {
        let now = self.now_ms();
        for effect in self.shell.key(key, now) {
            self.run_effect(effect);
        }
    }

    fn run_effect(&mut self, effect: Effect) {
        match effect {
            Effect::OpenApp { package, activity } => {
                if !self.bridge.start_app(&package, &activity) {
                    // It was uninstalled meanwhile: say so, and refresh the catalog.
                    let now = self.now_ms();
                    self.shell.app_not_found(now);
                    let discovered = self.bridge.scan_all(Some(&self.shell.app().catalog));
                    self.shell.rescan(&discovered, unix_seconds());
                }
            }
            Effect::OpenAndroidSettings => {
                if !self.bridge.open_android_settings() {
                    log::warn!("the Android settings could not be opened");
                }
            }
            Effect::OpenInputs => {
                if !self.bridge.open_inputs() {
                    log::warn!("the TV inputs panel could not be opened");
                }
            }
            Effect::OpenAppInfo { package } => {
                if !self.bridge.open_app_info(&package) {
                    log::warn!("the app info of {package} could not be opened");
                }
            }
            Effect::Uninstall { package } => {
                if !self.bridge.uninstall(&package) {
                    log::warn!("the uninstaller of {package} could not be opened");
                }
            }
            // The shell already saved these.
            Effect::SaveConfig | Effect::SaveUsage => {}
        }
    }

    /// While the splash is on screen (it does not draw the wallpaper), uploads the wallpaper texture
    /// for the reveal: uploaded by the reveal's first frame, it made that frame ~70 ms late.
    fn prepare_wallpaper(&mut self) {
        if let Some(graphics) = self.graphics.as_mut() {
            if self.shell.app().in_splash() && !graphics.backend().has_wallpaper() {
                graphics
                    .backend()
                    .ensure_wallpaper(self.shell.app().config.wallpaper);
            }
        }
    }

    /// Loads icons for a turn of the loop, as many as fit in the turn's budget (see
    /// [`SPLASH_BATCH`]); the screen is drawn once for all of them. Loading one icon per turn cost a
    /// frame per icon, which is what made them appear one by one. During the splash it stops in time
    /// for the deadline, which an icon on its way would delay.
    fn load_icons_batch(&mut self) {
        let now = self.now_ms();
        let budget = if self.shell.app().is_animating(now) {
            FADE_BATCH
        } else if self.shell.app().in_splash() {
            SPLASH_BATCH
        } else {
            IDLE_BATCH
        };
        let start = Instant::now();
        loop {
            self.load_icons();
            let time_left = self.shell.splash_time_left(self.now_ms());
            if !self.icons_pending
                || start.elapsed() >= budget
                || time_left.is_some_and(|left| left < SPLASH_MARGIN_MS)
            {
                break;
            }
        }
    }

    /// Fetches the next few missing icons and uploads them to the icon texture.
    fn load_icons(&mut self) {
        let now = self.now_ms();
        let Some(graphics) = self.graphics.as_mut() else {
            self.icons_pending = false;
            return;
        };
        // The dock is what Home shows: its icons come first, a couple per turn, so the first frames
        // are drawn right away with the app color and initial.
        let dock = self
            .shell
            .dock_icons_wanted(&self.dock_failed, ICONS_PER_TURN);
        for (package, activity) in &dock {
            let fetch = Instant::now();
            let pixels = self.bridge.dock_icon(package, activity);
            self.icon_times.fetched(fetch.elapsed());
            let region = pixels.and_then(|pixels| {
                self.shell
                    .reserve_dock_icon(package, now)
                    .map(|r| (r, pixels))
            });
            match region {
                Some((region, pixels)) => {
                    let upload = Instant::now();
                    let backend = graphics.backend();
                    backend.ensure_image(icons::ATLAS_WIDTH, icons::ATLAS_HEIGHT);
                    backend.update_image(region, &pixels);
                    self.icon_times.upload += upload.elapsed();
                }
                None => {
                    self.dock_failed.insert(package.clone());
                    self.shell.icon_failed(package, true);
                }
            }
        }
        if !dock.is_empty() {
            self.icons_pending = true;
            return;
        }
        let wanted = self.shell.icons_wanted(&self.icons_failed, ICONS_PER_TURN);
        for (package, activity) in &wanted {
            let fetch = Instant::now();
            let art = self
                .bridge
                .tile_art(package, activity)
                .filter(|(p, _)| p.len() == (icons::CELL_WIDTH * icons::CELL_HEIGHT * 4) as usize);
            self.icon_times.fetched(fetch.elapsed());
            let region = art
                .as_ref()
                .and_then(|(_, art)| self.shell.reserve_icon(package, *art, now));
            match (art, region) {
                (Some((pixels, _)), Some(region)) => {
                    let upload = Instant::now();
                    let backend = graphics.backend();
                    backend.ensure_image(icons::ATLAS_WIDTH, icons::ATLAS_HEIGHT);
                    backend.update_image(region, &pixels);
                    self.icon_times.upload += upload.elapsed();
                }
                _ => {
                    self.icons_failed.insert(package.clone());
                    self.shell.icon_failed(package, false);
                }
            }
        }
        self.icons_pending = !self.shell.icons_wanted(&self.icons_failed, 1).is_empty()
            || !self
                .shell
                .dock_icons_wanted(&self.dock_failed, 1)
                .is_empty();
        if !self.icons_pending {
            let times = &mut self.icon_times;
            if let Some(from) = times.from.take() {
                log::info!(
                    "icons: {} in {} ms after the window (bridge {} ms, slowest {} ms; upload {} ms)",
                    times.count,
                    from.elapsed().as_millis(),
                    times.bridge.as_millis(),
                    times.slowest.as_millis(),
                    times.upload.as_millis()
                );
            }
        }
    }

    fn draw(&mut self) {
        let now = self.now_ms();
        // The frame that ends the reveal is drawn after it stops being "revealing": it counts too.
        let revealing = self.shell.is_revealing();
        let Some(frame) = self.shell.build_frame(now) else {
            return;
        };
        let (Some(graphics), Some(viewport)) = (self.graphics.as_mut(), self.shell.viewport())
        else {
            return;
        };
        if let Some(text) = self.shell.text_engine() {
            graphics.backend().sync_text_atlas(text);
        }
        let background = match lume_core::theme::THEME.app.base.background {
            Some(lume_core::theme::Paint::Solid(color)) => color,
            _ => vars::BG,
        };
        let _: Color = background;
        graphics.backend().render(&viewport, background, &frame);
        let swapped = graphics.egl.swap_buffers();
        if let Err(e) = &swapped {
            log::warn!("{e}");
        } else if let Some(from) = self.first_frame_from.take() {
            log::info!(
                "first frame: {} ms after the window, {} ms after the process started",
                from.elapsed().as_millis(),
                self.started.elapsed().as_millis()
            );
        }
        if swapped.is_ok() && (revealing || self.shell.is_revealing()) {
            self.reveal_frames.push(Instant::now());
        }
        if self.shell.take_interface_ready() {
            let frames = std::mem::take(&mut self.reveal_frames);
            let gaps: Vec<u128> = frames
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).as_millis())
                .collect();
            let late = gaps.iter().filter(|gap| **gap >= 33).count();
            if let Some(from) = self.ready_from.take() {
                log::info!(
                    "interface ready: {} ms after the window, {} ms after the process started; reveal: {} frames, {} of 33 ms or more {:?}",
                    from.elapsed().as_millis(),
                    self.started.elapsed().as_millis(),
                    frames.len(),
                    late,
                    gaps
                );
            }
        }
    }
}
