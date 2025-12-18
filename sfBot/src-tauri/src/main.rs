#![allow(warnings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs::OpenOptions, io::Write, net::SocketAddr, panic, sync::Arc};

use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode, Uri},
    routing::{get, post},
    Router,
};
use chrono::Local;
use rust_embed::Embed;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use sfbot_lib::api::{self, AppState};
use sfbot_lib::autostart_bot_if_enabled;
use sfbot_lib::bot_runner::BotRunner;

mod updater;

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

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    println!("Starting SF Bot Server...");

    // Initialize logging
    if let Err(e) = tracing_subscriber::fmt::try_init() {
        eprintln!("Tracing already initialized: {}", e);
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
        .route("/api/characters/all-settings", get(api::get_all_character_settings))
        .route("/api/characters/log", get(api::get_character_log))
        .route("/api/characters/cached", get(api::get_cached_characters))

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
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("");
    println!("========================================");
    println!("  SF Bot Server running!");
    println!("  URL: http://{}", addr);
    println!("  Frontend: embedded in binary");
    println!("========================================");
    println!("");
    println!("Open http://localhost:3000 in your browser");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("");
            eprintln!("ERROR: Could not bind to port 3000!");
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
