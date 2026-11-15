//! "Minha imagem" in the launcher: the phone card of the editor, the Receive image and Confirm
//! screens, driven by keys and frames like the other behavior tests. The pictures arrive through a
//! real server on loopback, as the phone sends them.

use super::confirm::{APPLY, CANCEL as CONFIRM_SCREEN_CANCEL};
use super::context_menu::MenuAction;
use super::editor::{CONFIRM_CANCEL, CONFIRM_RESTORE, PHONE_CARD, WALLPAPER_BASE};
use super::tests::Rig;
use super::*;
use crate::companion::server::tests::{request_to, upload_request};
use crate::companion::{Session, Status, COUNTDOWN_MS, TIMEOUT_MS};
use crate::wallpaper::custom::{self, Applied, HEIGHT, WIDTH};
use crate::wallpaper::{Image, WallpaperId};
use std::path::PathBuf;

/// This test thread's own `files/` directory, empty.
fn files(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lume-app-custom-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    custom::set_test_files_dir(Some(dir.clone()));
    dir
}

/// The tests that show a received picture share the state of "Minha imagem": one at a time.
fn lock() -> std::sync::MutexGuard<'static, ()> {
    custom::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// A 1920x1080 picture, as the phone sends it, told apart by `shade`.
fn picture(shade: u8) -> Image {
    let rgb = (0..WIDTH * HEIGHT)
        .flat_map(|i| [(i % WIDTH * 255 / WIDTH) as u8, shade, 120])
        .collect();
    Image {
        width: WIDTH,
        height: HEIGHT,
        rgb,
    }
}

fn editor(rig: &mut Rig) {
    rig.app.open_editor(false);
    rig.advance(0);
    assert_eq!(rig.app.screen(), &Screen::Editor);
}

fn focus_on(rig: &mut Rig, id: FocusId) {
    assert!(rig.app.focus.focus(id), "{id} cannot take the focus");
    rig.advance(0);
}

fn press_on(rig: &mut Rig, id: FocusId) -> Vec<Effect> {
    focus_on(rig, id);
    rig.press(Key::Ok)
}

/// Opens the Receive image screen from the editor with a server on loopback.
fn receive(rig: &mut Rig) {
    editor(rig);
    if rig.app.config.custom.is_none() {
        press_on(rig, PHONE_CARD);
    } else {
        rig.app.open_receive();
        rig.advance(0);
    }
    assert_eq!(rig.app.screen(), &Screen::ReceiveImage);
    rig.app.receive.as_mut().unwrap().session = Session::open_on_loopback(rig.now);
    rig.advance(0);
}

fn session(rig: &Rig) -> &Session {
    &rig.app
        .receive
        .as_ref()
        .expect("the phone flow is open")
        .session
}

fn server_address(rig: &Rig) -> std::net::SocketAddr {
    let address = session(rig).address().unwrap();
    let port = address
        .split('/')
        .next()
        .and_then(|a| a.rsplit(':').next())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap();
    (std::net::Ipv4Addr::LOCALHOST, port).into()
}

