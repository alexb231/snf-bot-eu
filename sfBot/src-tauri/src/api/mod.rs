






use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

use crate::bot_runner::{AccountInfo, BotRunner};
use crate::expedition_utils::{read_expedition_stats, read_expedition_summary};
use crate::utils::{CharacterDisplay, PlayerConfig, UserConfig};
use crate::{generate_hash, perform_check_whether_user_is_allowed_to_start_bot};
use crate::webshop::{
    redeem_coupon_for_all_with_progress, CouponRedeemProgress,
    CouponRedeemSummary,
};


#[derive(Clone)]
pub struct AppState {
    pub bot_runner: Arc<RwLock<BotRunner>>,
}

#[derive(Clone, Default)]
struct CouponJobState {
    running: bool,
    started_at: Option<String>,
    finished_at: Option<String>,
    progress: CouponRedeemProgress,
    summary: Option<CouponRedeemSummary>,
    error: Option<String>,
}

static COUPON_JOB_STATE: Lazy<Mutex<CouponJobState>> =
    Lazy::new(|| Mutex::new(CouponJobState::default()));

#[derive(Serialize)]
struct CouponJobStatusResponse {
    running: bool,
    started_at: Option<String>,
    finished_at: Option<String>,
    progress: CouponRedeemProgress,
    summary: Option<CouponRedeemSummary>,
    error: Option<String>,
}





#[derive(Deserialize)]
pub struct StartBotRequest {
    pub accounts: Vec<StartAccountInfo>,
}


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


pub async fn start_bot(
    State(state): State<AppState>,
    Json(request): Json<StartBotRequest>,
) -> impl IntoResponse {
    let mut runner = state.bot_runner.write().await;

    
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


pub async fn stop_bot(State(state): State<AppState>) -> impl IntoResponse {
    let mut runner = state.bot_runner.write().await;
    runner.stop().await;
    (StatusCode::OK, Json(serde_json::json!({"status": "stopped"})))
}


pub async fn shutdown_server() -> impl IntoResponse {
    
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    (StatusCode::OK, Json(serde_json::json!({"status": "shutting_down"})))
}


pub async fn get_bot_status(State(state): State<AppState>) -> impl IntoResponse {
    let runner = state.bot_runner.read().await;
    let status = runner.get_status();
    (StatusCode::OK, Json(status))
}


pub async fn pause_bot(State(state): State<AppState>) -> impl IntoResponse {
    let mut runner = state.bot_runner.write().await;
    runner.pause();
    (StatusCode::OK, Json(serde_json::json!({"status": "paused"})))
}


pub async fn resume_bot(State(state): State<AppState>) -> impl IntoResponse {
    let mut runner = state.bot_runner.write().await;
    runner.resume();
    (StatusCode::OK, Json(serde_json::json!({"status": "resumed"})))
}






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


pub async fn save_character_settings(
    Json(request): Json<SaveCharacterSettingsRequest>,
) -> impl IntoResponse {
    match crate::utils::save_character_settings(&request.name, request.id, request.settings).await {
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


pub async fn get_all_character_settings() -> impl IntoResponse {
    match crate::utils::load_all_character_settings() {
        Ok(settings) => (StatusCode::OK, Json(serde_json::json!({"settings": settings}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}






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






pub async fn get_version() -> impl IntoResponse {
    let version = env!("CARGO_PKG_VERSION");
    (StatusCode::OK, Json(serde_json::json!({"version": version})))
}


pub async fn check_auth() -> impl IntoResponse {
    match perform_check_whether_user_is_allowed_to_start_bot().await {
        Ok(allowed) => (StatusCode::OK, Json(serde_json::json!({"allowed": allowed}))),
        Err(_) => (StatusCode::OK, Json(serde_json::json!({"allowed": false}))),
    }
}


pub async fn get_hash() -> impl IntoResponse {
    let hash = generate_hash();
    (StatusCode::OK, Json(serde_json::json!({"hash": hash})))
}





#[derive(Deserialize)]
pub struct CharacterLogQuery {
    pub name: String,
    pub id: u32,
}


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





#[derive(Deserialize)]
pub struct RedeemCouponRequest {
    pub code: String,
}


pub async fn redeem_coupon(
    Json(request): Json<RedeemCouponRequest>,
) -> impl IntoResponse {
    let code = request.code.trim().to_string();
    if code.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Coupon code is empty"})),
        );
    }

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    {
        let mut state = COUPON_JOB_STATE.lock().await;
        if state.running {
            return (
                StatusCode::OK,
                Json(serde_json::json!({"status": "running"})),
            );
        }
        state.running = true;
        state.started_at = Some(now.clone());
        state.finished_at = None;
        state.progress = CouponRedeemProgress::default();
        state.summary = None;
        state.error = None;
    }

    let code_clone = code.clone();
    tokio::spawn(async move {
        let result = redeem_coupon_for_all_with_progress(
            &code_clone,
            |progress| {
                if let Ok(mut state) = COUPON_JOB_STATE.try_lock() {
                    state.progress = progress;
                }
            },
        )
        .await;
        let mut state = COUPON_JOB_STATE.lock().await;
        state.running = false;
        state.finished_at =
            Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        match result {
            Ok(summary) => {
                state.progress = CouponRedeemProgress {
                    processed: summary.results.len(),
                    total: summary.results.len(),
                    current: None,
                };
                state.summary = Some(summary);
                state.error = None;
            }
            Err(err) => {
                state.progress.current = None;
                state.summary = None;
                state.error = Some(err.to_string());
            }
        }
    });

    (StatusCode::OK, Json(serde_json::json!({"status": "started"})))
}


pub async fn coupon_status() -> impl IntoResponse {
    let state = COUPON_JOB_STATE.lock().await;
    let response = CouponJobStatusResponse {
        running: state.running,
        started_at: state.started_at.clone(),
        finished_at: state.finished_at.clone(),
        progress: state.progress.clone(),
        summary: state.summary.clone(),
        error: state.error.clone(),
    };
    (StatusCode::OK, Json(response))
}





#[derive(Deserialize)]
pub struct ExpeditionStatsQuery {
    pub name: String,
    pub id: u32,
    pub server: String,
}


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


pub async fn get_expedition_summary() -> impl IntoResponse {
    match read_expedition_summary() {
        Ok(stats) => (StatusCode::OK, Json(stats)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}






pub async fn get_cached_characters() -> impl IntoResponse {
    match crate::character_cache::load_all_cached_characters() {
        Ok(mut characters) => {
            
            for character in &mut characters {
                if let Ok(Some(settings)) = crate::utils::load_character_settings(&character.name, character.id) {
                    if let Some(is_active) = settings.get("settingCharacterActive") {
                        character.is_active = is_active.as_bool().unwrap_or(false);
                    } else {
                        
                        character.is_active = false;
                    }
                } else {
                    
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
