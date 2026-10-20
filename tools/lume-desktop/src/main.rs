//! Desktop harness entry point.

use lume_desktop::demo::{self, Setup};
use lume_desktop::{cli, png_out, screenshot};

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(2);
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
        "home" | "apps" | "settings" | "about" | "editor" => {
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
                "about" => "up,right,down,down,right,ok,down,ok",
                "editor" => "up,right,down,down,right,ok,ok",
                _ => "",
            };
            let mut keys = demo::parse_keys(route).unwrap_or_else(|e| fail(&e));
            if let Some(list) = &options.keys {
                keys.extend(demo::parse_keys(list).unwrap_or_else(|e| fail(&e)));
            }
            screenshot::render_app(width, height, &mut app, &keys)
                .and_then(|pixels| png_out::write_rgba(path, width, height, &pixels))
        }
        other => fail(&format!(
            "unknown screen '{other}' (available: empty, logo, banner, home, apps, settings, about, editor)"
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
