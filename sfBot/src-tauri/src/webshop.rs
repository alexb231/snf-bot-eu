use std::{collections::HashMap, error::Error};

use once_cell::sync::Lazy;
use reqwest::Client;
use reqwest::header::{HeaderMap, SET_COOKIE};
use serde::{Deserialize, Serialize};
use sf_api::{command::Command, gamestate::GameState, SimpleSession};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::fetch_character_setting;
use crate::get_all_session_states;

type WebshopResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const WEB_SHOP_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
const SERVER_CONFIG_URL: &str = "https://sfgame.net/config.json";
const COUPON_REDEEM_URL: &str = "https://coupon.playa-games.com/redeem";

static WEB_SHOP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .gzip(true)
        .deflate(true)
        .build()
        .expect("failed to build webshop client")
});

static SERVER_ID_CACHE: Lazy<Mutex<Option<HashMap<String, i32>>>> =
    Lazy::new(|| Mutex::new(None));

#[derive(Debug, Deserialize)]
struct WebShopResponse {
    #[serde(rename = "success")]
    success: bool,
    #[serde(rename = "catalog")]
    catalog: Option<WebShopCatalog>,
}

#[derive(Debug, Deserialize)]
struct WebShopCatalog {
    #[serde(rename = "articles")]
    articles: Vec<WebShopArticle>,
}

#[derive(Debug, Deserialize)]
struct WebShopArticle {
    #[serde(rename = "identifier")]
    identifier: String,
    #[serde(rename = "sku")]
    sku: String,
    #[serde(rename = "validAgainInSeconds")]
    valid_again_in_seconds: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CouponRedeemResult {
    pub name: String,
    pub id: u32,
    pub server: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CouponRedeemSummary {
    pub applied: usize,
    pub failed: usize,
    pub results: Vec<CouponRedeemResult>,
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    #[serde(rename = "servers")]
    servers: Vec<ServerInfo>,
}

#[derive(Debug, Deserialize)]
struct ServerInfo {
    #[serde(rename = "i")]
    id: i32,
    #[serde(rename = "d")]
    domain: String,
    #[serde(rename = "md")]
    merged_into: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CouponApiResponse {
    status: Option<String>,
    message: Option<String>,
    success: Option<bool>,
}

struct CouponHttpResponse {
    status: reqwest::StatusCode,
    body: String,
}

struct CouponOutcome {
    success: bool,
    message: String,
    retryable: bool,
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => encoded.push(b as char),
            _ => {
                encoded.push('%');
                encoded.push(nibble_to_hex((b >> 4) & 0x0f));
                encoded.push(nibble_to_hex(b & 0x0f));
            }
        }
    }
    encoded
}

fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'A' + (nibble - 10)) as char,
        _ => '0',
    }
}

fn get_webshop_character_ids(gs: &GameState) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(id) = gs
        .character
        .webshop_id
        .as_ref()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
    {
        ids.push(id.to_string());
    }
    if let Some(id) = gs
        .character
        .sf_home_id
        .as_ref()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
    {
        if !ids.iter().any(|existing| existing == id) {
            ids.push(id.to_string());
        }
    }
    ids
}

async fn send_webshop_request(
    endpoint: &str,
    cookie_header: Option<&str>,
) -> WebshopResult<String> {
    let (body, _cookies) =
        send_webshop_request_with_cookies(endpoint, cookie_header).await?;
    Ok(body)
}

async fn send_webshop_request_with_cookies(
    endpoint: &str,
    cookie_header: Option<&str>,
) -> WebshopResult<(String, Vec<String>)> {
    let url = format!("https://home.sfgame.net/api/shop/{endpoint}");
    let mut request = WEB_SHOP_CLIENT
        .get(&url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "en-GB,en-US;q=0.9,en;q=0.8")
        .header("Referer", "https://home.sfgame.net")
        .header("User-Agent", WEB_SHOP_USER_AGENT);

    if let Some(cookie_header) = cookie_header {
        request = request.header("Cookie", cookie_header);
    }

    let response = request.send().await?;
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        let body = truncate_for_log(&body, 200);
        return Err(format!("Webshop request failed ({endpoint}): {status} {body}").into());
    }
    Ok((body, extract_set_cookies(&headers)))
}

