#![allow(warnings)]

use std::{collections::HashMap, error::Error, sync::Mutex};

use chrono::{Duration, Local};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use serde_json::json;
use sf_api::{
    command::{Command, Command::ViewPlayer},
    gamestate::{
        character::Class,
        items::{EquipmentSlot, InventoryType, Item, ItemPlace, ItemPlace::MainInventory, PlayerItemPlace},
        rewards::{DailyTasks, TaskType},
        tavern::GambleResult,
    },
    SimpleSession,
};

use crate::{fetch_character_setting, inventory_management::sorted_items_with_indices, utils::shitty_print};

#[derive(Debug, Deserialize, Clone)]
struct PlayerEntry
{
    server: String,
    player1: String,
}

static CLASS_MAP_CACHE: OnceCell<Mutex<Option<HashMap<String, Vec<PlayerEntry>>>>> = OnceCell::new();

fn get_cache() -> &'static Mutex<Option<HashMap<String, Vec<PlayerEntry>>>> { CLASS_MAP_CACHE.get_or_init(|| Mutex::new(None)) }

pub async fn perform_daily_tasks(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let mut result = String::from("");
    let gs = session.send_command(Command::Update).await?.clone();
    let enable_gambling: bool = fetch_character_setting(&gs, "miscPerformDailyGambling").unwrap_or(false);
    let dt = gs.specials.tasks.daily.clone();
    if (enable_gambling && should_do_daily_task(dt.clone(), TaskType::DefeatGambler))
    {
        let res = gamble(session).await?;
        result += res.as_str();
    }
    let enable_bare_hand_atk: bool = fetch_character_setting(&gs, "miscPerformDailyBareHand").unwrap_or(false);
    if enable_bare_hand_atk && should_do_daily_task(dt.clone(), TaskType::WinFightsBareHands)
    {
        let weapon_equipped_before = &gs.character.equipment.0[EquipmentSlot::Weapon];
        // TODO! readd bare hand attacks
        let res = bare_handed_attack_task(session).await?;

        if let Some(weapon) = weapon_equipped_before
        {
            ensure_weapon_is_equipped_back(session, weapon.clone()).await?;
        }
        result += res.as_str();
    }

    // 11 classes
    let dt = dt.clone();

    let class_settings = vec![
        ("miscPerformDailyFightScout", Class::Scout),
        ("miscPerformDailyFightMage", Class::Mage),
        ("miscPerformDailyFightAssassin", Class::Assassin),
        ("miscPerformDailyFightBattleMage", Class::BattleMage),
        ("miscPerformDailyFightDruid", Class::Druid),
        ("miscPerformDailyFightDemonHunter", Class::DemonHunter),
        ("miscPerformDailyFightBard", Class::Bard),
        ("miscPerformDailyFightNecromancer", Class::Necromancer),
        ("miscPerformDailyFightPaladin", Class::Paladin),
        ("miscPerformDailyFightWarrior", Class::Warrior),
        ("miscPerformDailyFightBerserker", Class::Berserker),
    ];

    for (setting_key, class_type) in class_settings
    {
        if fetch_character_setting(&gs, setting_key).unwrap_or(false) && should_do_daily_task(dt.clone(), TaskType::WinFightsAgainst(class_type.clone()))
        {
            let res = fight_against_class_daily(session, class_type).await?;
            result += res.as_str();
        }
    }

    let mut finalMessage = String::from("");
    if (result != "")
    {
        finalMessage += "Performed daily tasks: ";
        finalMessage += &result;
    }

    return Ok(finalMessage);
}

pub async fn ensure_weapon_is_equipped_back(session: &mut SimpleSession, weapon_equipped_before: Item) -> Result<(), Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let inventory = &gs.character.inventory;
    let inventory_sorted = sorted_items_with_indices(inventory);
    for (pos, item) in inventory_sorted.iter()
    {
        if **item == weapon_equipped_before
        {
            let equip_weapon = Command::ItemMove {
                from: MainInventory,
                from_pos: *pos,
                to: ItemPlace::Equipment,
                to_pos: 8,
            };
            session.send_command(equip_weapon).await?;
            return Ok(());
        }
    }
    return Ok(());
}

