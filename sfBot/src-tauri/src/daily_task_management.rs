#![allow(warnings)]

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::Mutex,
};

use chrono::{Duration, Local};
use enum_map::Enum;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use sf_api::{
    command::{Command, Command::ViewPlayer},
    gamestate::{
        character::Class,
        items::{EquipmentSlot, InventoryType, Item, ItemCommandIdent, ItemPlace, ItemPlace::MainInventory, PlayerItemPlace},
        rewards::{DailyTasks, TaskType},
        tavern::GambleResult,
    },
    SimpleSession,
};

use crate::{bot_runner::write_character_log, fetch_character_setting, inventory_management::sorted_items_with_indices};

#[derive(Debug, Deserialize, Clone)]
struct PlayerEntry
{
    server: String,
    player1: String,
}

static CLASS_MAP_CACHE: OnceCell<Mutex<Option<HashMap<String, Vec<PlayerEntry>>>>> = OnceCell::new();
static BAD_TARGET_CACHE: OnceCell<Mutex<HashMap<String, HashSet<String>>>> = OnceCell::new();

fn get_cache() -> &'static Mutex<Option<HashMap<String, Vec<PlayerEntry>>>> { CLASS_MAP_CACHE.get_or_init(|| Mutex::new(None)) }
fn get_bad_target_cache() -> &'static Mutex<HashMap<String, HashSet<String>>> { BAD_TARGET_CACHE.get_or_init(|| Mutex::new(HashMap::new())) }

fn bad_target_key(server: &str, class_str: &str) -> String { format!("{}|{}", server, class_str) }

fn is_bad_target(server: &str, class_str: &str, player_name: &str) -> bool
{
    let cache = get_bad_target_cache().lock().unwrap();
    cache.get(&bad_target_key(server, class_str)).map_or(false, |set| set.contains(player_name))
}

fn mark_bad_target(server: &str, class_str: &str, player_name: &str)
{
    let mut cache = get_bad_target_cache().lock().unwrap();
    cache.entry(bad_target_key(server, class_str)).or_insert_with(HashSet::new).insert(player_name.to_string());
}

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
        let res = bare_handed_attack_task(session).await?;
        result += res.as_str();
    }

    
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
    
    let gs = session.send_command(Command::CheckArena).await?;
    if gs.arena.fights_for_xp < 10
    {
        return Ok(String::from(""));
    }

    let current_name = gs.character.name.clone();
    let current_id = gs.character.player_id;
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
        if let Some(entry) = entries.iter().find(|e| e.server == raw_server && !is_bad_target(&raw_server, class_str, &e.player1))
        {
            let lookup_gs = match session.send_command(ViewPlayer { ident: entry.player1.to_string() }).await
            {
                Ok(gs) => gs,
                Err(e) =>
                {
                    if e.to_string().to_ascii_lowercase().contains("player not found")
                    {
                        write_character_log(&current_name, current_id, &format!("DAILY_TASK: bad target {} ({}) on {} - player not found", entry.player1, class_str, raw_server));
                        mark_bad_target(&raw_server, class_str, &entry.player1);
                        return Ok(String::from("looked up player not found"));
                    }
                    return Err(e.into());
                }
            };
            let lookup = &lookup_gs.lookup;
            let player = lookup.lookup_name(&*entry.player1.to_string());
            let player_metadata = match player
            {
                None =>
                {
                    write_character_log(&current_name, current_id, &format!("DAILY_TASK: bad target {} ({}) on {} - player not found", entry.player1, class_str, raw_server));
                    mark_bad_target(&raw_server, class_str, &entry.player1);
                    return Ok(String::from("looked up player doesnt exist"));
                }
                Some(p) => p.clone(),
            };
            let player_class = player_metadata.class;
            let player_level = player_metadata.level;
            let player_name = player_metadata.name.clone();
            if (player_level > 20)
            {
            }

            if (player_class != class_type)
            {
                mark_bad_target(&raw_server, class_str, &entry.player1);
                return Ok(String::from("looked up player doesnt match"));
            }

            let gs = session.send_command(Command::CheckArena).await?;
            let free_fight_sanity_check = match gs.arena.next_free_fight
            {
                None => return Ok(String::from("")),
                Some(arena_fight_time) if arena_fight_time > Local::now() => return Ok(String::from("")),
                Some(arena_fight_time) => arena_fight_time,
            };
            if let Err(e) = session.send_command(Command::Fight { name: player_name.clone(), use_mushroom: false }).await
            {
                if e.to_string().to_ascii_lowercase().contains("player not found")
                {
                    write_character_log(&current_name, current_id, &format!("DAILY_TASK: bad target {} ({}) on {} - player not found", entry.player1, class_str, raw_server));
                    mark_bad_target(&raw_server, class_str, &entry.player1);
                    return Ok(String::from("fight player not found"));
                }
                return Err(e.into());
            }

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
    let response = reqwest::get("https://raw.githubusercontent.com/alexb231/snf-bot-eu/master/sfBot/src-tauri/charsToFight.json").await?;
    let class_map: HashMap<String, Vec<PlayerEntry>> = response.json().await?;

    {
        let cache_lock = get_cache();
        let mut cache = cache_lock.lock().unwrap();
        *cache = Some(class_map.clone());
        println!("[DEBUG] Caching class_map for future use.");
    }

    Ok(class_map)
}

