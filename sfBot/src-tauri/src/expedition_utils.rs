#![allow(warnings)]

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    sync::Mutex,
};

use chrono::Local;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sf_api::gamestate::{
    items::{Equipment, EquipmentSlot},
    rewards::{Reward, RewardType},
    tavern::ExpeditionThing,
};

use crate::paths::exe_relative_path;
use crate::utils::shitty_print;

pub static CHARACTER_ENCOUNTER_COUNTERS: Lazy<Mutex<HashMap<String, HashMap<ExpeditionThing, u32>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub static EXPEDITION_STATS: Lazy<Mutex<HashMap<ExpeditionThing, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct ExpeditionStats
{
    picked: u32,
    encounters: HashMap<String, u32>,
    heroism_total: u64,
    heroism_max: u32,
    heroism_last: u32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct CharacterExpeditionStats
{
    character: String,
    character_id: u32,
    server: String,
    expeditions: HashMap<String, ExpeditionStats>,
    modes: HashMap<String, ModeExpeditionStats>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct ModeExpeditionStats
{
    expeditions: HashMap<String, ExpeditionStats>,
}

#[derive(Default)]
struct AggregateStats
{
    picked: u64,
    heroism_total: u64,
    heroism_max: u32,
    keys: u64,
    chests: u64,
}

/// Increment the encounter count for a specific character.
pub fn increment_encounter_count(character_name: &str, encounter: ExpeditionThing)
{
    let mut char_counters = CHARACTER_ENCOUNTER_COUNTERS.lock().unwrap();
    let counters = char_counters.entry(character_name.to_string()).or_insert_with(HashMap::new);
    *counters.entry(encounter).or_insert(0) += 1;
}

/// Get the encounter count for a specific character and expedition thing.
pub fn get_encounter_count(character_name: &str, encounter: ExpeditionThing) -> u32
{
    let char_counters = CHARACTER_ENCOUNTER_COUNTERS.lock().unwrap();
    char_counters.get(character_name).and_then(|counters| counters.get(&encounter).cloned()).unwrap_or(0)
}

/// Print all encounter counts for a specific character.
pub fn print_all_encounter_counts(character_name: &str)
{
    let char_counters = CHARACTER_ENCOUNTER_COUNTERS.lock().unwrap();
    if let Some(counters) = char_counters.get(character_name)
    {
        shitty_print(format!("Encounter Counts for {}:", character_name));
        for (encounter, count) in counters.iter()
        {
            shitty_print(format!("{:?}: {}", encounter, count));
        }
    }
}

/// Clear all encounter counts for a specific character.
pub fn clear_all_encounters_counts(character_name: &str)
{
    let mut char_counters = CHARACTER_ENCOUNTER_COUNTERS.lock().unwrap();
    if let Some(counters) = char_counters.get_mut(character_name)
    {
        counters.clear();
    }
}

/// Get all encounter counts for a specific character.
pub fn get_all_encounters_counts(character_name: &str) -> HashMap<ExpeditionThing, u32>
{
    let char_counters = CHARACTER_ENCOUNTER_COUNTERS.lock().unwrap();
    char_counters.get(character_name).cloned().unwrap_or_default()
}

/// Log expedition information for a specific character.
pub fn log_expedition_info(character_name: &str, character_id: u32, server: &str, mode: &str, current_floor: u8, chosen_expedition_type: Option<&ExpeditionThing>, active_heroism: u32, encounter_counts: &HashMap<ExpeditionThing, u32>)
{
    if current_floor != 10
    {
        return;
    }

    let sanitized_heroism = sanitize_heroism(active_heroism);

    if let Some(expedition_type) = chosen_expedition_type
    {
        if let Err(err) = update_expedition_stats(character_name, character_id, server, mode, expedition_type, sanitized_heroism, encounter_counts)
        {
            eprintln!("Failed to update expedition stats: {}", err);
        }
    }

    let log_folder = exe_relative_path("expedition_logs");
    if !log_folder.exists()
    {
        fs::create_dir_all(&log_folder).expect("Failed to create log folder");
    }

    let log_file_name = log_folder.join(format!("{}_expedition_log.txt", character_name));
    let mut log_file = OpenOptions::new().create(true).append(true).open(&log_file_name).expect("Failed to open or create log file");

    let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string();
    log_file.write_all(format!("\n{}\n", timestamp).as_bytes()).expect("Failed to write timestamp to log file");

    if let Some(expedition_type) = chosen_expedition_type
    {
        let expedition_message = format!("Expedition chosen: {:?}\t", expedition_type);
        log_file.write_all(expedition_message.as_bytes()).expect("Failed to write expedition info to log file");
    }

    let encounter_counts_message = format!("Encounter Counts: {:?}\t", encounter_counts);
    log_file.write_all(encounter_counts_message.as_bytes()).expect("Failed to write encounter counts to log file");

    let heroism_message = format!("Heroism: {}\n", sanitized_heroism);
    log_file.write_all(heroism_message.as_bytes()).expect("Failed to write heroism info to log file");
}

fn update_expedition_stats(character_name: &str, character_id: u32, server: &str, mode: &str, expedition_type: &ExpeditionThing, active_heroism: u32, encounter_counts: &HashMap<ExpeditionThing, u32>) -> Result<(), Box<dyn std::error::Error>>
{
    let stats_folder = exe_relative_path("expeditions_stats");
    if !stats_folder.exists()
    {
        fs::create_dir_all(&stats_folder)?;
    }

    let safe_name = sanitize_filename(character_name);
    let safe_server = sanitize_filename_with_fallback(&server.to_lowercase(), "unknown");
    let stats_file = stats_folder.join(format!("{}_{}_{}_expeditions.json", safe_name, safe_server, character_id));
    let legacy_file = stats_folder.join(format!("{}_expeditions.json", safe_name));

    let mut stats: CharacterExpeditionStats = if stats_file.exists()
    {
        let raw = fs::read_to_string(&stats_file).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    }
    else if legacy_file.exists()
    {
        let raw = fs::read_to_string(&legacy_file).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    }
    else
    {
        CharacterExpeditionStats::default()
    };

    if stats.character.is_empty()
    {
        stats.character = character_name.to_string();
    }
    if stats.character_id == 0
    {
        stats.character_id = character_id;
    }
    if stats.server.is_empty()
    {
        stats.server = server.to_lowercase();
    }

    let expedition_key = format!("{:?}", expedition_type);
    let expedition_entry = stats.expeditions.entry(expedition_key).or_insert_with(ExpeditionStats::default);
    update_expedition_entry(expedition_entry, active_heroism, encounter_counts);

    if !mode.is_empty()
    {
        let mode_key = mode.to_lowercase();
        let mode_entry = stats.modes.entry(mode_key).or_insert_with(ModeExpeditionStats::default);
        let mode_expedition_entry = mode_entry.expeditions.entry(format!("{:?}", expedition_type)).or_insert_with(ExpeditionStats::default);
        update_expedition_entry(mode_expedition_entry, active_heroism, encounter_counts);
    }

    let serialized = serde_json::to_string_pretty(&stats)?;
    fs::write(stats_file, serialized.as_bytes())?;
    Ok(())
}

fn update_expedition_entry(entry: &mut ExpeditionStats, active_heroism: u32, encounter_counts: &HashMap<ExpeditionThing, u32>)
{
    entry.picked = entry.picked.saturating_add(1);
    entry.heroism_total = entry.heroism_total.saturating_add(active_heroism as u64);
    if active_heroism > entry.heroism_max
    {
        entry.heroism_max = active_heroism;
    }
    entry.heroism_last = active_heroism;

    for (encounter, count) in encounter_counts
    {
        let encounter_key = format!("{:?}", encounter);
        let entry = entry.encounters.entry(encounter_key).or_insert(0);
        *entry = entry.saturating_add(*count);
    }
}

fn sanitize_heroism(value: u32) -> u32
{
    if value > 100
    {
        fastrand::u32(0..=40)
    }
    else
    {
        value
    }
}

pub fn read_expedition_stats(character_name: &str, character_id: u32, server: &str) -> Result<Option<Value>, String>
{
    let stats_folder = exe_relative_path("expeditions_stats");
    if !stats_folder.exists()
    {
        return Ok(None);
    }

    let safe_name = sanitize_filename(character_name);
    let safe_server = sanitize_filename_with_fallback(&server.to_lowercase(), "unknown");
    let stats_file = stats_folder.join(format!("{}_{}_{}_expeditions.json", safe_name, safe_server, character_id));
    let legacy_file = stats_folder.join(format!("{}_expeditions.json", safe_name));

    let stats_path = if stats_file.exists()
    {
        stats_file
    }
    else if legacy_file.exists()
    {
        legacy_file
    }
    else
    {
        return Ok(None);
    };

    let raw = fs::read_to_string(&stats_path).map_err(|e| e.to_string())?;
    let stats = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(Some(stats))
}

pub fn read_expedition_summary() -> Result<Value, String>
{
    let stats_folder = exe_relative_path("expeditions_stats");
    if !stats_folder.exists()
    {
        return Ok(serde_json::json!({ "expeditions": {}, "modes": {} }));
    }

    let mut aggregated: HashMap<String, AggregateStats> = HashMap::new();
    let mut aggregated_modes: HashMap<String, HashMap<String, AggregateStats>> = HashMap::new();

    let entries = fs::read_dir(&stats_folder).map_err(|e| e.to_string())?;
    for entry in entries
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if !file_type.is_file()
        {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()).unwrap_or("") != "json"
        {
            continue;
        }

        let raw = match fs::read_to_string(&path)
        {
            Ok(content) => content,
            Err(_) => continue,
        };

        let data: Value = match serde_json::from_str(&raw)
        {
            Ok(value) => value,
            Err(_) => continue,
        };

        let expeditions = match data.get("expeditions").and_then(|v| v.as_object())
        {
            Some(obj) => obj,
            None => continue,
        };

        accumulate_expeditions(expeditions, &mut aggregated);

        if let Some(modes) = data.get("modes").and_then(|v| v.as_object())
        {
            for (mode_key, mode_data) in modes
            {
                if let Some(mode_expeditions) = mode_data.get("expeditions").and_then(|v| v.as_object())
                {
                    let mode_entry = aggregated_modes.entry(mode_key.clone()).or_default();
                    accumulate_expeditions(mode_expeditions, mode_entry);
                }
            }
        }
    }

    let mut expeditions_json = serde_json::Map::new();
    for (expedition_name, stats) in aggregated
    {
        expeditions_json.insert(
            expedition_name,
            serde_json::json!({
                "picked": stats.picked,
                "heroism_total": stats.heroism_total,
                "heroism_max": stats.heroism_max,
                "keys": stats.keys,
                "chests": stats.chests
            }),
        );
    }

    let mut modes_json = serde_json::Map::new();
    for (mode_name, mode_stats) in aggregated_modes
    {
        let mut mode_expeditions_json = serde_json::Map::new();
        for (expedition_name, stats) in mode_stats
        {
            mode_expeditions_json.insert(
                expedition_name,
                serde_json::json!({
                    "picked": stats.picked,
                    "heroism_total": stats.heroism_total,
                    "heroism_max": stats.heroism_max,
                    "keys": stats.keys,
                    "chests": stats.chests
                }),
            );
        }
        modes_json.insert(mode_name, serde_json::json!({ "expeditions": mode_expeditions_json }));
    }

    Ok(serde_json::json!({ "expeditions": expeditions_json, "modes": modes_json }))
}

fn get_encounter_total(encounters: Option<&serde_json::Map<String, Value>>, names: &[&str]) -> u64
{
    let mut total = 0u64;
    if let Some(map) = encounters
    {
        for name in names
        {
            if let Some(value) = map.get(*name).and_then(|v| v.as_u64())
            {
                total = total.saturating_add(value);
            }
        }
    }
    total
}

fn accumulate_expeditions(expeditions: &serde_json::Map<String, Value>, aggregated: &mut HashMap<String, AggregateStats>)
{
    for (expedition_name, expedition_data) in expeditions
    {
        let picked = expedition_data.get("picked").and_then(|v| v.as_u64()).unwrap_or(0);
        let heroism_total = expedition_data.get("heroism_total").and_then(|v| v.as_u64()).unwrap_or(0);
        let heroism_max = expedition_data.get("heroism_max").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let encounters = expedition_data.get("encounters").and_then(|v| v.as_object());
        let keys = get_encounter_total(encounters, &["Key", "Keys"]);
        let chests = get_encounter_total(encounters, &["Suitcase", "Chests"]);

        let entry = aggregated.entry(expedition_name.clone()).or_default();
        entry.picked = entry.picked.saturating_add(picked);
        entry.heroism_total = entry.heroism_total.saturating_add(heroism_total);
        entry.keys = entry.keys.saturating_add(keys);
        entry.chests = entry.chests.saturating_add(chests);
        if heroism_max > entry.heroism_max
        {
            entry.heroism_max = heroism_max;
        }
    }
}

fn sanitize_filename(name: &str) -> String
{
    sanitize_filename_with_fallback(name, "character")
}

fn sanitize_filename_with_fallback(name: &str, fallback: &str) -> String
{
    let mut sanitized = String::with_capacity(name.len());
    for ch in name.chars()
    {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
        {
            sanitized.push(ch);
        }
        else
        {
            sanitized.push('_');
        }
    }

    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty()
    {
        fallback.to_string()
    }
    else
    {
        trimmed.to_string()
    }
}

pub fn should_buy_beer(character_equip: &Equipment, amount_of_beers_to_drink: u8, amount_of_beers_drunk: u8, amount_of_beers_max: u8, current_thirst: u32, current_mushroom_amount: u32, shrooms_to_keep: u32) -> bool
{
    let not_enough_mushrooms = current_mushroom_amount <= shrooms_to_keep;
    if (not_enough_mushrooms)
    {
        let equip = &character_equip.0;
        let belt = equip[EquipmentSlot::Belt].as_ref();
        if let Some(belt_item) = belt
        {
            if current_thirst == 0 && amount_of_beers_drunk == 0
            {
                return belt_item.enchantment.is_some();
            }
        }
    }

    if (amount_of_beers_drunk >= amount_of_beers_max)
    {
        return false;
    }

    if (amount_of_beers_drunk >= amount_of_beers_to_drink)
    {
        return false;
    }

    if (not_enough_mushrooms)
    {
        return false;
    }

    if current_thirst == 0 && current_mushroom_amount > 0
    {
        return true;
    }

    false
}

pub fn should_buy_beer_old(beers_to_drink: u8, beers_drunk: u8, beers_max: u8, current_thirst: u32, current_mushroom_amount: u16) -> bool
{
    if current_thirst == 0 && beers_to_drink > beers_drunk && current_mushroom_amount > 0
    {
        return true;
    }
    false
}

pub fn is_expedition_still_completeable(chosen_expedition_type: Option<&ExpeditionThing>, current_floor: u8, char_name: &str) -> bool
{
    match chosen_expedition_type
    {
        Some(ExpeditionThing::Unicorn) if get_encounter_count(char_name, ExpeditionThing::UnicornHorn) == 0 && current_floor == 8 =>
        {
            return false;
        }
        Some(ExpeditionThing::Klaus) if get_encounter_count(char_name, ExpeditionThing::Hand) == 0 && current_floor == 8 =>
        {
            return false;
        }
        Some(ExpeditionThing::RevealingCouple) if get_encounter_count(char_name, ExpeditionThing::Socks) == 0 && current_floor == 9 =>
        {
            return false;
        }
        Some(ExpeditionThing::Balloons) if get_encounter_count(char_name, ExpeditionThing::Well) == 0 && current_floor == 9 =>
        {
            return false;
        }
        Some(ExpeditionThing::WinnersPodium) if get_encounter_count(char_name, ExpeditionThing::SmallHurdle) == 0 && current_floor == 9 =>
        {
            return false;
        }
        Some(ExpeditionThing::BurntCampfire) if get_encounter_count(char_name, ExpeditionThing::CampFire) == 0 && current_floor == 9 =>
        {
            return false;
        }
        Some(ExpeditionThing::BrokenSword) if get_encounter_count(char_name, ExpeditionThing::SwordInStone) == 0 && current_floor == 9 =>
        {
            return false;
        }
        // expedition is completable if it doesn't match the above cases
        Some(_) => return true,
        None => return false,
    }
}

pub fn select_best_expedition_reward_based_on_priority(rewards_to_pick_from: &[Reward], reward_priority_map: &HashMap<RewardType, usize>) -> Option<usize>
{
    if rewards_to_pick_from.is_empty()
    {
        eprintln!("No rewards available to select from.");
        return None;
    }

    let mut best_pos = 0;
    let mut best_priority = reward_priority_map.get(&rewards_to_pick_from[0].typ).copied().unwrap_or(usize::MAX);

    for (i, reward) in rewards_to_pick_from.iter().enumerate()
    {
        let priority = reward_priority_map.get(&reward.typ).copied().unwrap_or(usize::MAX);

        if priority < best_priority
        {
            best_priority = priority;
            best_pos = i;
        }
    }

    // println!("Reward chosen at position {} with type {:?} (Priority: {:?})",
    // best_pos, rewards_to_pick_from[best_pos].typ, best_priority);

    Some(best_pos)
}

// pub fn select_best_expedition_reward_based_on_priority(rewards_to_pick_from:
// &[Reward], reward_priority_map: &HashMap<RewardType, usize>) -> Option<usize>
// {
//     if rewards_to_pick_from.is_empty()
//     {
//         eprintln!("No rewards available to select from.");
//         return None;
//     }
//
//     let mut best_pos = 0;
//     let mut best_priority =
// reward_priority_map.get(&rewards_to_pick_from[0].typ).copied().
// unwrap_or(u32::MAX);
//
//     for (i, reward) in rewards_to_pick_from.iter().enumerate()
//     {
//         let priority =
// reward_priority_map.get(&reward.typ).copied().unwrap_or(u32::MAX);
//
//         if priority < best_priority
//         {
//             best_priority = priority;
//             best_pos = i;
//         }
//     }
//
//     println!("Reward chosen at position {} with type {:?} (Priority: {:?})",
// best_pos, rewards_to_pick_from[best_pos].typ, best_priority);
//     Some(best_pos)
// }

// pub fn select_best_expedition_reward_based_on_priority(rewards_to_pick_from:
// &[Reward]) -> Option<usize> {
//     if rewards_to_pick_from.is_empty()
//     {
//         eprintln!("No rewards available to select from.");
//         return None;
//     }
//
//     let mut best_pos = 0;
//     let mut best_priority =
// rewards_to_pick_from[0].typ.priority().unwrap_or(u32::MAX);
//
//     for (i, reward) in rewards_to_pick_from.iter().enumerate()
//     {
//         let priority = reward.typ.priority().unwrap_or(u32::MAX);
//
//         if priority < best_priority
//         {
//             best_priority = priority;
//             best_pos = i;
//         }
//     }
//     shitty_print(format!("Reward chosen at position {} with type {:?}
// (Priority: {:?})", best_pos, rewards_to_pick_from[best_pos].typ,
// best_priority));     Some(best_pos)
// }
//
// trait RewardTypePriority
// {
//     fn priority(&self) -> Option<u32>;
// }
// impl RewardTypePriority for RewardType
// {
//     fn priority(&self) -> Option<u32>
//     {
//         match self
//         {
//             //heir prio liste rein
//             RewardType::Egg => Some(0),
//             RewardType::Mushrooms => Some(1),
//             RewardType::Silver => Some(2),
//             RewardType::FruitBasket => Some(3),
//             RewardType::QuicksandGlass => Some(4),
//             RewardType::LuckyCoins => Some(6),
//             RewardType::Souls => Some(12),
//             RewardType::Arcane => Some(14),
//             RewardType::Metal => Some(15),
//             RewardType::Fruit(_) => Some(16),
//             RewardType::Stone => Some(97),
//             RewardType::Wood => Some(98),
//             RewardType::XP => Some(9999),
//             RewardType::Honor => Some(9999),
//             RewardType::HellevatorPoints => Some(9999),
//             RewardType::HellevatorCards => Some(9999),
//             RewardType::LegendaryGem => Some(9999),
//             RewardType::Beer => Some(9999),
//             RewardType::Mount(_) => Some(9999),
//             RewardType::SilverFidget => Some(9999),
//             RewardType::BronzeFidget => Some(9999),
//             RewardType::Gem => Some(9999),
//             RewardType::GoldFidget => Some(9999),
//             RewardType::Frame => Some(9999),
//             RewardType::Unknown => None,
//         }
//     }
// }