fn extract_set_cookies(headers: &HeaderMap) -> Vec<String> {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn build_cookie_header(char_id: Option<&str>, extra_cookies: &[String]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(char_id) = char_id {
        if !char_id.trim().is_empty() {
            parts.push(format!("characterid={char_id}"));
        }
    }
    for cookie in extra_cookies {
        if !cookie.trim().is_empty() {
            parts.push(cookie.trim().to_string());
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

fn truncate_for_log(input: &str, max_len: usize) -> String {
    let mut cleaned = input.replace('\r', "\\r").replace('\n', "\\n");
    if cleaned.len() > max_len {
        cleaned.truncate(max_len);
        cleaned.push_str("...");
    }
    cleaned
}

fn normalize_server_host(host: &str) -> String {
    let trimmed = host.trim();
    let trimmed = trimmed.strip_prefix("https://").unwrap_or(trimmed);
    let trimmed = trimmed.strip_prefix("http://").unwrap_or(trimmed);
    trimmed.trim_end_matches('/').to_ascii_lowercase()
}

fn is_rate_limit_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("rate limit")
}

fn random_coupon_delay_ms() -> u64 {
    fastrand::u64(10_000..20_001)
}

async fn fetch_server_id_map() -> WebshopResult<HashMap<String, i32>> {
    let response = WEB_SHOP_CLIENT
        .get(SERVER_CONFIG_URL)
        .header("Accept", "application/json, text/plain, */*")
        .header("User-Agent", WEB_SHOP_USER_AGENT)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        let body = truncate_for_log(&body, 200);
        return Err(format!("Server config request failed: {status} {body}").into());
    }

    let config: ServerConfig = serde_json::from_str(&body)?;
    let mut map = HashMap::new();
    for server in config.servers {
        let domain = normalize_server_host(&server.domain);
        if !domain.is_empty() {
            map.insert(domain, server.id);
        }
        if let Some(merged) = server.merged_into {
            let merged = normalize_server_host(&merged);
            if !merged.is_empty() {
                map.insert(merged, server.id);
            }
        }
    }

    if map.is_empty() {
        return Err("Server config returned no servers".into());
    }

    Ok(map)
}

async fn get_server_id(server_host: &str) -> WebshopResult<i32> {
    let host = normalize_server_host(server_host);
    if host.is_empty() {
        return Err("Server host is empty".into());
    }

    {
        let guard = SERVER_ID_CACHE.lock().await;
        if let Some(map) = guard.as_ref() {
            if let Some(id) = map.get(&host) {
                return Ok(*id);
            }
        }
    }

    let map = fetch_server_id_map().await?;
    let mut guard = SERVER_ID_CACHE.lock().await;
    *guard = Some(map);
    guard
        .as_ref()
        .and_then(|map| map.get(&host).copied())
        .ok_or_else(|| format!("Server id not found for {host}").into())
}

fn build_payment_string(gs: &GameState, server_id: i32) -> Option<String> {
    let player_id = gs.character.player_id;
    let player_save_id = gs.character.player_save_id;
    if player_id == 0 || player_save_id == 0 {
        return None;
    }
    Some(format!("{player_id}_{player_save_id}_{server_id}_1"))
}

fn parse_coupon_response(
    status: reqwest::StatusCode,
    body: &str,
) -> CouponOutcome {
    let mut message = String::new();
    let mut success = status.is_success();

    if let Ok(parsed) = serde_json::from_str::<CouponApiResponse>(body) {
        if let Some(msg) = parsed.message {
            message = msg;
        }
        if let Some(flag) = parsed.success {
            success = flag;
        }
        if let Some(status_field) = parsed.status {
            if status_field.eq_ignore_ascii_case("error") {
                success = false;
            } else if status_field.eq_ignore_ascii_case("success") {
                success = true;
            }
        }
    }

    if message.is_empty() {
        if success {
            message = "Coupon redeemed".to_string();
        } else {
            message = format!("Coupon request failed ({status})");
        }
    }

    let retryable = !success
        && (status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || is_rate_limit_message(&message));

    CouponOutcome {
        success,
        message,
        retryable,
    }
}

async fn send_coupon_request(
    code: &str,
    payment_string: &str,
) -> WebshopResult<CouponHttpResponse> {
    let response = WEB_SHOP_CLIENT
        .post(COUPON_REDEEM_URL)
        .header("Accept", "application/json, text/plain, */*")
        .header("User-Agent", WEB_SHOP_USER_AGENT)
        .form(&[
            ("coupon", code),
            ("paymentstring", payment_string),
            ("lang", "en"),
        ])
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    Ok(CouponHttpResponse { status, body })
}

async fn fetch_free_article(
    cookie_header: Option<&str>,
) -> WebshopResult<Option<WebShopArticle>> {
    let catalog_resp = send_webshop_request("catalog", cookie_header).await?;
    let catalog: WebShopResponse = serde_json::from_str(&catalog_resp)?;
    if !catalog.success {
        return Ok(None);
    }

    Ok(catalog
        .catalog
        .and_then(|catalog| {
            catalog
                .articles
                .into_iter()
                .find(|article| article.sku == "FREE")
        }))
}

pub async fn claim_free_mushroom(
    session: &mut SimpleSession,
) -> WebshopResult<String> {
    let gs = session.send_command(Command::Update).await?.clone();
    let enable_collect: bool =
        fetch_character_setting(&gs, "miscCollectFreeMushroom")
            .unwrap_or(false);
    if !enable_collect {
        return Ok(String::new());
    }

    let char_ids = get_webshop_character_ids(&gs);
    if char_ids.is_empty() {
        return Ok(String::new());
    }

    let server_host = session.server_url().host_str().unwrap_or("").to_string();
    if server_host.is_empty() {
        return Ok(String::new());
    }

    let server_host_encoded = url_encode(&server_host);
    let mut active_char_id_encoded: Option<String> = None;
    let mut last_error: Option<String> = None;
    let mut cookies: Vec<String> = Vec::new();
    for char_id in char_ids {
        let char_id_encoded = url_encode(&char_id);
        match send_webshop_request_with_cookies(
            &format!(
                "setServer?server={server_host_encoded}&characterid={char_id_encoded}"
            ),
            None,
        )
        .await
        {
            Ok((login_resp, set_cookies)) => {
                if login_resp.trim() == "[]" {
                    active_char_id_encoded = Some(char_id_encoded);
                    cookies = set_cookies;
                    break;
                }
                last_error = Some(format!(
                    "Webshop: setServer failed: {}",
                    login_resp.trim()
                ));
            }
            Err(err) => {
                last_error = Some(err.to_string());
            }
        }
    }
    let Some(char_id_encoded) = active_char_id_encoded else {
        let _ = last_error;
        return Ok(String::new());
    };
    let cookie_header = build_cookie_header(Some(&char_id_encoded), &cookies);

    let mut result = String::new();
    if let Some(free_item) = fetch_free_article(cookie_header.as_deref()).await? {
        if free_item.valid_again_in_seconds.is_none() {
            let _ = send_webshop_request(
                &format!(
                    "checkout?identifier={}&countrycode=en&affiliatecode=",
                    free_item.identifier
                ),
                cookie_header.as_deref(),
            )
            .await?;
            result = "Collected the free mushroom from the WebShop".to_string();
        }
    }

    Ok(result)
}

async fn redeem_coupon_for_session(
    session: &mut SimpleSession,
    code: &str,
) -> CouponRedeemResult {
    let mut result = CouponRedeemResult {
        name: session.username().to_string(),
        id: 0,
        server: session
            .server_url()
            .host_str()
            .unwrap_or("")
            .to_string(),
        success: false,
        message: String::new(),
    };

    let max_attempts = 3;
    for attempt in 1..=max_attempts {
        let gs = match session.send_command(Command::Update).await {
            Ok(gs) => gs.clone(),
            Err(err) => {
                result.message = format!("Update failed: {err}");
                if attempt < max_attempts {
                    sleep(Duration::from_millis(random_coupon_delay_ms())).await;
                    continue;
                }
                return result;
            }
        };

        result.name = gs.character.name.clone();
        result.id = gs.character.player_id;
        result.server = session
            .server_url()
            .host_str()
            .unwrap_or("")
            .to_string();

        if result.server.is_empty() {
            result.message = "Missing server host".to_string();
            return result;
        }

        let server_id = match get_server_id(&result.server).await {
            Ok(id) => id,
            Err(err) => {
                result.message = format!("Server id lookup failed: {err}");
                if attempt < max_attempts {
                    sleep(Duration::from_millis(random_coupon_delay_ms())).await;
                    continue;
                }
                return result;
            }
        };

        let payment_string = match build_payment_string(&gs, server_id) {
            Some(payment_string) => payment_string,
            None => {
                result.message = "Missing player save id".to_string();
                return result;
            }
        };

        let outcome = match send_coupon_request(code, &payment_string).await {
            Ok(resp) => parse_coupon_response(resp.status, &resp.body),
            Err(err) => CouponOutcome {
                success: false,
                message: err.to_string(),
                retryable: true,
            },
        };

        result.success = outcome.success;
        result.message = outcome.message;

        if result.success {
            return result;
        }

        if attempt < max_attempts {
            sleep(Duration::from_millis(random_coupon_delay_ms())).await;
        }
    }

    result
}

pub async fn redeem_coupon_for_all(
    code: &str,
) -> WebshopResult<CouponRedeemSummary> {
    let code = code.trim();
    if code.is_empty() {
        return Err("Coupon code is empty".into());
    }

    let session_states = get_all_session_states();
    if session_states.is_empty() {
        return Err("No active sessions available".into());
    }

    let mut results = Vec::with_capacity(session_states.len());
    let last_index = session_states.len().saturating_sub(1);
    for (idx, mut session_state) in session_states.into_iter().enumerate() {
        let result =
            redeem_coupon_for_session(&mut session_state.session, code).await;
        results.push(result);
        if idx < last_index {
            sleep(Duration::from_millis(random_coupon_delay_ms())).await;
        }
    }

    let applied = results.iter().filter(|r| r.success).count();
    let failed = results.len().saturating_sub(applied);

    Ok(CouponRedeemSummary {
        applied,
        failed,
        results,
    })
}