/// The phone sends `image`; returns once the screen took it.
fn send(rig: &mut Rig, image: &Image) {
    let body = custom::encode_qoi(image).unwrap();
    let answer = request_to(
        server_address(rig),
        &upload_request(session(rig).code(), &body),
    );
    assert!(answer.contains(r#"{"ok":true}"#), "{answer}");
    for _ in 0..100 {
        if rig.app.poll_background(rig.now).is_some()
            && custom::preview().is_some_and(|p| p.rgb == image.rgb)
        {
            rig.advance(0);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the picture did not reach the screen");
}

/// Applies the picture of the Confirm screen and waits for it.
fn apply(rig: &mut Rig) -> Vec<Effect> {
    press_on(rig, APPLY);
    let effects = rig.app.wait_apply();
    rig.advance(0);
    effects
}

/// From the editor to an applied "Minha imagem".
fn apply_picture(rig: &mut Rig, shade: u8) {
    receive(rig);
    send(rig, &picture(shade));
    apply(rig);
}

// ---- the phone card --------------------------------------------------------------------------

#[test]
fn ok_on_the_empty_phone_card_opens_receive_and_back_returns_to_the_card() {
    let mut rig = Rig::new();
    editor(&mut rig);
    assert!(rig.app.config.custom.is_none());
    press_on(&mut rig, PHONE_CARD);
    assert_eq!(rig.app.screen(), &Screen::ReceiveImage);
    assert!(rig.app.companion_active());
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Editor);
    assert_eq!(rig.focused(), Some(PHONE_CARD));
    assert!(!rig.app.companion_active());
}

#[test]
fn cancel_on_receive_returns_to_the_card() {
    let mut rig = Rig::new();
    editor(&mut rig);
    press_on(&mut rig, PHONE_CARD);
    press_on(&mut rig, super::receive::CANCEL);
    assert_eq!(rig.app.screen(), &Screen::Editor);
    assert_eq!(rig.focused(), Some(PHONE_CARD));
}

#[test]
fn ok_on_my_image_makes_it_the_wallpaper() {
    let _lock = lock();
    files("use");
    let mut rig = Rig::new();
    apply_picture(&mut rig, 10);
    rig.app.config.wallpaper = WallpaperId::MirrorLake;
    focus_on(&mut rig, WALLPAPER_BASE);
    let effects = press_on(&mut rig, PHONE_CARD);
    assert_eq!(rig.app.config.wallpaper, WallpaperId::Custom);
    assert!(effects.contains(&Effect::SaveConfig));
}

// ---- the Receive image screen ------------------------------------------------------------------

#[test]
fn without_a_network_the_screen_says_so_and_opens_no_port() {
    let mut rig = Rig::new();
    editor(&mut rig);
    press_on(&mut rig, PHONE_CARD);
    let receive = rig.app.receive.as_ref().unwrap();
    // No platform interfaces in the tests: no network.
    assert_eq!(receive.session.status(), Status::NoNetwork);
    assert!(!receive.session.is_listening());
    // It looks again within 3 s, without the user leaving.
    let now = rig.now;
    let deadline = rig.app.next_deadline(now).unwrap();
    assert!(deadline <= now + crate::companion::NETWORK_RETRY_MS);
}

#[test]
fn the_idle_receive_screen_draws_no_frame_before_the_last_minute() {
    let mut rig = Rig::new();
    receive(&mut rig);
    rig.advance(2000);
    let opened = rig.now - 2000;
    // The loop draws at each deadline, as the shell does.
    let mut frames = 0;
    while rig.now < opened + TIMEOUT_MS - COUNTDOWN_MS {
        let next = rig.app.next_deadline(rig.now).unwrap();
        assert!(next > rig.now, "a deadline in the past would spin the loop");
        rig.now = next;
        rig.frame();
        frames += 1;
    }
    // Only the top bar's clock, once a minute, at most.
    assert!(frames <= 5, "{frames} frames in 4 minutes");
    // In the last minute, the countdown: one frame a second.
    let start = rig.now;
    let mut frames = 0;
    while rig.now < start + 10_000 {
        rig.now = rig.app.next_deadline(rig.now).unwrap();
        rig.frame();
        frames += 1;
    }
    assert!((9..=12).contains(&frames), "{frames} frames in 10 s");
    rig.press(Key::Back);
    assert!(!rig.app.companion_active());
}

#[test]
fn the_screen_says_when_the_phone_is_editing() {
    let mut rig = Rig::new();
    receive(&mut rig);
    let raw = format!(
        "POST /ping?c={}&idle=3 HTTP/1.1\r\n\r\n",
        session(&rig).code()
    );
    request_to(server_address(&rig), raw.as_bytes());
    let mut changed = false;
    for _ in 0..50 {
        changed = rig.app.poll_background(rig.now).is_some();
        if changed {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(changed, "the ping changed what the screen shows");
    assert!(session(&rig).connected(rig.now));
    let (text, color) = rig.app.receive_status_text(session(&rig), rig.now);
    assert_eq!(
        text,
        rig.app
            .translator
            .get(crate::i18n::StringId::ReceiveConnected)
    );
    assert_eq!(color, crate::theme::vars::PRIMARY);
    // A second ping that changes nothing on screen asks for no frame.
    request_to(server_address(&rig), raw.as_bytes());
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(rig.app.poll_background(rig.now).is_none());
}

#[test]
fn the_server_stops_before_lume_goes_to_the_background_and_comes_back_with_a_new_code() {
    let mut rig = Rig::new();
    receive(&mut rig);
    let code = session(&rig).code().to_string();
    let port = server_address(&rig).port();
    rig.app.went_to_background();
    assert!(!session(&rig).is_listening());
    let closed = (0..50).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::net::TcpStream::connect_timeout(
            &(std::net::Ipv4Addr::LOCALHOST, port).into(),
            std::time::Duration::from_millis(200),
        )
        .is_err()
    });
    assert!(closed, "the port is still open in the background");
    rig.app.came_to_foreground();
    assert_ne!(session(&rig).code(), code);
    rig.app.go_home();
    assert!(!rig.app.companion_active());
}

#[test]
fn time_up_for_good_puts_the_focus_on_a_new_code() {
    let mut rig = Rig::new();
    receive(&mut rig);
    let mut at = rig.now;
    for _ in 0..=crate::companion::MAX_RENEWALS {
        at += TIMEOUT_MS;
        assert!(rig.app.poll_background(at).is_some());
    }
    rig.now = at;
    rig.frame();
    assert_eq!(session(&rig).status(), Status::TimedOut);
    assert_eq!(rig.focused(), Some(super::receive::NEW_CODE));
    rig.press(Key::Left);
    assert_eq!(
        rig.focused(),
        Some(super::receive::CANCEL),
        "Cancel is still reachable"
    );
    rig.app.go_home();
    assert!(!rig.app.companion_active());
}

/// The loop asks for the next deadline at every turn: the one of the countdown must not slide away
/// (it froze on the TVs), and once the loop draws at it, the next second comes.
#[test]
fn the_countdown_deadline_stays_put_until_its_frame_is_drawn() {
    let mut rig = Rig::new();
    receive(&mut rig);
    rig.now += TIMEOUT_MS - 45_500;
    rig.frame();
    let drawn = rig.now;
    let due = rig.app.next_deadline(drawn).unwrap();
    assert!(due > drawn && due <= drawn + 1000);
    for later in [drawn + 1, drawn + 400, due - 1, due, due + 300] {
        assert_eq!(rig.app.next_deadline(later), Some(due), "asked at {later}");
    }
    rig.now = due;
    rig.frame();
    let next = rig.app.next_deadline(due).unwrap();
    assert!(next > due && next <= due + 1000);
    rig.press(Key::Back);
}

// ---- the Confirm screen ------------------------------------------------------------------------

#[test]
fn a_picture_opens_confirm_with_the_focus_on_apply_and_a_new_one_replaces_it() {
    let _lock = lock();
    files("confirm");
    let mut rig = Rig::new();
    receive(&mut rig);
    send(&mut rig, &picture(1));
    assert_eq!(rig.app.screen(), &Screen::ConfirmImage);
    assert_eq!(rig.focused(), Some(APPLY));
    assert!(session(&rig).is_listening(), "the phone may send again");
    // The phone frames again and sends: the TV shows it without a key.
    send(&mut rig, &picture(2));
    assert_eq!(rig.app.screen(), &Screen::ConfirmImage);
    assert_eq!(custom::preview().unwrap().rgb, picture(2).rgb);
    rig.press(Key::Back);
}

#[test]
fn cancel_and_back_on_confirm_go_back_to_receive_with_the_same_code() {
    let _lock = lock();
    files("cancel");
    let mut rig = Rig::new();
    receive(&mut rig);
    let code = session(&rig).code().to_string();
    let before = rig.app.config.clone();
    for leave in [Key::Back, Key::Ok] {
        send(&mut rig, &picture(3));
        if leave == Key::Ok {
            focus_on(&mut rig, CONFIRM_SCREEN_CANCEL);
        }
        rig.press(leave);
        assert_eq!(rig.app.screen(), &Screen::ReceiveImage);
        let ping = format!("POST /ping?c={code}&idle=0 HTTP/1.1\r\n\r\n");
        let answer = request_to(server_address(&rig), ping.as_bytes());
        assert!(answer.ends_with(r#""tv":"declined"}"#), "{answer}");
        assert_eq!(session(&rig).code(), code);
        assert!(session(&rig).is_listening());
        assert!(custom::preview().is_none(), "the picture left memory");
        assert_eq!(rig.app.config, before);
    }
    rig.press(Key::Back);
}

#[test]
fn apply_tells_the_page_stops_the_server_10_s_later_and_my_image_survives_a_restart() {
    let _lock = lock();
    let dir = files("apply");
    let mut rig = Rig::new();
    receive(&mut rig);
    let address = server_address(&rig);
    let port = address.port();
    let code = session(&rig).code().to_string();
    send(&mut rig, &picture(4));
    let effects = apply(&mut rig);
    assert!(effects.contains(&Effect::SaveConfig));
    assert_eq!(rig.app.screen(), &Screen::Editor);
    assert_eq!(rig.focused(), Some(PHONE_CARD));
    // The page hears it at its next ping; the server is still up for that only.
    let ping = format!("POST /ping?c={code}&idle=0 HTTP/1.1\r\n\r\n");
    let answer = request_to(address, ping.as_bytes());
    assert!(answer.ends_with(r#""tv":"applied"}"#), "{answer}");
    assert!(rig.app.companion_active());
    let applied_at = rig.now;
    let linger_end = applied_at + crate::companion::APPLIED_LINGER_MS;
    // The loop is woken for the end of it, without drawing in between.
    assert!(rig
        .app
        .next_deadline(rig.now)
        .is_some_and(|at| at <= linger_end));
    rig.app.poll_background(linger_end);
    assert!(!rig.app.companion_active());
    let closed = (0..50).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::net::TcpStream::connect_timeout(
            &(std::net::Ipv4Addr::LOCALHOST, port).into(),
            std::time::Duration::from_millis(200),
        )
        .is_err()
    });
    assert!(closed, "no port stays open after Apply");
    assert_eq!(rig.app.config.wallpaper, WallpaperId::Custom);
    assert_eq!(rig.app.config.custom, Some(Applied { rev: 1 }));
    assert_eq!(custom::load_kept(&dir).unwrap().rgb, picture(4).rgb);
    assert!(custom::preview().is_none());
    // The TV is unplugged and plugged back: the same image.
    let path = dir.join("config.v1");
    rig.app.config.save(&path).unwrap();
    let mut again = crate::home::config::HomeConfig::load(&path).unwrap();
    assert!(!again.reconcile_custom(&dir));
    assert_eq!(again.wallpaper, WallpaperId::Custom);
    assert_eq!(again.custom, Some(Applied { rev: 1 }));
}

#[test]
fn cancelling_a_replacement_keeps_the_old_image() {
    let _lock = lock();
    let dir = files("replace");
    let mut rig = Rig::new();
    apply_picture(&mut rig, 5);
    let applied = rig.app.config.custom;
    receive(&mut rig);
    send(&mut rig, &picture(6));
    rig.press(Key::Back);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Editor);
    assert_eq!(rig.app.config.custom, applied);
    assert_eq!(custom::load_kept(&dir).unwrap().rgb, picture(5).rgb);
}

// ---- the menu of "Minha imagem" -----------------------------------------------------------------

#[test]
fn the_menu_button_opens_the_menu_of_my_image_with_either_code() {
    let mut rig = Rig::new();
    rig.app.config.custom = Some(Applied { rev: 1 });
    editor(&mut rig);
    // The catalog's cards have no menu.
    focus_on(&mut rig, WALLPAPER_BASE);
    rig.app.handle_keycode(82, rig.now);
    assert!(rig.app.context_menu().is_none());
    focus_on(&mut rig, PHONE_CARD);
    for code in [82, 174] {
        rig.app.handle_keycode(code, rig.now);
        rig.advance(0);
        let menu = rig.app.context_menu().expect("the menu opened");
        let actions: Vec<MenuAction> = menu.items.iter().map(|i| i.action).collect();
        assert_eq!(
            actions,
            [
                MenuAction::UseImage,
                MenuAction::ReplaceImage,
                MenuAction::RemoveImage
            ]
        );
        assert_eq!(menu.selected, 0);
        rig.press(Key::Back);
        assert!(rig.app.context_menu().is_none());
        assert_eq!(rig.focused(), Some(PHONE_CARD));
    }
}

#[test]
fn the_empty_phone_card_has_no_menu() {
    let mut rig = Rig::new();
    editor(&mut rig);
    focus_on(&mut rig, PHONE_CARD);
    rig.press(Key::Menu);
    assert!(rig.app.context_menu().is_none());
}

/// Opens the menu of "Minha imagem" and runs its item at `index`.
fn run_image_menu(rig: &mut Rig, index: usize) -> Vec<Effect> {
    focus_on(rig, PHONE_CARD);
    rig.press(Key::Menu);
    for _ in 0..index {
        rig.press(Key::Down);
    }
    rig.press(Key::Ok)
}

#[test]
fn remove_asks_first_then_deletes_and_goes_back_to_the_aurora() {
    let _lock = lock();
    let dir = files("remove");
    let mut rig = Rig::new();
    apply_picture(&mut rig, 7);
    run_image_menu(&mut rig, 2);
    assert_eq!(rig.focused(), Some(CONFIRM_CANCEL), "it asks first");
    rig.press(Key::Ok);
    assert!(rig.app.config.custom.is_some(), "Cancel keeps it");
    assert_eq!(rig.focused(), Some(PHONE_CARD));
    run_image_menu(&mut rig, 2);
    let effects = press_on(&mut rig, CONFIRM_RESTORE);
    assert!(effects.contains(&Effect::SaveConfig));
    assert_eq!(rig.app.config.custom, None);
    assert_eq!(rig.app.config.wallpaper, WallpaperId::Aurora);
    assert!(!custom::has_kept(&dir));
    assert_eq!(rig.focused(), Some(PHONE_CARD));
}

#[test]
fn replace_and_use_from_the_menu() {
    let _lock = lock();
    files("menu");
    let mut rig = Rig::new();
    apply_picture(&mut rig, 8);
    rig.app.config.wallpaper = WallpaperId::GlassFacade;
    let effects = run_image_menu(&mut rig, 0);
    assert!(effects.contains(&Effect::SaveConfig));
    assert_eq!(rig.app.config.wallpaper, WallpaperId::Custom);
    run_image_menu(&mut rig, 1);
    assert_eq!(rig.app.screen(), &Screen::ReceiveImage);
    rig.press(Key::Back);
    assert_eq!(rig.focused(), Some(PHONE_CARD));
}

#[test]
fn back_from_the_new_screens_shows_the_phone_card_at_the_start_of_the_row() {
    let mut rig = Rig::new();
    editor(&mut rig);
    press_on(&mut rig, PHONE_CARD);
    rig.press(Key::Back);
    rig.advance(1000);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(PHONE_CARD));
    assert_eq!(rig.app.editor.wallpaper_scroll_x, 0.0);
}

/// The platform asks for news at every turn of its loop: that must not push the next frame of a
/// running animation away (it stopped the splash and the focus on the TVs).
#[test]
fn polling_the_background_keeps_the_animation_frames_coming() {
    let mut rig = Rig::new();
    editor(&mut rig);
    focus_on(&mut rig, PHONE_CARD);
    rig.press(Key::Left);
    let now = rig.now;
    assert!(rig.app.is_animating(now), "the focus moves");
    let due = rig.app.next_deadline(now).unwrap();
    for turn in 1..50 {
        rig.app.poll_background(now + turn);
        assert_eq!(rig.app.next_deadline(now + turn), Some(due));
    }
}

#[test]
fn the_phone_card_comes_first_and_the_editor_opens_on_the_aurora() {
    let mut rig = Rig::new();
    editor(&mut rig);
    // First use: the Aurora is the wallpaper and has the focus.
    assert_eq!(rig.app.config.wallpaper, WallpaperId::Aurora);
    assert_eq!(rig.focused(), Some(WALLPAPER_BASE));
    let row: Vec<FocusId> = rig.app.focus.rows()[0].items.iter().map(|i| i.id).collect();
    assert_eq!(row[0], PHONE_CARD);
    assert_eq!(row[1], WALLPAPER_BASE);
    assert_eq!(row.len(), WallpaperId::ALL.len() + 1);
    // Left from the Aurora is the phone card.
    rig.press(Key::Left);
    assert_eq!(rig.focused(), Some(PHONE_CARD));
}

#[test]
fn ok_on_my_image_when_it_is_already_the_wallpaper_opens_its_menu() {
    let mut rig = Rig::new();
    rig.app.config.custom = Some(Applied { rev: 1 });
    rig.app.config.wallpaper = WallpaperId::Custom;
    editor(&mut rig);
    assert_eq!(rig.focused(), Some(PHONE_CARD));
    rig.press(Key::Ok);
    let menu = rig.app.context_menu().expect("OK opened the menu");
    assert!(menu
        .items
        .iter()
        .any(|i| i.action == MenuAction::ReplaceImage));
    // "Replace image" leads to the Receive image screen.
    rig.press(Key::Down);
    rig.press(Key::Ok);
    assert_eq!(rig.app.screen(), &Screen::ReceiveImage);
}
