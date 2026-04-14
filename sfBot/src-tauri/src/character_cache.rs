





use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::paths::get_cache_dir;


fn cache_dir() -> PathBuf {
    get_cache_dir()
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCharacter {
    pub id: u32,
    pub name: String,
    pub lvl: u16,
    pub alu: u32,
    pub guild: String,
    pub beers: u8,
    pub mushrooms: u32,
    pub hourglasses: u32,
    pub gold: u64,
    pub luckycoins: u32,
    pub fights: u8,
    pub luckyspins: u8,
    pub petfights: u8,
    pub dicerolls: u8,
    pub server: String,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    pub mount: String,
    pub account: String,
    
    pub cached_at: String,
}


pub fn should_update_cache(existing: Option<&CachedCharacter>) -> bool {
    match existing {
        Some(cached) => {
            if let Ok(ts) = chrono::NaiveDateTime::parse_from_str(&cached.cached_at, "%Y-%m-%dT%H:%M:%S") {
                let age = Local::now().naive_local() - ts;
                return age.num_minutes() >= 60;
            }
            true
        }
        None => true,
    }
}



fn get_cache_filename(name: &str, server: &str) -> PathBuf {
    
    let safe_name = name
        .to_lowercase()
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    let safe_server = normalize_server(server);

    cache_dir().join(format!("{}_{}.json", safe_name, safe_server))
}

fn normalize_server(server: &str) -> String {
    server
        .to_lowercase()
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        .replace("https_", "")
        .replace("http_", "")
        .replace(".sfgame.net", "")
        .replace(".sfgame.de", "")
        .replace(".sfgame.eu", "")
}


pub fn save_character_cache(character: &CachedCharacter) -> Result<(), String> {
    
    if let Err(e) = fs::create_dir_all(cache_dir()) {
        return Err(format!("Failed to create cache directory: {}", e));
    }

    let filename = get_cache_filename(&character.name, &character.server);
    let json = serde_json::to_string_pretty(character)
        .map_err(|e| format!("Failed to serialize character: {}", e))?;

    fs::write(&filename, json)
        .map_err(|e| format!("Failed to write cache file: {}", e))?;

    println!("[CACHE] Saved character {} on {} to {}", character.name, character.server, filename.display());
    Ok(())
}


pub fn load_character_cache(name: &str, server: &str) -> Result<Option<CachedCharacter>, String> {
    let filename = get_cache_filename(name, server);

    if !filename.exists() {
        
        let target_name = name.to_lowercase();
        let target_server = normalize_server(server);
        let characters = load_all_cached_characters()?;
        if let Some(found) = characters.into_iter().find(|c| {
            let cached_server = normalize_server(&c.server);
            c.name.to_lowercase() == target_name && (cached_server == target_server || cached_server.is_empty())
        }) {
            return Ok(Some(found));
        }
        return Ok(None);
    }

    let content = fs::read_to_string(&filename)
        .map_err(|e| format!("Failed to read cache file: {}", e))?;

    let character: CachedCharacter = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse cache file: {}", e))?;

    Ok(Some(character))
}


pub fn load_all_cached_characters() -> Result<Vec<CachedCharacter>, String> {
    let cache_path = cache_dir();

    
    if !cache_path.exists() {
        return Ok(Vec::new());
    }

    let mut characters = Vec::new();

    let entries = fs::read_dir(&cache_path)
        .map_err(|e| format!("Failed to read cache directory: {}", e))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[CACHE] Failed to read {}: {}", path.display(), e);
                continue;
            }
        };

        let character: CachedCharacter = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[CACHE] Failed to parse {}: {}", path.display(), e);
                continue;
            }
        };

        characters.push(character);
    }

    println!("[CACHE] Loaded {} cached characters", characters.len());
    Ok(characters)
}


pub fn delete_character_cache(name: &str, server: &str) -> Result<(), String> {
    let filename = get_cache_filename(name, server);

    if filename.exists() {
        fs::remove_file(&filename)
            .map_err(|e| format!("Failed to delete cache file: {}", e))?;
        println!("[CACHE] Deleted cache for {} on {}", name, server);
    }

    Ok(())
}


pub fn clear_all_cache() -> Result<(), String> {
    let cache_path = cache_dir();
    if cache_path.exists() {
        fs::remove_dir_all(&cache_path)
            .map_err(|e| format!("Failed to clear cache directory: {}", e))?;
        println!("[CACHE] Cleared all character cache");
    }
    Ok(())
}


#[derive(Debug, Clone)]
pub struct CharacterIdentity {
    pub id: u32,
    pub name: String,
}



pub fn get_character_identity(name: &str, server: &str) -> Result<Option<CharacterIdentity>, String> {
    let filename = get_cache_filename(name, server);

    if !filename.exists() {
        
        let target_name = name.to_lowercase();
        let target_server = normalize_server(server);
        let characters = load_all_cached_characters()?;
        if let Some(found) = characters.into_iter().find(|c| {
            let cached_server = normalize_server(&c.server);
            c.name.to_lowercase() == target_name && (cached_server == target_server || cached_server.is_empty())
        }) {
            return Ok(Some(CharacterIdentity {
                id: found.id,
                name: found.name,
            }));
        }
        return Ok(None);
    }

    let content = fs::read_to_string(&filename)
        .map_err(|e| format!("Failed to read cache file: {}", e))?;

    let character: CachedCharacter = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse cache file: {}", e))?;

    Ok(Some(CharacterIdentity {
        id: character.id,
        name: character.name,
    }))
}



pub fn update_character_active_status(name: &str, id: u32, is_active: bool) -> Result<(), String> {
    
    let characters = load_all_cached_characters()?;

    for mut character in characters {
        if character.name.to_lowercase() == name.to_lowercase() && character.id == id {
            
            character.is_active = is_active;
            save_character_cache(&character)?;
            println!("[CACHE] Updated is_active={} for {} (ID: {})", is_active, name, id);
            return Ok(());
        }
    }

    
    println!("[CACHE] Character {} (ID: {}) not found in cache, skipping update", name, id);
    Ok(())
}