pub async fn bare_handed_attack_task(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    use chrono::{Duration, Local};
    use sf_api::gamestate::items::{EquipmentSlot, ItemCommandIdent, ItemPlace};

    const WEAPON_SLOT_POS: usize = EquipmentSlot::Weapon as usize - 1;

    fn raw_item_move(from: ItemPlace, from_pos: usize, to: ItemPlace, to_pos: usize, ident: ItemCommandIdent) -> String { format!("PlayerItemMove:{}/{}/{}/{}/{}", from as usize, from_pos + 1, to as usize, to_pos + 1, ident) }

    let mut result = String::from(", Bare handed attack (");

    let gs = session.send_command(Command::CheckArena).await?.clone();

    let free_slots = gs.character.inventory.count_free_slots();
    let weapon_equipped = gs.character.equipment.0[EquipmentSlot::Weapon].is_some();
    if free_slots == 0 && weapon_equipped
    {
        return Ok(String::from(""));
    }

    let arena = &gs.arena;
    let total_amount_of_players = gs.hall_of_fames.players_total;
    let pages = total_amount_of_players / 51; 
    if pages < 2
    {
        return Ok(String::from(""));
    }
    let max_page_to_attack_lowest_enemy = pages - 2;

    let current_time = Local::now();
    let current_time_minus_2 = current_time - Duration::minutes(2);

    let free_fight = if let Some(next_free_fight) = arena.next_free_fight { current_time_minus_2 >= next_free_fight } else { false };

    if !free_fight
    {
        return Ok(String::from(""));
    }

    
    let mut moved_weapon_place: Option<ItemPlace> = None;
    let mut moved_weapon_pos: Option<usize> = None;
    let mut moved_weapon_ident: Option<ItemCommandIdent> = None;

    
    let mut dbg_free_backpack_pos: Option<usize> = None;
    let mut dbg_inv_type: Option<InventoryType> = None;
    let mut dbg_inv_pos: Option<usize> = None;
    let mut dbg_to_place: Option<ItemPlace> = None;
    let mut dbg_ident: Option<ItemCommandIdent> = None;

    
    if weapon_equipped
    {
        let Some(free) = gs.character.inventory.free_slot()
        else
        {
            return Ok(String::from(""));
        };

        
        let (inv_type, inv_pos) = free.inventory_pos();
        let to_place = inv_type.item_position(); 
                                                 

        let ident = gs.character.equipment.0[EquipmentSlot::Weapon].as_ref().unwrap().command_ident();

        
        dbg_free_backpack_pos = Some(free.backpack_pos());
        dbg_inv_type = Some(inv_type);
        dbg_inv_pos = Some(inv_pos);
        dbg_to_place = Some(to_place);
        dbg_ident = Some(ident);

        println!("[DEBUG] free_slot: backpack_pos={} inv_type={:?} inv_pos={} to_place={:?}", free.backpack_pos(), inv_type, inv_pos, to_place);
        println!("[DEBUG] weapon ident = {}", ident);

        let unequip_cmd = Command::ItemMove {
            from: sf_api::gamestate::items::ItemPosition {
                place: ItemPlace::Equipment,
                position: WEAPON_SLOT_POS,
            },
            to: sf_api::gamestate::items::ItemPosition {
                place: to_place,
                position: inv_pos,
            },
            item_ident: ident,
        };

        println!("[DEBUG] unequip raw: {}", raw_item_move(ItemPlace::Equipment, WEAPON_SLOT_POS, to_place, inv_pos, ident));

        moved_weapon_place = Some(to_place);
        moved_weapon_pos = Some(inv_pos);
        moved_weapon_ident = Some(ident);

        if let Err(e) = session.send_command(unequip_cmd).await
        {
            println!("[ERROR] Unequip failed: {}", e);

            if let (Some(from_place), Some(from_pos), Some(ident)) = (moved_weapon_place, moved_weapon_pos, moved_weapon_ident)
            {
                println!("[DEBUG] re-equip (on unequip error) raw: {}", raw_item_move(from_place, from_pos, ItemPlace::Equipment, WEAPON_SLOT_POS, ident));

                let _ = session
                    .send_command(Command::ItemMove {
                        from: sf_api::gamestate::items::ItemPosition {
                            place: from_place,
                            position: from_pos,
                        },
                        to: sf_api::gamestate::items::ItemPosition {
                            place: ItemPlace::Equipment,
                            position: WEAPON_SLOT_POS,
                        },
                        item_ident: ident,
                    })
                    .await;
            }

            return Err(e.into());
        }

        result.push_str("unequipped weapon - ");

        
        let after = match session.send_command(Command::Update).await
        {
            Ok(after) => after.clone(),
            Err(e) =>
            {
                println!("[ERROR] Update failed after unequip: {}", e);

                if let (Some(from_place), Some(from_pos), Some(ident)) = (moved_weapon_place, moved_weapon_pos, moved_weapon_ident)
                {
                    println!("[DEBUG] re-equip (on update error) raw: {}", raw_item_move(from_place, from_pos, ItemPlace::Equipment, WEAPON_SLOT_POS, ident));

                    let _ = session
                        .send_command(Command::ItemMove {
                            from: sf_api::gamestate::items::ItemPosition {
                                place: from_place,
                                position: from_pos,
                            },
                            to: sf_api::gamestate::items::ItemPosition {
                                place: ItemPlace::Equipment,
                                position: WEAPON_SLOT_POS,
                            },
                            item_ident: ident,
                        })
                        .await;
                }

                return Err(e.into());
            }
        };
        let weapon_now = after.character.equipment.0[EquipmentSlot::Weapon].is_some();
        println!("[DEBUG] after unequip: weapon slot occupied? {}", weapon_now);
    }

    
    let updated_gs = match session.send_command(Command::HallOfFamePage { page: max_page_to_attack_lowest_enemy as usize }).await
    {
        Ok(v) => v,
        Err(e) =>
        {
            println!("[ERROR] HallOfFamePage failed: {}", e);

            
            println!("[DEBUG] context: free.backpack_pos={:?} inv_type={:?} inv_pos={:?} to_place={:?} ident={:?}", dbg_free_backpack_pos, dbg_inv_type, dbg_inv_pos, dbg_to_place, dbg_ident);

            
            if let (Some(from_place), Some(from_pos), Some(ident)) = (moved_weapon_place, moved_weapon_pos, moved_weapon_ident)
            {
                println!("[DEBUG] re-equip (on HOF error) raw: {}", raw_item_move(from_place, from_pos, ItemPlace::Equipment, WEAPON_SLOT_POS, ident));

                let _ = session
                    .send_command(Command::ItemMove {
                        from: sf_api::gamestate::items::ItemPosition {
                            place: from_place,
                            position: from_pos,
                        },
                        to: sf_api::gamestate::items::ItemPosition {
                            place: ItemPlace::Equipment,
                            position: WEAPON_SLOT_POS,
                        },
                        item_ident: ident,
                    })
                    .await;
            }

            return Err(e.into());
        }
    };

    
    if let Some(player) = updated_gs.hall_of_fames.players.get(0)
    {
        let fight_player = Command::Fight { name: player.name.clone(), use_mushroom: false };
        result.push_str(&format!("fought opponent: {} - ", player.name));

        if let Err(e) = session.send_command(fight_player).await
        {
            println!("[ERROR] Fight failed: {}", e);

            if let (Some(from_place), Some(from_pos), Some(ident)) = (moved_weapon_place, moved_weapon_pos, moved_weapon_ident)
            {
                println!("[DEBUG] re-equip (on fight error) raw: {}", raw_item_move(from_place, from_pos, ItemPlace::Equipment, WEAPON_SLOT_POS, ident));

                let _ = session
                    .send_command(Command::ItemMove {
                        from: sf_api::gamestate::items::ItemPosition {
                            place: from_place,
                            position: from_pos,
                        },
                        to: sf_api::gamestate::items::ItemPosition {
                            place: ItemPlace::Equipment,
                            position: WEAPON_SLOT_POS,
                        },
                        item_ident: ident,
                    })
                    .await;
            }

            return Err(e.into());
        }

        
        if let (Some(from_place), Some(from_pos), Some(ident)) = (moved_weapon_place, moved_weapon_pos, moved_weapon_ident)
        {
            println!("[DEBUG] re-equip raw: {}", raw_item_move(from_place, from_pos, ItemPlace::Equipment, WEAPON_SLOT_POS, ident));

            let _ = session
                .send_command(Command::ItemMove {
                    from: sf_api::gamestate::items::ItemPosition {
                        place: from_place,
                        position: from_pos,
                    },
                    to: sf_api::gamestate::items::ItemPosition {
                        place: ItemPlace::Equipment,
                        position: WEAPON_SLOT_POS,
                    },
                    item_ident: ident,
                })
                .await;

            result.push_str("equipped weapon back on.)");
        }
        else
        {
            result.push_str("done.)");
        }

        return Ok(result);
    }

    
    if let (Some(from_place), Some(from_pos), Some(ident)) = (moved_weapon_place, moved_weapon_pos, moved_weapon_ident)
    {
        println!("[DEBUG] re-equip (no player) raw: {}", raw_item_move(from_place, from_pos, ItemPlace::Equipment, WEAPON_SLOT_POS, ident));

        let _ = session
            .send_command(Command::ItemMove {
                from: sf_api::gamestate::items::ItemPosition {
                    place: from_place,
                    position: from_pos,
                },
                to: sf_api::gamestate::items::ItemPosition {
                    place: ItemPlace::Equipment,
                    position: WEAPON_SLOT_POS,
                },
                item_ident: ident,
            })
            .await;
    }

    Ok(String::from(""))
}

pub fn should_do_daily_task(available_tasks: DailyTasks, task_type: TaskType) -> bool
{
    
    return available_tasks.get_available(task_type).is_some();
}

fn raw_item_move(from: ItemPlace, from_pos: usize, to: ItemPlace, to_pos: usize, item_ident: ItemCommandIdent) -> String { format!("PlayerItemMove:{}/{}/{}/{}/{}", from as usize, from_pos + 1, to as usize, to_pos + 1, item_ident) }