pub async fn gamble(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut won = 0;
    let mut lost = 0;

    loop
    {
        let gs = session.send_command(Command::Update).await?;

        if let Some(GambleResult::SilverChange(res)) = gs.tavern.gamble_result
        {
            if res > 0
            {
                won += 1;
            }
            else
            {
                lost += 1;
            }
        }

        if won == 3
        {
            break;
        }

        if gs.character.silver == 0
        {
            break;
        }

        session.send_command(Command::GambleSilver { amount: 1 }).await?;
    }

    Ok(String::from(" Gambling"))
}

pub async fn fight_against_class_daily(session: &mut SimpleSession, class_type: Class) -> Result<String, Box<dyn std::error::Error>>
{
    // https://downloader.sfbot.eu/updates/charsToFight.json
    let gs = session.send_command(Command::CheckArena).await?;
    if gs.arena.fights_for_xp < 10
    {
        return Ok(String::from(""));
    }

    let next_free_fight_unwrapped = match gs.arena.next_free_fight
    {
        None => return Ok(String::from("")),
        Some(arena) if Local::now() < arena + Duration::minutes(3) => return Ok(String::from("")),
        Some(arena) => arena,
    };

    let class_map = get_class_map().await?;

    let class_str = match class_type
    {
        Class::Warrior => "Warrior",
        Class::Mage => "Mage",
        Class::Scout => "Scout",
        Class::Assassin => "Assassin",
        Class::BattleMage => "Battle Mage",
        Class::Berserker => "Berserker",
        Class::DemonHunter => "Demon Hunter",
        Class::Druid => "Druid",
        Class::Bard => "Bard",
        Class::Necromancer => "Necromancer",
        Class::Paladin => "Paladin",
        Class::PlagueDoctor => "Plague Doctor",
    };

    let raw_server = session.server_url().to_string().trim_start_matches("https://").trim_end_matches('/').to_string();

    if let Some(entries) = class_map.get(class_str)
    {
        if let Some(entry) = entries.iter().find(|e| e.server == raw_server)
        {
            let lookup_gs = session.send_command(ViewPlayer { ident: entry.player1.to_string() }).await?;
            let lookup = &lookup_gs.lookup;
            let player = lookup.lookup_name(&*entry.player1.to_string());
            let player_metadata = match player
            {
                None =>
                {
                    send_to_hook(&format!("player doesn't exist {}", entry.player1)).await;
                    return Ok(String::from("looked up player doesnt exist"));
                }
                Some(p) => p.clone(),
            };
            let player_class = player_metadata.class;
            let player_level = player_metadata.level;
            let player_name = player_metadata.name.clone();
            if (player_level > 20)
            {
                send_to_hook(&format!("player is above level 20 might need replacement name: {} level: {}", entry.player1, player_level)).await;
            }

            if (player_class != class_type)
            {
                send_to_hook(&format!("playername:{} class doesnt match expected {:?} but is : {:?} for server: {}", player_metadata.name, class_type, player_metadata.class, raw_server)).await;
                return Ok(String::from("looked up player doesnt match"));
            }

            let gs = session.send_command(Command::CheckArena).await?;
            let free_fight_sanity_check = match gs.arena.next_free_fight
            {
                None => return Ok(String::from("")),
                Some(arena_fight_time) if arena_fight_time > Local::now() => return Ok(String::from("")),
                Some(arena_fight_time) => arena_fight_time,
            };
            session.send_command(Command::Fight { name: player_name.clone(), use_mushroom: false }).await?;

            return Ok(format!("Fighting player for daily task: {}", player_name));
        }
    }

    Ok(String::from(""))
}

