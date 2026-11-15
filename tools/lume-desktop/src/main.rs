//! Desktop harness entry point.

use lume_desktop::demo::{self, Setup};
use lume_desktop::{cli, png_out, screenshot};

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(2);
}

/// The keys that lead to the key mapping screen from the Home tab, plus the ones of a state of it
/// (`keymap-menu`, `keymap-listen`, `keymap-reserved`, `keymap-picker`, `keymap-pick-app`,
/// `keymap-confirm`), and whether some shortcuts are mapped first (`-mapped` at the end).
fn keymap_route(name: &str) -> Option<(String, bool)> {
    let (name, mapped) = match name.strip_suffix("-mapped") {
        Some(rest) => (rest, true),
        None => (name, false),
    };
    let base = "up,right,down,down,right,ok,down,ok";
    // Downs from the first row to a line: the second command, then the mapped shortcuts of
    // `map_some_keys` (3 settings, 2 apps) in front of each "add" row.
    let (settings, apps) = if mapped { (3, 2) } else { (0, 0) };
    let downs = |n: usize| ",down".repeat(n);
    let state = match name.strip_prefix("keymap")?.strip_prefix('-') {
        None => String::new(),
        // The context command's row, and its menu.
        Some("menu") => ",ok".to_string(),
        // "Add key" of that menu opens the listening; OK is a reserved key, so it warns.
        Some("listen") => ",ok,ok".to_string(),
        Some("reserved") => ",ok,ok,ok".to_string(),
        // The "Add shortcut" row: the list of the shortcuts without a key.
        Some("picker") => format!("{},ok", downs(2 + settings)),
        Some("pick-app") => format!("{},ok", downs(3 + settings + apps)),
        Some("confirm") => format!("{},ok", downs(4 + settings + apps)),
        Some(_) => return None,
    };
    Some((format!("{base}{state}"), mapped))
}

