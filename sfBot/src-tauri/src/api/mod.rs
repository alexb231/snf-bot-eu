//! HTTP API module for the SF Bot
//!
//! Provides RESTful endpoints for:
//! - Bot control (start/stop/pause/resume)
//! - Account and character management
//! - Settings management

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::bot_runner::{AccountInfo, BotRunner};
use crate::expedition_utils::{read_expedition_stats, read_expedition_summary};
use crate::utils::{CharacterDisplay, PlayerConfig, UserConfig};
use crate::{generate_hash, perform_check_whether_user_is_allowed_to_start_bot};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub bot_runner: Arc<RwLock<BotRunner>>,
}

// ============================================================================
// Bot Control Endpoints
// ============================================================================

#[derive(Deserialize)]
pub struct StartBotRequest {
    pub accounts: Vec<StartAccountInfo>,
}

/// Account info received from frontend (simplified - no characters needed)
#[derive(Deserialize, Clone)]
pub struct StartAccountInfo {
    pub accname: String,
    pub password: String,
    pub single: bool,
    pub server: String,
}

#[derive(Serialize)]
pub struct BotStatusResponse {
    pub running: bool,
    pub paused: bool,
    pub accounts: Vec<String>,
    pub current_character: Option<CurrentCharacterInfo>,
    pub characters: Vec<CharacterStatusInfo>,
}

#[derive(Serialize, Clone)]
pub struct CharacterStatusInfo {
    pub id: u32,
    pub name: String,
    pub server: String,
    pub account: String,
    pub lvl: u32,
    pub guild: String,
    pub gold: u64,
    pub mushrooms: i32,
    pub beers: u32,
    pub fights: u32,
    pub alu: u32,
    pub petfights: u8,
    pub dicerolls: u8,
    pub current_action: String,
    #[serde(rename = "isActive")]
    pub is_active: bool,
}

#[derive(Serialize, Clone)]
pub struct CurrentCharacterInfo {
    pub account: String,
    pub name: String,
    pub id: u32,
    pub current_action: String,
}

