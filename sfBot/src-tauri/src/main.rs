#![allow(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    net::SocketAddr,
    panic,
    process::Command,
    sync::Arc,
    thread,
    time::Duration,
};

use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode, Uri},
    routing::{get, post},
    Router,
};
use chrono::Local;
use rust_embed::Embed;
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use sfbot_lib::api::{self, AppState};
use sfbot_lib::autostart_bot_if_enabled;
use sfbot_lib::bot_runner::BotRunner;
use sfbot_lib::paths;

#[cfg(windows)]
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIconBuilder, TrayIconEvent,
};
#[cfg(windows)]
use winapi::um::winuser::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};
#[cfg(windows)]
use open;
mod updater;

#[cfg(target_os = "linux")]
fn suppress_console_output() {
    use std::os::unix::io::AsRawFd;
    let devnull = match std::fs::OpenOptions::new().write(true).open("/dev/null") {
        Ok(f) => f,
        Err(_) => return,
    };
    let fd = devnull.as_raw_fd();
    unsafe {
        libc::dup2(fd, libc::STDOUT_FILENO);
        libc::dup2(fd, libc::STDERR_FILENO);
    }
    std::mem::forget(devnull);
}

// Embed frontend files into the binary at compile time
#[derive(Embed)]
#[folder = "../src/"]
struct FrontendAssets;

