#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod batch;
mod browser_media;
mod browser_session;
mod browser_setup;
mod categories;
mod columns;
mod engine;
mod external;
mod grabber;
mod model;
mod schedule;
mod streaming;
mod ui;
mod ui_assets;
use std::{io::Write, net::TcpListener, path::PathBuf};
fn app_data() -> PathBuf {
    // Explicit per-process override keeps browser/UI acceptance runs isolated
    // from the user's library, including native-host child processes.
    if let Some(path) = std::env::var_os("IDX_DATA_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        return path;
    }
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("IDX")
}
fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--enqueue-json") {
        use std::io::Read;
        let result = (|| -> Result<(), String> {
            let mut input = Vec::new();
            std::io::stdin()
                .take(262145)
                .read_to_end(&mut input)
                .map_err(|_| "Could not read browser handoff.")?;
            if input.len() > 262144 {
                return Err("Browser handoff is too large.".into());
            }
            let request: external::Request =
                serde_json::from_slice(&input).map_err(|_| "Invalid browser handoff.")?;
            external::enqueue_request(&app_data().join("inbox"), request)
        })();
        if result.is_err() {
            std::process::exit(1);
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--connect-browser") {
        let result = std::env::current_exe()
            .map_err(|e| e.to_string())
            .and_then(|exe| browser_setup::connect_bundled(&app_data(), &exe));
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--enqueue") {
        let result = args
            .get(i + 1)
            .ok_or_else(|| "Missing URL".to_string())
            .and_then(|url| {
                external::enqueue_with_metadata(
                    &app_data().join("inbox"),
                    url,
                    args.iter().any(|a| a == "--automatic"),
                    args.iter()
                        .position(|a| a == "--parent-page")
                        .and_then(|i| args.get(i + 1))
                        .map(String::as_str)
                        .unwrap_or(""),
                    args.iter()
                        .position(|a| a == "--filename")
                        .and_then(|i| args.get(i + 1))
                        .map(String::as_str)
                        .unwrap_or(""),
                    args.iter()
                        .position(|a| a == "--referer")
                        .and_then(|i| args.get(i + 1))
                        .map(String::as_str)
                        .unwrap_or(""),
                )
            });
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--add") {
        if let Some(url) = args.get(i + 1) {
            if let Err(e) = external::enqueue(&app_data().join("inbox"), url, false) {
                eprintln!("{e}");
                return Ok(());
            }
        }
    }
    if args.iter().any(|a| a == "--version") {
        println!("IDX – Internet Downloader X {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let instance_port = std::env::var("IDX_INSTANCE_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(47839);
    let listener = match TcpListener::bind(("127.0.0.1", instance_port)) {
        Ok(l) => l,
        Err(_) => {
            if let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", instance_port)) {
                let _ = s.write_all(b"focus");
            }
            return Ok(());
        }
    };
    listener.set_nonblocking(true).ok();
    let mut library_path = app_data().join("library.json");
    if let Some(i) = args.iter().position(|a| a == "--data-dir") {
        if let Some(p) = args.get(i + 1) {
            library_path = PathBuf::from(p).join("library.json");
        }
    }
    #[cfg(windows)]
    {
        if let Ok(executable) = std::env::current_exe() {
            if let Err(error) = browser_setup::connect_bundled(
                library_path.parent().unwrap_or(std::path::Path::new(".")),
                &executable,
            ) {
                eprintln!("Browser connection: {error}");
            }
        }
        if let Err(error) = ui::native_main::run(library_path, listener) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    #[cfg(not(windows))]
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("IDX – Internet Downloader X")
            .with_inner_size([1160.0, 760.0])
            .with_min_inner_size([900.0, 580.0])
            .with_icon(ui::app_icon()),
        ..Default::default()
    };
    #[cfg(not(windows))]
    return eframe::run_native(
        "IDX – Internet Downloader X",
        options,
        Box::new(move |cc| Ok(Box::new(ui::IdxApp::new(cc, library_path, listener)))),
    );
}