fn main() {
    let options = match cli::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("error: {message}\n");
            }
            eprint!("{}", cli::USAGE);
            std::process::exit(if message.is_empty() { 0 } else { 2 });
        }
    };

    let Some((screen, path)) = &options.screenshot else {
        let setup = Setup {
            locale: options.locale.clone(),
            pseudo_locale: options.pseudo_locale,
            is_24h: !options.clock_12h,
            favorites: options.favorites,
        };
        // LUME_EXIT_AFTER_MS closes the window by itself, for smoke tests.
        let exit_after = std::env::var("LUME_EXIT_AFTER_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(std::time::Duration::from_millis);
        if let Err(message) = lume_desktop::window::run(
            options.width,
            options.height,
            demo::app_with(&setup),
            exit_after,
        ) {
            eprintln!("error: {message}");
            std::process::exit(1);
        }
        return;
    };
    let (width, height) = (options.width, options.height);

    let result = match screen.as_str() {
        "empty" => screenshot::save(path, width, height, screenshot::draw_empty),
        "logo" => screenshot::save(path, width, height, screenshot::draw_logo),
        "banner" => screenshot::save(path, width, height, screenshot::draw_banner),
        "home" | "apps" | "settings" | "tvinfo" | "about" | "editor" | "tools" | "tool-display"
        | "tool-picture" | "tool-remote" | "tool-audio" | "tool-storage" | "receive" => {
            let setup = Setup {
                locale: options.locale.clone(),
                pseudo_locale: options.pseudo_locale,
                is_24h: !options.clock_12h,
                favorites: options.favorites,
            };
            let mut app = demo::app_with(&setup);
            // The keys that lead to each screen from the Home tab.
            let to_settings = "up,right,down,down,right,ok";
            let route = match screen.as_str() {
                "apps" => "up,right,ok",
                "settings" => to_settings,
                "tvinfo" => "up,right,down,down,right,ok,down,down,ok",
                "about" => "up,right,down,down,right,ok,down,down,down,down,ok",
                "editor" => "up,right,down,down,right,ok,ok",
                // The editor, then its last mini-card ("+ From phone").
                "receive" => "up,right,down,down,right,ok,ok,left,ok",
                // The Tools menu, then each tool by its place in the menu.
                "tools" => "up,right,down,down,right,ok,down,down,down,ok",
                "tool-display" => "up,right,down,down,right,ok,down,down,down,ok,ok",
                "tool-picture" => "up,right,down,down,right,ok,down,down,down,ok,down,ok",
                "tool-remote" => "up,right,down,down,right,ok,down,down,down,ok,down,down,ok",
                "tool-audio" => "up,right,down,down,right,ok,down,down,down,ok,down,down,down,ok",
                "tool-storage" => {
                    "up,right,down,down,right,ok,down,down,down,ok,down,down,down,down,ok"
                }
                _ => "",
            };
            let mut keys = demo::parse_keys(route).unwrap_or_else(|e| fail(&e));
            if let Some(list) = &options.keys {
                keys.extend(demo::parse_keys(list).unwrap_or_else(|e| fail(&e)));
            }
            screenshot::render_app(width, height, &mut app, &keys, options.splash)
                .and_then(|pixels| png_out::write_rgba(path, width, height, &pixels))
        }
        // Serves the Receive image screen for real (a phone or a browser on this network can send a
        // picture), then shoots the Adjust screen it opens. LUME_SERVE_SECS: how long to wait (120).
        "receive-live" => {
            let setup = Setup {
                locale: options.locale.clone(),
                pseudo_locale: options.pseudo_locale,
                is_24h: !options.clock_12h,
                favorites: options.favorites,
            };
            let mut app = demo::app_with(&setup);
            let keys = demo::parse_keys(
                "up,right,down,down,right,ok,ok,right,right,right,right,right,right,ok",
            )
            .unwrap_or_else(|e| fail(&e));
            let secs = std::env::var("LUME_SERVE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120);
            screenshot::serve_receive(
                width,
                height,
                &mut app,
                &keys,
                std::time::Duration::from_secs(secs),
            )
            .and_then(|pixels| png_out::write_rgba(path, width, height, &pixels))
        }
        "confirm" => {
            let setup = Setup {
                locale: options.locale.clone(),
                pseudo_locale: options.pseudo_locale,
                is_24h: !options.clock_12h,
                favorites: options.favorites,
            };
            let mut app = demo::app_with(&setup);
            let image = match &options.image {
                Some(path) => {
                    let (width, height, rgb) =
                        png_out::read_rgb(path).unwrap_or_else(|e| fail(&e));
                    if (width, height) != (1920, 1080) {
                        fail("the phone sends 1920x1080: --image must have that size");
                    }
                    lume_core::wallpaper::Image { width, height, rgb: rgb.into() }
                }
                None => demo::sample_photo(),
            };
            // The editor first, so Cancel and Apply go back to it as on the TV.
            let mut keys = demo::parse_keys("up,right,down,down,right,ok,ok").unwrap_or_else(|e| fail(&e));
            let mut extra = Vec::new();
            if let Some(list) = &options.keys {
                extra = demo::parse_keys(list).unwrap_or_else(|e| fail(&e));
            }
            screenshot::render_app_then(width, height, &mut app, &keys.split_off(0), options.splash, |app| {
                app.confirm_with(image.clone());
            }, &extra)
                .and_then(|pixels| png_out::write_rgba(path, width, height, &pixels))
        }
        other if other.starts_with("keymap") => {
            let Some((route, mapped)) = keymap_route(other) else {
                fail(&format!(
                    "unknown key mapping state '{other}' (keymap[-menu|-listen|-reserved|-picker|-pick-app|-confirm][-mapped])"
                ))
            };
            let setup = Setup {
                locale: options.locale.clone(),
                pseudo_locale: options.pseudo_locale,
                is_24h: !options.clock_12h,
                favorites: options.favorites,
            };
            let mut app = demo::app_with(&setup);
            if mapped {
                demo::map_some_keys(&mut app);
            }
            let mut keys = demo::parse_keys(&route).unwrap_or_else(|e| fail(&e));
            if let Some(list) = &options.keys {
                keys.extend(demo::parse_keys(list).unwrap_or_else(|e| fail(&e)));
            }
            screenshot::render_app(width, height, &mut app, &keys, options.splash)
                .and_then(|pixels| png_out::write_rgba(path, width, height, &pixels))
        }
        other => fail(&format!(
            "unknown screen '{other}' (available: empty, logo, banner, home, apps, settings, tvinfo, about, editor, keymap, tools, tool-display, tool-picture, tool-remote, tool-audio, tool-storage, receive, confirm)"
        )),
    };
    match result {
        Ok(()) => println!("wrote {} ({width}x{height})", path.display()),
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(1);
        }
    }
}