/// POST /api/bot/start - Start the bot with given accounts
pub async fn start_bot(
    State(state): State<AppState>,
    Json(request): Json<StartBotRequest>,
) -> impl IntoResponse {
    let mut runner = state.bot_runner.write().await;

    // Convert StartAccountInfo to AccountInfo
    let accounts: Vec<AccountInfo> = request.accounts.into_iter().map(|a| AccountInfo {
        accname: a.accname,
        password: a.password,
        single: a.single,
        server: a.server,
    }).collect();

    match runner.start(accounts).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status": "started"}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

/// POST /api/bot/stop - Stop the bot
pub async fn stop_bot(State(state): State<AppState>) -> impl IntoResponse {
    let mut runner = state.bot_runner.write().await;
    runner.stop().await;
    (StatusCode::OK, Json(serde_json::json!({"status": "stopped"})))
}

/// POST /api/shutdown - Shutdown the entire server process
pub async fn shutdown_server() -> impl IntoResponse {
    // Send response before shutting down
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    (StatusCode::OK, Json(serde_json::json!({"status": "shutting_down"})))
}

/// GET /api/bot/status - Get current bot status
pub async fn get_bot_status(State(state): State<AppState>) -> impl IntoResponse {
    let runner = state.bot_runner.read().await;
    let status = runner.get_status();
    (StatusCode::OK, Json(status))
}

/// POST /api/bot/pause - Pause the bot
pub async fn pause_bot(State(state): State<AppState>) -> impl IntoResponse {
    let mut runner = state.bot_runner.write().await;
    runner.pause();
    (StatusCode::OK, Json(serde_json::json!({"status": "paused"})))
}

/// POST /api/bot/resume - Resume the bot
pub async fn resume_bot(State(state): State<AppState>) -> impl IntoResponse {
    let mut runner = state.bot_runner.write().await;
    runner.resume();
    (StatusCode::OK, Json(serde_json::json!({"status": "resumed"})))
}

// ============================================================================
// Account Management Endpoints
// ============================================================================

/// GET /api/accounts - Get saved accounts from userConfig.json
pub async fn get_accounts() -> impl IntoResponse {
    match crate::utils::read_user_conf() {
        Ok(accounts) => (StatusCode::OK, Json(serde_json::json!({"accounts": accounts}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// POST /api/accounts/login - Login to SF account (SSO, multiple characters)
pub async fn login_account(Json(request): Json<LoginRequest>) -> impl IntoResponse {
    match crate::utils::login(&request.username, &request.password).await {
        Ok(characters) => (StatusCode::OK, Json(serde_json::json!({"characters": characters}))),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

#[derive(Deserialize)]
pub struct LoginSingleRequest {
    pub username: String,
    pub password: String,
    pub server: String,
}

/// POST /api/accounts/login-single - Login to single server account
pub async fn login_single_account(Json(request): Json<LoginSingleRequest>) -> impl IntoResponse {
    match crate::utils::login_single_account(
        &request.username,
        &request.password,
        true,
        &request.server,
    )
    .await
    {
        Ok(characters) => (StatusCode::OK, Json(serde_json::json!({"characters": characters}))),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ============================================================================
// Character Management Endpoints
// ============================================================================

/// GET /api/characters - Get all logged-in characters
pub async fn get_characters(State(state): State<AppState>) -> impl IntoResponse {
    let runner = state.bot_runner.read().await;
    let characters = runner.get_characters();
    (StatusCode::OK, Json(serde_json::json!({"characters": characters})))
}

#[derive(Deserialize)]
pub struct CharacterSettingsQuery {
    pub name: String,
    pub id: u32,
}

/// GET /api/characters/settings?name=X&id=Y - Get settings for a character
pub async fn get_character_settings(Query(query): Query<CharacterSettingsQuery>) -> impl IntoResponse {
    match crate::utils::load_character_settings(&query.name, query.id) {
        Ok(Some(settings)) => (StatusCode::OK, Json(serde_json::json!({"settings": settings}))),
        Ok(None) => (StatusCode::OK, Json(serde_json::json!({"settings": {}}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

#[derive(Deserialize)]
pub struct SaveCharacterSettingsRequest {
    pub name: String,
    pub id: u32,
    pub settings: HashMap<String, Value>,
}

/// POST /api/characters/settings - Save settings for a character
pub async fn save_character_settings(
    Json(request): Json<SaveCharacterSettingsRequest>,
) -> impl IntoResponse {
    match crate::utils::save_character_settings(&request.name, request.id, request.settings).await {
        Ok(json_str) => {
            // Parse the JSON string from utils and return it directly
            match serde_json::from_str::<Value>(&json_str) {
                Ok(json_value) => (StatusCode::OK, Json(json_value)),
                Err(_) => (StatusCode::OK, Json(serde_json::json!({"message": json_str}))),
            }
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

/// GET /api/characters/all-settings - Get all character settings
pub async fn get_all_character_settings() -> impl IntoResponse {
    match crate::utils::load_all_character_settings() {
        Ok(settings) => (StatusCode::OK, Json(serde_json::json!({"settings": settings}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ============================================================================
// Global Settings Endpoints
// ============================================================================

/// GET /api/settings - Get global settings
pub async fn get_global_settings() -> impl IntoResponse {
    match crate::utils::get_global_settings().await {
        Ok(settings) => (StatusCode::OK, Json(serde_json::json!({"settings": settings}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

#[derive(Deserialize)]
pub struct SaveGlobalSettingsRequest {
    pub settings: HashMap<String, Value>,
}

/// POST /api/settings - Save global settings
pub async fn save_global_settings(
    Json(request): Json<SaveGlobalSettingsRequest>,
) -> impl IntoResponse {
    match crate::utils::save_global_settings(request.settings).await {
        Ok(msg) => (StatusCode::OK, Json(serde_json::json!({"message": msg}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ============================================================================
// User Config Endpoints
// ============================================================================

/// GET /api/config - Get user config (accounts)
pub async fn get_user_config() -> impl IntoResponse {
    match crate::utils::read_user_conf() {
        Ok(accounts) => (StatusCode::OK, Json(serde_json::json!({"accounts": accounts}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

#[derive(Deserialize)]
pub struct SaveUserConfigRequest {
    pub accname: String,
    pub password: String,
    pub single: bool,
    pub server: String,
}

/// POST /api/config - Save user config
pub async fn save_user_config(Json(request): Json<SaveUserConfigRequest>) -> impl IntoResponse {
    match crate::utils::save_user_conf(
        request.accname,
        request.password,
        request.single,
        request.server,
    ) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status": "saved"}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ============================================================================
// Misc Endpoints
// ============================================================================

/// GET /api/version - Get app version
pub async fn get_version() -> impl IntoResponse {
    let version = env!("CARGO_PKG_VERSION");
    (StatusCode::OK, Json(serde_json::json!({"version": version})))
}

/// GET /api/auth/check - Check if user is authorized
pub async fn check_auth() -> impl IntoResponse {
    match perform_check_whether_user_is_allowed_to_start_bot().await {
        Ok(allowed) => (StatusCode::OK, Json(serde_json::json!({"allowed": allowed}))),
        Err(_) => (StatusCode::OK, Json(serde_json::json!({"allowed": false}))),
    }
}

/// GET /api/auth/hash - Get user's hardware hash
pub async fn get_hash() -> impl IntoResponse {
    let hash = generate_hash();
    (StatusCode::OK, Json(serde_json::json!({"hash": hash})))
}

// ============================================================================
// Character Log Endpoints
// ============================================================================

#[derive(Deserialize)]
pub struct CharacterLogQuery {
    pub name: String,
    pub id: u32,
}

/// GET /api/characters/log?name=X&id=Y - Get log for a character
pub async fn get_character_log(Query(query): Query<CharacterLogQuery>) -> impl IntoResponse {
    match crate::bot_runner::read_character_log(&query.name, query.id) {
        Ok(content) => (StatusCode::OK, Json(serde_json::json!({"log": content}))),
        Err(e) => (StatusCode::OK, Json(serde_json::json!({"log": "", "error": e}))),
    }
}

#[derive(Deserialize)]
pub struct SaveAllCharacterSettingsRequest {
    pub settings: HashMap<String, Value>,
}

/// POST /api/characters/settings-all - Save settings for all characters
pub async fn save_all_character_settings(
    Json(request): Json<SaveAllCharacterSettingsRequest>,
) -> impl IntoResponse {
    match crate::utils::save_settings_for_all_characters(request.settings).await {
        Ok(json_str) => {
            match serde_json::from_str::<Value>(&json_str) {
                Ok(json_value) => (StatusCode::OK, Json(json_value)),
                Err(_) => (StatusCode::OK, Json(serde_json::json!({"message": json_str}))),
            }
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ============================================================================
// Expedition Stats Endpoints
// ============================================================================

#[derive(Deserialize)]
pub struct ExpeditionStatsQuery {
    pub name: String,
    pub id: u32,
    pub server: String,
}

/// GET /api/characters/expedition-stats?name=X&id=Y&server=Z - Get expedition stats for a character
pub async fn get_character_expedition_stats(Query(query): Query<ExpeditionStatsQuery>) -> impl IntoResponse {
    match read_expedition_stats(&query.name, query.id, &query.server) {
        Ok(Some(stats)) => (StatusCode::OK, Json(serde_json::json!({"stats": stats}))),
        Ok(None) => (StatusCode::OK, Json(serde_json::json!({"stats": null}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

/// GET /api/expeditions/summary - Get aggregated expedition stats across all characters
pub async fn get_expedition_summary() -> impl IntoResponse {
    match read_expedition_summary() {
        Ok(stats) => (StatusCode::OK, Json(stats)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ============================================================================
// Character Cache Endpoints
// ============================================================================

/// GET /api/characters/cached - Get all cached characters (for display before bot starts)
pub async fn get_cached_characters() -> impl IntoResponse {
    match crate::character_cache::load_all_cached_characters() {
        Ok(mut characters) => {
            // Update is_active from current settings (settings are the source of truth)
            for character in &mut characters {
                if let Ok(Some(settings)) = crate::utils::load_character_settings(&character.name, character.id) {
                    if let Some(is_active) = settings.get("settingCharacterActive") {
                        character.is_active = is_active.as_bool().unwrap_or(false);
                    } else {
                        // Setting key doesn't exist, default to inactive
                        character.is_active = false;
                    }
                } else {
                    // No settings for this character, default to inactive
                    character.is_active = false;
                }
            }
            (StatusCode::OK, Json(serde_json::json!({"characters": characters})))
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}