// Handler for serving embedded static files
async fn serve_frontend(uri: Uri) -> Response<Body> {
    let path = uri.path().trim_start_matches('/');

    // Default to index.html for root path
    let path = if path.is_empty() { "index.html" } else { path };

    match FrontendAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => {
            match FrontendAssets::get("index.html") {
                Some(content) => {
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/html")
                        .body(Body::from(content.data.into_owned()))
                        .unwrap()
                }
                None => {
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("404 Not Found"))
                        .unwrap()
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    port: Option<u16>,
}

fn load_server_port() -> u16 {
    let default_port = 3000;
    let config_path = paths::get_server_config_path();
    let contents = match fs::read_to_string(&config_path) {
        Ok(data) => data,
        Err(_) => return default_port,
    };

    let config: ServerConfig = match serde_json::from_str(&contents) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("[SERVER] Failed to parse {}: {}", config_path.display(), e);
            return default_port;
        }
    };

    match config.port {
        Some(port) if port > 0 => port,
        _ => default_port,
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 6)]
async fn main() {
    #[cfg(target_os = "linux")]
    if !cfg!(debug_assertions) {
        suppress_console_output();
    }
    println!("Starting SF Bot Server...");

    // Initialize logging
    if let Err(e) = tracing_subscriber::fmt::try_init() {
        eprintln!("Tracing already initialized: {}", e);
    }

    #[cfg(windows)]
    {
        updater::cleanup_old_backups();
        match updater::maybe_run_update(env!("CARGO_PKG_VERSION")).await {
            Ok(true) => {
                println!("[UPDATER] Update triggered, exiting.");
                return;
            }
            Ok(false) => {}
            Err(e) => eprintln!("[UPDATER] Update check failed: {}", e),
        }
    }

    println!("Initializing character settings cache...");

    // Initialize character settings cache
    sfbot_lib::cache_character_settings();

    println!("Creating application state...");

    // Create shared application state
    let bot_runner = Arc::new(RwLock::new(BotRunner::new()));
    let app_state = AppState {
        bot_runner: bot_runner.clone(),
    };

    let port = load_server_port();

    // Start system tray (Windows only)
    #[cfg(windows)]
    start_tray_icon(port);

    // Auto-start bot based on global settings flag
    {
        let bot_runner = bot_runner.clone();
        tokio::spawn(async move {
            autostart_bot_if_enabled(bot_runner).await;
        });
    }

    println!("Configuring HTTP routes...");

    // Configure CORS for browser access
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build the router with all API routes
    let app = Router::new()
        // Bot control
        .route("/api/bot/start", post(api::start_bot))
        .route("/api/bot/stop", post(api::stop_bot))
        .route("/api/bot/status", get(api::get_bot_status))
        .route("/api/bot/pause", post(api::pause_bot))
        .route("/api/bot/resume", post(api::resume_bot))
        .route("/api/shutdown", post(api::shutdown_server))

        // Account management
        .route("/api/accounts", get(api::get_accounts))
        .route("/api/accounts/login", post(api::login_account))
        .route("/api/accounts/login-single", post(api::login_single_account))

        // Character management
        .route("/api/characters", get(api::get_characters))
        .route("/api/characters/settings", get(api::get_character_settings))
        .route("/api/characters/settings", post(api::save_character_settings))
        .route("/api/characters/settings-all", post(api::save_all_character_settings))
        .route("/api/characters/all-settings", get(api::get_all_character_settings))
        .route("/api/characters/log", get(api::get_character_log))
        .route("/api/characters/expedition-stats", get(api::get_character_expedition_stats))
        .route("/api/characters/cached", get(api::get_cached_characters))

        // Expedition summary
        .route("/api/expeditions/summary", get(api::get_expedition_summary))

        // Coupons
        .route("/api/coupons/redeem", post(api::redeem_coupon))
        .route("/api/coupons/status", get(api::coupon_status))

        // Global settings
        .route("/api/settings", get(api::get_global_settings))
        .route("/api/settings", post(api::save_global_settings))

        // User config
        .route("/api/config", get(api::get_user_config))
        .route("/api/config", post(api::save_user_config))

        // Misc
        .route("/api/version", get(api::get_version))
        .route("/api/auth/check", get(api::check_auth))
        .route("/api/auth/hash", get(api::get_hash))

        // Serve embedded frontend files (fallback for non-API routes)
        .fallback(serve_frontend)

        .layer(cors)
        .with_state(app_state);

    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("");
    println!("========================================");
    println!("  SF Bot Server running!");
    println!("  URL: http://{}", addr);
    println!("  Frontend: embedded in binary");
    println!("========================================");
    println!("");
    println!("Open http://localhost:{} in your browser", port);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("");
            eprintln!("ERROR: Could not bind to port {}!", port);
            eprintln!("Reason: {}", e);
            eprintln!("");
            eprintln!("Another instance of sfbot might already be running.");
            eprintln!("Kill it with: taskkill /f /im sfbot.exe");
            eprintln!("");
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn start_tray_icon(port: u16) {
    thread::spawn(move || {
        let icon = load_tray_icon();

        let mut menu = Menu::new();
        let open_item = MenuItem::new("Open UI", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        menu.append(&open_item).ok();
        menu.append(&quit_item).ok();

        let _tray = TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_tooltip("SF Bot")
            .build()
            .expect("Failed to build tray icon");

        println!("[TRAY] Tray icon initialized");

        // Listen for menu and tray icon events; pump Windows message loop on this thread
        let menu_rx = MenuEvent::receiver();
        let tray_rx = TrayIconEvent::receiver();
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            loop {
                let res = GetMessageW(&mut msg, 0 as _, 0, 0);
                if res <= 0 {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                // Handle tray icon click events (currently none, only menu items)
                // We intentionally ignore direct clicks to avoid multiple opens.

                // Handle menu events
                if let Ok(event) = menu_rx.try_recv() {
                    if event.id() == open_item.id() {
                        println!("[TRAY] Menu: Open UI");
                        open_ui(port);
                    } else if event.id() == quit_item.id() {
                        println!("[TRAY] Menu: Quit");
                        std::process::exit(0);
                    }
                }
            }
        }
    });
}

#[cfg(windows)]
fn load_tray_icon() -> Icon {
    let bytes = include_bytes!("../icons/tray.png");
    let image = image::load_from_memory(bytes).expect("Failed to load tray icon bytes");
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).expect("Failed to build tray icon")
}

#[cfg(windows)]
fn open_ui(port: u16) {
    // Try default browser; fallback to cmd start if needed
    let url = format!("http://localhost:{}", port);
    if open::that(&url).is_err() {
        let _ = Command::new("cmd")
            .args(&["/C", "start", &url])
            .spawn();
    }
}
