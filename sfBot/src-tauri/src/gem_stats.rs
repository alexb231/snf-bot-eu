use std::{collections::HashMap, fs};

use serde::{Deserialize, Serialize};
use sf_api::gamestate::items::GemType;

use crate::paths::exe_relative_path;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct GemStats {
    total: u32,
    sizes: HashMap<String, u32>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct CharacterGemStats {
    character: String,
    character_id: u32,
    server: String,
    gems: HashMap<String, GemStats>,
}

pub fn record_gem_stat(
    character_name: &str,
    character_id: u32,
    server: &str,
    gem_type: GemType,
    gem_value: u32,
) {
    if let Err(err) =
        update_gem_stats(character_name, character_id, server, gem_type, gem_value)
    {
        eprintln!("Failed to update gem stats: {}", err);
    }
}

fn update_gem_stats(
    character_name: &str,
    character_id: u32,
    server: &str,
    gem_type: GemType,
    gem_value: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let stats_folder = exe_relative_path("gem_stats");
    if !stats_folder.exists() {
        fs::create_dir_all(&stats_folder)?;
    }

    let safe_name = sanitize_filename(character_name);
    let safe_server = sanitize_filename_with_fallback(&server.to_lowercase(), "unknown");
    let stats_file = stats_folder.join(format!(
        "{}_{}_{}_gems.json",
        safe_name, safe_server, character_id
    ));
    let legacy_file = stats_folder.join(format!("{}_gems.json", safe_name));

    let mut stats: CharacterGemStats = if stats_file.exists() {
        let raw = fs::read_to_string(&stats_file).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    } else if legacy_file.exists() {
        let raw = fs::read_to_string(&legacy_file).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        CharacterGemStats::default()
    };

    if stats.character.is_empty() {
        stats.character = character_name.to_string();
    }
    if stats.character_id == 0 {
        stats.character_id = character_id;
    }
    if stats.server.is_empty() {
        stats.server = server.to_lowercase();
    }

    let gem_key = format!("{:?}", gem_type);
    let gem_entry = stats.gems.entry(gem_key).or_insert_with(GemStats::default);
    gem_entry.total = gem_entry.total.saturating_add(1);

    let size_key = gem_value.to_string();
    let size_entry = gem_entry.sizes.entry(size_key).or_insert(0);
    *size_entry = size_entry.saturating_add(1);

    let serialized = serde_json::to_string_pretty(&stats)?;
    fs::write(stats_file, serialized.as_bytes())?;
    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    sanitize_filename_with_fallback(name, "character")
}

fn sanitize_filename_with_fallback(name: &str, fallback: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}
