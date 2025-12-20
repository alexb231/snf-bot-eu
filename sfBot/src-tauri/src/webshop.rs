use std::error::Error;

use once_cell::sync::Lazy;
use reqwest::Client;
use reqwest::header::{HeaderMap, SET_COOKIE};
use serde::Deserialize;
use sf_api::{command::Command, gamestate::GameState, SimpleSession};

use crate::fetch_character_setting;

type WebshopResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const WEB_SHOP_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

static WEB_SHOP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .gzip(true)
        .deflate(true)
        .build()
        .expect("failed to build webshop client")
});

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