async fn get_class_map() -> Result<HashMap<String, Vec<PlayerEntry>>, reqwest::Error>
{
    if let Some(cached) = {
        let cache_lock = get_cache();
        let cache = cache_lock.lock().unwrap();
        cache.clone()
    }
    {
        println!("[DEBUG] Returning class_map from cache.");
        return Ok(cached);
    }

    println!("[DEBUG] Fetching class_map from remote source...");
    let response = reqwest::get("https://downloader.sfbot.eu/updates/charsToFight.json").await?;
    let class_map: HashMap<String, Vec<PlayerEntry>> = response.json().await?;

    {
        let cache_lock = get_cache();
        let mut cache = cache_lock.lock().unwrap();
        *cache = Some(class_map.clone());
        println!("[DEBUG] Caching class_map for future use.");
    }

    Ok(class_map)
}
//
pub async fn bare_handed_attack_task(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::from(", Bare handed attack (");
    let gs = session.send_command(Command::CheckArena).await?.clone();
    let free_slots = gs.character.inventory.count_free_slots();
    let weapon_equipped = gs.character.equipment.0[EquipmentSlot::Weapon].is_some();
    if free_slots <= 0 && weapon_equipped
    {
        return Ok(String::from(""));
    }
    let arena = &gs.arena;
    let total_amount_of_players = gs.hall_of_fames.players_total;
    let pages = total_amount_of_players / 51; // sometimes HOF return -1
    if pages < 2
    {
        return Ok(String::from(""));
    }
    let max_page_to_attack_lowest_enemy = pages - 2;

    let current_time = Local::now();
    let current_time_minus_2 = current_time - Duration::minutes(2);

    let free_fight = if let Some(next_free_fight) = arena.next_free_fight { current_time_minus_2 >= next_free_fight } else { false };
    if free_fight
    {
        if let Some(slot) = gs.character.inventory.free_slot()
        {
            let index = slot.backpack_pos();
            let final_place = ItemPlace::MainInventory;

            // Unequip weapon if needed
            if weapon_equipped
            {
                let unequip_item = Command::ItemMove {
                    from: ItemPlace::Equipment,
                    from_pos: 8,
                    to: final_place,
                    to_pos: index,
                };
                session.send_command(unequip_item).await?;
                result += "unequipped weapon - ";
            }

            // Fetch HOF page
            let hall_of_fame_fetch = Command::HallOfFamePage { page: max_page_to_attack_lowest_enemy as usize };
            let updated_gs = session.send_command(hall_of_fame_fetch).await?;

            // Attack
            if let Some(player) = updated_gs.hall_of_fames.players.get(0)
            {
                let fight_player = Command::Fight { name: player.name.clone(), use_mushroom: false };
                result += &format!("fought opponent: {} - ", player.name);
                session.send_command(fight_player).await?;

                // Equip weapon back if needed
                if weapon_equipped
                {
                    let equip_item = Command::ItemMove {
                        from: final_place,
                        from_pos: index,
                        to: ItemPlace::Equipment,
                        to_pos: 8,
                    };
                    session.send_command(equip_item).await?;
                    result += "equipped weapon back on.)";
                }
            }
        }
    }
    return Ok(String::from(""));
}

pub fn should_do_daily_task(available_tasks: DailyTasks, task_type: TaskType) -> bool
{
    //
    return available_tasks.get_available(task_type).is_some();
}

async fn send_to_hook(message: &str)
{
    let payload = json!({
        "content": message
    });

    if let Err(e) = reqwest::Client::new().post("https://discord.com/api/webhooks/1362614373935354068/TGUXafY-GYTTUjFLGAcTLS_QtVMes_xRs_AakujyACSNw0ULaPLj0FedGpud-6nQh6Xr").json(&payload).send().await
    {
        eprintln!("Error sending webhook: {}", e);
    }
}
