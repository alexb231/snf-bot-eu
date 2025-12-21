#![allow(warnings)]

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    fs,
    fs::OpenOptions,
    hash::Hash,
    io::{Read, Write},
    path::Path,
    process,
    string::ToString,
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use chrono::{Local, NaiveTime};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty, Value};
use sf_api::{
    command::Command,
    error::SFError,
    gamestate::{
        character::Mount,
        fortress::{Fortress, FortressBuilding, FortressBuildingType, FortressUnitType},
        guild::GuildRank,
        items::{Item, ItemType},
        rewards::Event,
        social::{ClaimableMailType, ClaimableStatus},
        tavern::CurrentAction,
        underworld::UnderworldBuildingType,
        GameState,
    },
    SimpleSession,
};
use tokio::sync::Mutex;

use crate::{
    add_session_state,
    arena::arena_fight,
    arena_manager::play_idle_game,
    cache_character_settings,
    character_cache::{load_all_cached_characters, load_character_cache, save_character_cache, should_update_cache, update_character_active_status, CachedCharacter},
    paths::{get_character_settings_path, get_global_settings_path, get_user_config_path},
    city_guard::city_guard,
    clear_all_session_state,
    clear_all_session_state_server,
    clear_current_character_list,
    collect_daily_weekly_reward::collect_daily_and_weekly_rewards,
    daily_task_management::perform_daily_tasks,
    dungeon_management::fight_dungeon_with_highest_win_rate,
    // equipment_swapping::check_and_swap_equipment,
    expeditions_exp::play_expeditions_exp,
    expeditions_gold::play_expeditions_gold,
    fetch_character_setting,
    find_and_remove_single_session_state,
    fortress::{attack_fortress, build_fortress_our_order, collect_fortress_resources, start_searching_for_gem, train_fortress_units},
    generate_hash,
    get_current_character_obj,
    get_session_state,
    get_session_state_for_single,
    guild::{declare_guild_attack, fight_demon_portal, fight_guild_portal, fight_hydra, sign_up_for_guild_attack_and_defense},
    hellevator_management::play_hellevator,
    helperUtils::skipFunction,
    inventory_management::manage_inventory,
    lottery::play_dice,
    overwrite_session_state,
    perform_check_whether_user_is_allowed_to_start_bot,
    pet_management::{feed_all_pets, fight_pet_arena, fight_pet_dungeon, get_pets_left_for_pet_arena},
    process_ingame_mails::collect_gifts_from_mail,
    quarter::spin_lucky_wheel,
    scrapbook_filler::fill_scrapbook,
    set_current_character_obj,
    stable::buy_mount,
    stat_point_management::upgrade_skill_points,
    // toilet_management::use_toilet,
    underword_management::{build_underworld_perfect_order, collect_underworld_resources, level_up_uw_keeper, perform_underworld_atk_favourite_enemy, perform_underworld_atk_suggested_enemy},
    unlockables::accept_unlockables,
    witch_enchantment::{enchant_items, should_enchant},
    CurrentCharacterAccount,
    SessionState,
};
use crate::{
    equipment_swapping::check_and_swap_equipment,
    inventory_management::brew_potions_using_pet_fruits,
    toilet_management::use_toilet,
    webshop::claim_free_mushroom,
};

static FILE_LOCK: once_cell::sync::Lazy<Arc<Mutex<()>>> = once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(())));

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerConfig
{
    pub accounts: Vec<UserConfig>,
}

#[derive(Debug)]
pub struct BotError
{
    pub(crate) message: String,
}

impl fmt::Display for BotError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.message) }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UserConfig
{
    pub accname: String,
    pub password: String,
    pub single: bool,
    pub server: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CharacterSettings
{
    character_id: u32,
    character_name: String,
    settings: HashMap<String, Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GlobalSettings
{
    settings: HashMap<String, Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct CharacterDisplay
{
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
    pub isActive: bool,
    pub logMessage: String,
    pub mount: String,
    pub account: String,
}

pub async fn save_character_settings(charactername: &str, characterid: u32, settings: HashMap<String, Value>) -> Result<String, String>
{
    let file_path = get_character_settings_path();
    let mut file_content = String::new();
    let mut characters: Vec<CharacterSettings> = Vec::new();

    let _lock = FILE_LOCK.lock().await; // hier lock

    // 1. komplett neue liste für die settings
    if let Ok(mut file) = OpenOptions::new().read(true).open(&file_path)
    {
        file.read_to_string(&mut file_content).map_err(|e| e.to_string())?;

        if !file_content.trim().is_empty()
        {
            characters = from_str(&file_content).unwrap_or_else(|_| Vec::new());
        }
    } else {
        fs::write(&file_path, "[]").map_err(|e| e.to_string())?;
    }

    // 2. einzelne character
    let mut updated = false;
    for character in &mut characters
    {
        if character.character_id == characterid && character.character_name == charactername
        {
            character.settings = settings.clone();
            updated = true;
            break;
        }
    }

    if !updated
    {
        let new_character = CharacterSettings {
            character_id: characterid,
            character_name: charactername.to_string(),
            settings: settings.clone(),
        };
        characters.push(new_character);
    }

    let updated_content = to_string_pretty(&characters).map_err(|e| e.to_string())?;
    fs::write(&file_path, updated_content).map_err(|e| e.to_string())?;

    cache_character_settings();

    // Update character cache if settingCharacterActive was changed
    if let Some(is_active) = settings.get("settingCharacterActive") {
        if let Some(active_bool) = is_active.as_bool() {
            if let Err(e) = update_character_active_status(charactername, characterid, active_bool) {
                eprintln!("[CACHE] Failed to update active status in cache: {}", e);
            }
        }
    }

    // Return the saved settings as JSON so frontend can verify
    let response = serde_json::json!({
        "success": true,
        "message": format!("Settings saved successfully for {}, id {}.", charactername, characterid),
        "settings": settings
    });
    Ok(response.to_string())
}

pub async fn save_settings_for_all_characters(settings: HashMap<String, Value>) -> Result<String, String>
{
    if settings.is_empty() {
        let response = serde_json::json!({
            "success": true,
            "message": "No settings changes to apply.",
            "settings": settings,
            "count": 0
        });
        return Ok(response.to_string());
    }

    let mut targets: Vec<(String, u32)> = Vec::new();
    let mut seen: HashSet<(u32, String)> = HashSet::new();

    if let Ok(cached_characters) = load_all_cached_characters() {
        for character in cached_characters {
            let key = (character.id, character.name.to_lowercase());
            if seen.insert(key) {
                targets.push((character.name, character.id));
            }
        }
    }

    if let Ok(existing_settings) = load_all_character_settings() {
        for character in existing_settings {
            let key = (character.character_id, character.character_name.to_lowercase());
            if seen.insert(key) {
                targets.push((character.character_name, character.character_id));
            }
        }
    }

    if targets.is_empty() {
        return Err("No characters found to update settings".to_string());
    }

    let mut updated = 0usize;
    for (name, id) in targets {
        let mut merged = match load_character_settings(&name, id) {
            Ok(Some(existing)) => existing,
            Ok(None) => HashMap::new(),
            Err(_) => HashMap::new(),
        };
        for (key, value) in &settings {
            merged.insert(key.clone(), value.clone());
        }
        save_character_settings(&name, id, merged).await?;
        updated += 1;
    }

    let response = serde_json::json!({
        "success": true,
        "message": format!("Settings saved for {} character(s).", updated),
        "settings": settings,
        "count": updated
    });
    Ok(response.to_string())
}

pub fn load_character_settings(charactername: &str, characterid: u32) -> Result<Option<HashMap<String, Value>>, String>
{
    let file_path = get_character_settings_path();
    let mut file_content = String::new();
    let characters: Vec<CharacterSettings>;

    if let Ok(mut file) = OpenOptions::new().read(true).open(&file_path)
    {
        file.read_to_string(&mut file_content).map_err(|e| e.to_string())?;

        if !file_content.trim().is_empty()
        {
            characters = serde_json::from_str(&file_content).map_err(|e| e.to_string())?;
        } else {
            return Ok(None);
        }
    } else {
        return Err("File not found".to_string());
    }

    for character in &characters
    {
        if character.character_id == characterid && character.character_name == charactername
        {
            return Ok(Some(character.settings.clone()));
        }
    }

    Ok(None)
}

pub async fn build_character_display(session: &mut SimpleSession, server: &str, result: String, account_name: &str) -> CharacterDisplay
{
    // Extract server from session URL BEFORE send_command to avoid borrow issues
    let actual_server = if server.is_empty() {
        session.server_url()
            .host_str()
            .unwrap_or("unknown")
            .to_string()
    } else {
        server.to_lowercase()
    };

    let gamestate_result = session.send_command(Command::Update).await;

    match gamestate_result
    {
        Ok(gamestate) =>
            {
                let characterActive: bool = fetch_character_setting(&gamestate, "settingCharacterActive").unwrap_or(false);
                let mut character_name = gamestate.character.name.clone();

                let current_mount = match &gamestate.character.mount
                {
                    Some(mount) => match mount
                    {
                        Mount::Cow => "Cow".to_string(),
                        Mount::Horse => "Horse".to_string(),
                        Mount::Tiger => "Tiger".to_string(),
                        Mount::Dragon => "Dragon".to_string(),
                    },
                    None => "None".to_string(),
                };
                let guildname = match &gamestate.guild
                {
                    Some(guild) => &guild.name,
                    None => &"".to_string(),
                };

                let pet_fights = get_pets_left_for_pet_arena(&gamestate).len();
                if (server != "")
                {
                    character_name = character_name.to_lowercase();
                }

                let display = CharacterDisplay {
                    id: gamestate.character.player_id,
                    name: character_name.clone(),
                    lvl: gamestate.character.level,
                    guild: guildname.to_string(),
                    alu: gamestate.tavern.thirst_for_adventure_sec.clone() / 60,
                    beers: gamestate.tavern.beer_drunk,
                    mushrooms: gamestate.character.mushrooms.clone(),
                    hourglasses: gamestate.tavern.quicksand_glasses,
                    gold: gamestate.character.silver.clone() / 100,
                    luckycoins: gamestate.specials.wheel.lucky_coins,
                    fights: gamestate.arena.fights_for_xp,
                    luckyspins: gamestate.specials.wheel.spins_today,
                    petfights: pet_fights as u8,
                    dicerolls: gamestate.tavern.dice_game.remaining,
                    server: actual_server.clone(),
                    isActive: characterActive,
                    mount: current_mount.clone(),
                    logMessage: result.clone(),
                    account: account_name.to_string(),
                };

                // Save to character cache
                let cached = CachedCharacter {
                    id: display.id,
                    name: character_name,
                    lvl: display.lvl,
                    alu: display.alu,
                    guild: display.guild.clone(),
                    beers: display.beers,
                    mushrooms: display.mushrooms,
                    hourglasses: display.hourglasses,
                    gold: display.gold,
                    luckycoins: display.luckycoins,
                    fights: display.fights,
                    luckyspins: display.luckyspins,
                    petfights: display.petfights,
                    dicerolls: display.dicerolls,
                    server: actual_server.clone(),
                    is_active: characterActive,
                    mount: current_mount,
                    account: account_name.to_string(),
                    cached_at: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                };

                let existing_cache = load_character_cache(&display.name, &actual_server).unwrap_or(None);
                if should_update_cache(existing_cache.as_ref()) {
                    if let Err(e) = save_character_cache(&cached) {
                        eprintln!("[CACHE] Failed to save cache: {}", e);
                    }
                }

                display
            }
        Err(_) => CharacterDisplay {
            id: 0,
            name: "--None--".to_string(),
            lvl: 0,
            alu: 0,
            beers: 0,
            guild: "".to_string(),
            mushrooms: 0,
            hourglasses: 0,
            gold: 0,
            luckycoins: 0,
            fights: 0,
            luckyspins: 0,
            petfights: 0,
            dicerolls: 0,
            server: String::from(""),
            isActive: false,
            logMessage: String::from(""),
            mount: "None".to_string(),
            account: account_name.to_string(),
        },
    }
}

pub async fn singleAccountExecution(accountName: &str, pw: &str, charactername: &str, ziel: &str) -> Result<Vec<CharacterDisplay>, String>
{
    // account name ist der server!!!!!!!!!!!!
    let sessionState = get_session_state_for_single(String::from(charactername.to_lowercase()), &*accountName.to_lowercase());
    if sessionState.is_none()
    {
        return Ok(Vec::new());
    }
    let mut sessionState = sessionState.unwrap();

    // accountName is actually the server name for single accounts
    if sessionState.character_name == charactername && sessionState.server == accountName
    {
        let mut session = &mut sessionState.session; // mutable reference
        let gs = session.send_command(Command::Update).await;

        match gs
        {
            Ok(gamestate) =>
                {
                    let id = gamestate.character.player_id;
                    if (skipFunction(gamestate, ziel))
                    {
                        // println!("char: {} ->: {}", gamestate.character.name, ziel);
                        return Ok(Vec::new());
                    }

                    set_current_character_obj(String::from(accountName), String::from(charactername), 0, String::from(ziel));
                    let display = run_func(session, ziel, true, accountName).await.map_err(|e| e.to_string())?;
                    // accountName is actually the server name for single accounts
                    // Get the real account name from the session state before removing
                    let real_account_name = sessionState.account_name.clone();
                    find_and_remove_single_session_state(charactername.to_lowercase(), &*accountName.to_lowercase());
                    add_session_state(SessionState {
                        account_name: real_account_name,                            // actual account name
                        character_name: String::from(charactername.to_lowercase()), // charname
                        character_id: id,
                        server: accountName.to_string(),                            // server
                        session: session.clone(),
                    });
                    return Ok(display);
                }
            Err(SFError::ServerError(msg)) =>
                {
                    if msg == "sessionid invalid"
                    {
                        let timeout = Duration::from_secs(2); // TODO: aus den configs lesen
                        tokio::time::sleep(timeout).await;
                        return Err("Invalid Session".to_string());
                    } else {
                        return Err("ERROR1".to_string());
                    }
                }
            Err(msg) =>
                {
                    return Err(msg.to_string());
                }
        }
    }
    Ok(Vec::new())
}

pub async fn getFunctionNamesToExecute(id: u32, charname: &str) -> Result<Vec<String>, String>
{
    let commandsToExecute: Vec<String> = vec![
        "cmd_play_expeditions_gold".to_string(),
        "cmd_play_expeditions_exp".to_string(),
        "cmd_city_guard".to_string(),
        "cmd_upgrade_skill_points".to_string(),
        "cmd_collect_daily_and_weekly_rewards".to_string(),
        "cmd_play_dice".to_string(),
        "cmd_accept_unlockables".to_string(),
        "cmd_play_idle_game".to_string(),
        "cmd_collect_fortress_resources".to_string(),
        "cmd_use_toilet".to_string(),
        "cmd_manage_inventory".to_string(),
        "cmd_collect_free_mushroom".to_string(),
        "cmd_fight_demon_portal".to_string(),
        "cmd_fight_guild_portal".to_string(),
        "cmd_fight_dungeon_with_lowest_level".to_string(),
        "cmd_arena_fight".to_string(),
        "cmd_enchant_items".to_string(),
        "cmd_play_expeditions_gold".to_string(),
        "cmd_play_expeditions_exp".to_string(),
        "cmd_start_searching_for_gem".to_string(),
        "cmd_play_expeditions_gold".to_string(),
        "cmd_play_expeditions_exp".to_string(),
        "cmd_attack_fortress".to_string(),
        "cmd_train_fortress_units".to_string(),
        "cmd_perform_underworld_atk_suggested_enemy".to_string(),
        "cmd_collect_underworld_resources".to_string(),
        "cmd_build_underworld_perfect_order".to_string(),
        "cmd_fight_pet_arena".to_string(),
        "cmd_check_and_swap_equipment".to_string(),
        "cmd_perform_daily_tasks".to_string(),
        "cmd_buy_mount".to_string(),
        "cmd_play_expeditions_gold".to_string(),
        "cmd_play_expeditions_exp".to_string(),
        "cmd_spin_lucky_wheel".to_string(),
        "cmd_build_fortress_our_order".to_string(),
        "cmd_sign_up_for_guild_attack_and_defense".to_string(),
        "cmd_fight_hydra".to_string(),
        "cmd_feed_all_pets".to_string(),
        "cmd_collect_gifts_from_mail".to_string(),
        "cmd_play_dice".to_string(),
        "cmd_fight_pet_dungeon".to_string(),
        "cmd_city_guard".to_string(),
        "cmd_brew_potions_using_fruits".to_string(),
        "cmd_level_up_uw_keeper".to_string(),
        "cmd_play_hellevator".to_string(),
        "cmd_fill_scrapbook".to_string(),
        "cmd_complete".to_string(),
    ];

    let sessionState = get_session_state(String::from(charname), id);
    if sessionState.is_none()
    {
        return Ok(commandsToExecute);
    }

    let mut filteredCommandNamesToExecute: Vec<String> = Vec::new();
    let mut sessionState = sessionState.unwrap();
    let gs = sessionState.session.send_command(Command::Update).await;
    match gs
    {
        Ok(gamestate) =>
            {
                for x in commandsToExecute.iter()
                {
                    if (skipFunction(gamestate, x))
                    {
                        continue;
                    }
                    // cannot use hashset no idea what comes out on js side
                    if !filteredCommandNamesToExecute.contains(&x.to_string())
                    {
                        filteredCommandNamesToExecute.push(x.to_string());
                    }
                }
            }
        Err(SFError::ServerError(msg)) =>
            {
                return Ok(commandsToExecute);
            }
        _ =>
            {}
    }
    return Ok(filteredCommandNamesToExecute);
}

pub async fn startedenbot2(accname: &str, pw: &str, id: u32, charname: &str, ziel: &str) -> Result<Vec<CharacterDisplay>, String>
{
    let sessionState = get_session_state(String::from(charname), id);
    if sessionState.is_none()
    {
        return Ok(Vec::new());
    }
    let mut sessionState = sessionState.unwrap();

    if sessionState.character_name == charname && sessionState.character_id == id
    {
        let mut session = &mut sessionState.session; // mutable reference
        // Extract server from session URL
        let server = session.server_url()
            .host_str()
            .unwrap_or("unknown")
            .to_string();
        let gs = session.send_command(Command::Update).await;

        match gs
        {
            Ok(gamestate) =>
                {
                    // println!("character:{} ziel {}", gamestate.character.name,ziel);
                    if (skipFunction(gamestate, ziel))
                    {
                        return Ok(Vec::new());
                    }
                    set_current_character_obj(String::from(accname), String::from(charname), id, String::from(ziel));
                    let display = run_func(session, ziel, false, "").await.map_err(|e| e.to_string())?;
                    overwrite_session_state(accname.to_string(), charname.to_string(), id, session.clone());
                    add_session_state(SessionState {
                        account_name: accname.to_string(),
                        character_name: charname.to_string(),
                        character_id: id,
                        server: server.clone(),
                        session: session.clone(),
                    });
                    return Ok(display);
                }
            Err(SFError::ServerError(msg)) =>
                {
                    overwrite_session_state(accname.to_string(), charname.to_string(), id, session.clone());
                    add_session_state(SessionState {
                        account_name: accname.to_string(),
                        character_name: charname.to_string(),
                        character_id: id,
                        server: server.clone(),
                        session: session.clone(),
                    });
                    if msg == "sessionid invalid"
                    {
                        let timeout = Duration::from_secs(1); // TODO: aus den configs lesen
                        tokio::time::sleep(timeout).await;
                        return Err("Invalid Session".to_string());
                    } else {
                        return Err("ERROR1".to_string());
                    }
                }
            Err(msg) =>
                {
                    return Err(msg.to_string());
                }
        }
    }
    Ok(Vec::new())
}

pub async fn run_func(session: &mut SimpleSession, ziel: &str, isSingle: bool, server: &str) -> Result<Vec<CharacterDisplay>, Box<dyn std::error::Error>>
{
    let result = match ziel
    {
        "cmd_fight_guild_portal" => cmd_fight_guild_portal(session).await?,
        "cmd_fight_demon_portal" => cmd_fight_demon_portal(session).await?,
        "cmd_manage_inventory" => cmd_manage_inventory(session).await?, // TODO move stuff into sub functions to avoid logic duplication and copy paste errors
        "cmd_collect_free_mushroom" => cmd_collect_free_mushroom(session).await?,
        "cmd_play_idle_game" => cmd_play_idle_game(session).await?,
        "cmd_upgrade_skill_points" => cmd_upgrade_skill_points(session).await?,
        "cmd_collect_fortress_resources" => cmd_collect_fortress_resources(session).await?,
        "cmd_collect_daily_and_weekly_rewards" => cmd_collect_daily_and_weekly_rewards(session).await?,
        "cmd_use_toilet" => cmd_use_toilet(session).await?,
        "cmd_fight_dungeon_with_lowest_level" => cmd_fight_dungeon_with_lowest_level(session).await?,
        "cmd_arena_fight" => cmd_arena_fight(session).await?,
        "cmd_play_expeditions_exp" => cmd_play_expeditions_exp(session).await?,
        "cmd_start_searching_for_gem" => cmd_start_searching_for_gem(session).await?,
        "cmd_attack_fortress" => cmd_attack_fortress(session).await?,
        "cmd_train_fortress_units" => cmd_train_fortress_units(session).await?,
        "cmd_perform_underworld_atk_suggested_enemy" => cmd_perform_underworld_atk_suggested_enemy(session).await?,
        "cmd_collect_underworld_resources" => cmd_collect_underworld_resources(session).await?,
        "cmd_build_underworld_perfect_order" => cmd_build_underworld_perfect_order(session).await?,
        "cmd_fight_pet_arena" => cmd_fight_pet_arena(session).await?,
        "cmd_check_and_swap_equipment" => cmd_check_and_swap_equipment(session).await?,
        "cmd_accept_unlockables" => cmd_accept_unlockables(session).await?,
        "cmd_perform_daily_tasks" => cmd_perform_daily_tasks(session).await?,
        "cmd_buy_mount" => cmd_buy_mount(session).await?,
        "cmd_play_dice" => cmd_play_dice(session).await?,
        "cmd_spin_lucky_wheel" => cmd_spin_lucky_wheel(session).await?,
        "cmd_build_fortress_our_order" => cmd_build_fortress_our_order(session).await?,
        "cmd_sign_up_for_guild_attack_and_defense" => cmd_sign_up_for_guild_attack_and_defense(session).await?,
        "cmd_fight_hydra" => cmd_fight_hydra(session).await?,
        "cmd_feed_all_pets" => cmd_feed_all_pets(session).await?,
        "cmd_collect_gifts_from_mail" => cmd_collect_gifts_from_mail(session).await?,
        "cmd_fight_pet_dungeon" => cmd_fight_pet_dungeon(session).await?,
        "cmd_enchant_items" => cmd_enchant_items(session).await?,
        "cmd_play_expeditions_gold" => cmd_play_expeditions_gold(session).await?,
        "cmd_city_guard" => cmd_city_guard(session).await?,
        "cmd_brew_potions_using_fruits" => cmd_brew_potions_using_fruits(session).await?,
        "cmd_level_up_uw_keeper" => cmd_level_up_uw_keeper(session).await?,
        "cmd_play_hellevator" => cmd_play_hellevator(session).await?,
        "cmd_fill_scrapbook" => cmd_fill_scrapbook(session).await?,
        "cmd_complete" => cmd_complete(session).await?,
        _ => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid command"))), /* Return an error if no match
                                                                                                              * TODO Add gem insertion functions
                                                                                                              * TODO add companion equipment functions */
    };

    let mut displays: Vec<CharacterDisplay> = Vec::new();
    if (isSingle)
    {
        displays.push(build_character_display(session, server, result, server).await);
    } else {
        displays.push(build_character_display(session, "", result, "").await);
    }
    Ok(displays)
}

pub async fn reload_acc_from_session(sf_account_sessions: Vec<SimpleSession>)
{
    for char in sf_account_sessions
    {
        char.game_state().is_none();
    }
}

pub async fn cmd_play_idle_game(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result: String = play_idle_game(session).await?;

    if result.contains("Seat: 0")
        && result.contains("PopcornStand: 0")
        && result.contains("ParkingLot: 0")
        && result.contains("Drinks: 0")
        && result.contains("DeadlyTrap: 0")
        && result.contains("VIPSeat: 0")
        && result.contains("Snacks: 0")
        && result.contains("StrayingMonsters: 0")
        && result.contains("Toilet: 0")
    {
        println!("Play IDLE SKIPPED");
        result = String::new();
    }
    Ok(result)
}

pub async fn cmd_upgrade_skill_points(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::from("");
    result = upgrade_skill_points(session).await?;
    Ok(result)
}

pub async fn cmd_complete(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let isDoNothing: bool = fetch_character_setting(&gs, "miscDonothing").unwrap_or(false);
    if (isDoNothing)
    {
        for attempt in 1..=3
        {
            match session
                .send_command(Command::Custom {
                    cmd_name: "AdvertisementsCompleted".to_string(),
                    arguments: vec!["1".to_string()],
                })
                .await
            {
                Ok(res) =>
                    {
                        println!("  Attempt {attempt} succeeded. Lucky coins now: {:?}", res.specials.wheel.lucky_coins);
                    }
                Err(e) =>
                    {
                        eprintln!("  Attempt {attempt} failed: {e}");
                    }
            }

            if attempt < 3
            {
                tokio::time::sleep(std::time::Duration::from_millis(fastrand::u64(10..45))).await;
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(fastrand::u64(1_000..5_001))).await;
    }
    let mut result = String::from("");
    Ok(result)
}

pub async fn cmd_play_hellevator(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::from("");
    result = play_hellevator(session).await?;
    Ok(result)
}

pub async fn cmd_fill_scrapbook(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::from("");
    result = fill_scrapbook(session).await?;
    Ok(result)
}

pub async fn cmd_collect_fortress_resources(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let result = collect_fortress_resources(session).await?;
    Ok(result)
}
pub async fn cmd_collect_daily_and_weekly_rewards(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let result = collect_daily_and_weekly_rewards(session).await?;
    Ok(result)
}

pub async fn cmd_collect_free_mushroom(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    claim_free_mushroom(session)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn std::error::Error>
        })
}
pub async fn cmd_use_toilet(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let enable_toilet: bool = fetch_character_setting(&gs, "toiletEnableToilet").unwrap_or(false);
    let throw_epics: bool = fetch_character_setting(&gs, "toiletSacrificeEpics").unwrap_or(false);
    let throw_normal_items: bool = fetch_character_setting(&gs, "toiletSacrificeNormalItems").unwrap_or(false);
    let throw_gems_only: bool = fetch_character_setting(&gs, "toiletSacrificeGems").unwrap_or(false);
    let flush_when_full: bool = fetch_character_setting(&gs, "toiletFlushWhenFull").unwrap_or(false);
    let mut result = String::new();
    if (enable_toilet)
    {
        result = use_toilet(session, throw_epics, throw_normal_items, throw_gems_only, flush_when_full).await?;
    }
    Ok(result)
}
pub async fn cmd_manage_inventory(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result: String = String::from("");
    result = manage_inventory(session).await?;
    Ok(result)
}

pub async fn cmd_enchant_items(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result: String = String::from("");
    result = enchant_items(session).await?;
    Ok(result)
}

pub async fn cmd_fight_dungeon_with_lowest_level(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    crate::dungeon_management::fight_dungeon_with_highest_win_rate(session).await?;
    Ok(String::from(""))
}
pub async fn cmd_arena_fight(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = arena_fight(session).await?;
    Ok(result)
}
pub async fn cmd_play_expeditions_exp(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let name = &gs.character.name;
    let enable_expeditions: bool = fetch_character_setting(&gs, "tavernPlayExpeditions").unwrap_or(false);
    let play_exp_expeditions: String = fetch_character_setting(&gs, "tavernPlayExpExpedition").unwrap_or("".to_string());
    let skip_using_hourglasses: bool = fetch_character_setting(&gs, "tavernSkipWithHourglasses").unwrap_or(false);
    let beers_to_drink: i32 = std::cmp::min(fetch_character_setting(&gs, "tavernDrinkBeerAmount").unwrap_or(0), 12);
    let priorities_list: Vec<String> = fetch_character_setting(&gs, "expeditionRewardPrioList").unwrap_or_default();
    let play_expedtion_from_str: String = fetch_character_setting(&gs, "tavernPlayExpeditionFrom").unwrap_or("00:00".to_string());
    let play_expedtion_from_time = NaiveTime::parse_from_str(&play_expedtion_from_str, "%H:%M").unwrap();

    let is_in_range = Local::now().time() > play_expedtion_from_time;

    if (enable_expeditions && is_in_range && play_exp_expeditions == "tavernPlayExpExpeditionExp")
    {
        play_expeditions_exp(session, &*name, skip_using_hourglasses, beers_to_drink as u8, priorities_list).await?;
    }
    Ok("".to_string())
}

pub async fn cmd_city_guard(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::from("");
    result = city_guard(session).await?;
    Ok(result)
}

pub async fn cmd_start_searching_for_gem(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = start_searching_for_gem(session).await?;
    Ok(result)
}
pub async fn cmd_attack_fortress(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = attack_fortress(session).await?;
    Ok(result)
}
pub async fn cmd_train_fortress_units(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = train_fortress_units(session).await?;
    Ok(result)
}

pub async fn cmd_brew_potions_using_fruits(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = brew_potions_using_pet_fruits(session).await?;
    Ok(result)
}

pub async fn cmd_perform_underworld_atk_suggested_enemy(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let perform_underworld_attacks_suggested: bool = fetch_character_setting(&gs, "underworldPerformAttacks").unwrap_or(false);
    let perform_favourite_underworld_atk: bool = fetch_character_setting(&gs, "underWorldAttackFavouriteOpponent").unwrap_or(false);
    let mut result = String::new();
    if (perform_favourite_underworld_atk && !perform_underworld_attacks_suggested)
    {
        perform_underworld_atk_favourite_enemy(session).await?;
    }

    if (perform_underworld_attacks_suggested && !perform_favourite_underworld_atk)
    {
        result = perform_underworld_atk_suggested_enemy(session).await?;
    }
    Ok(result)
}
pub async fn cmd_collect_underworld_resources(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = collect_underworld_resources(session).await?;
    Ok(result)
}
pub async fn cmd_build_underworld_perfect_order(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = build_underworld_perfect_order(session).await?;
    Ok(result)
}
pub async fn cmd_fight_pet_arena(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = fight_pet_arena(session).await?;
    Ok(result)
}
pub async fn cmd_check_and_swap_equipment(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = check_and_swap_equipment(session).await?;
    Ok(result)
}
pub async fn cmd_accept_unlockables(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    accept_unlockables(session).await?;
    Ok(String::from(""))
}
pub async fn cmd_perform_daily_tasks(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let result = perform_daily_tasks(session).await?;
    Ok(result)
}
pub async fn cmd_buy_mount(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = buy_mount(session).await?;
    Ok(result)
}
pub async fn cmd_play_dice(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = play_dice(session).await?;
    Ok(result)
}
pub async fn cmd_spin_lucky_wheel(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    spin_lucky_wheel(session).await?;
    Ok(String::from(""))
}
pub async fn cmd_build_fortress_our_order(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut result = String::new();
    result = build_fortress_our_order(session).await?;
    Ok(result)
}

pub async fn cmd_sign_up_for_guild_attack_and_defense(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut res = "".to_string();
    res = sign_up_for_guild_attack_and_defense(session).await?;

    res = declare_guild_attack(session).await?;

    Ok(res)
}
pub async fn cmd_fight_hydra(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut res = "".to_string();
    res = fight_hydra(session).await?;
    Ok(res)
}

pub async fn cmd_fight_guild_portal(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut res = "".to_string();
    res = fight_guild_portal(session).await?;
    Ok(res)
}

pub async fn cmd_level_up_uw_keeper(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut res = "".to_string();
    res = level_up_uw_keeper(session).await?;
    Ok(res)
}

pub async fn cmd_fight_demon_portal(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut res = "".to_string();
    res = fight_demon_portal(session).await?;
    Ok(res)
}

pub async fn cmd_feed_all_pets(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let do_feed_pets: bool = fetch_character_setting(&gs, "petsDoFeed").unwrap_or(false);
    let pet_feed_mode: String = fetch_character_setting(&gs, "petsFeedMode").unwrap_or("".to_string());
    let amount_of_pets_to_feed_per_day: i32 = fetch_character_setting(&gs, "petsToFeedPerDay").unwrap_or(0);
    let expensive_route = pet_feed_mode == "feedPetsExpensive";
    feed_all_pets(session, do_feed_pets, expensive_route, amount_of_pets_to_feed_per_day as usize).await?;
    Ok("".to_string())
}

pub async fn cmd_collect_gifts_from_mail(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut collected_info = String::from("");
    collected_info += &collect_gifts_from_mail(session).await?;
    Ok(collected_info)
}
pub async fn cmd_fight_pet_dungeon(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut res = "".to_string();
    res = fight_pet_dungeon(session).await?;
    Ok(res)
}

pub async fn cmd_play_expeditions_gold(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let name = &gs.character.name;
    let char_id = gs.character.player_id;

    let enable_expeditions: bool = fetch_character_setting(&gs, "tavernPlayExpeditions").unwrap_or(false);
    let play_gold_expeditions: String = fetch_character_setting(&gs, "tavernPlayExpExpedition").unwrap_or("".to_string());
    let skip_using_hourglasses: bool = fetch_character_setting(&gs, "tavernSkipWithHourglasses").unwrap_or(false);
    let beers_to_drink: i32 = std::cmp::min(fetch_character_setting(&gs, "tavernDrinkBeerAmount").unwrap_or(0), 12);
    let priorities_list: Vec<String> = fetch_character_setting(&gs, "expeditionRewardPrioList").unwrap_or_default();
    let play_expedtion_from_str: String = fetch_character_setting(&gs, "tavernPlayExpeditionFrom").unwrap_or("00:00".to_string());
    let play_expedtion_from_time = NaiveTime::parse_from_str(&play_expedtion_from_str, "%H:%M").unwrap();

    let is_in_range = Local::now().time() > play_expedtion_from_time;

    // Debug logging (console only)
    println!(
        "EXPEDITION_GOLD_DEBUG: enabled={}, play_gold_exp='{}', is_in_range={}, time_from={}, current_time={}, mount={:?}, thirst={}, beers={}/{}, current_action={:?}",
        enable_expeditions,
        play_gold_expeditions,
        is_in_range,
        play_expedtion_from_str,
        Local::now().time().format("%H:%M"),
        gs.character.mount,
        gs.tavern.thirst_for_adventure_sec,
        gs.tavern.beer_drunk,
        gs.tavern.beer_max,
        gs.tavern.current_action
    );

    if (enable_expeditions && is_in_range && play_gold_expeditions == "tavernPlayExpExpeditionGold")
    {
        play_expeditions_gold(session, &*name, skip_using_hourglasses, beers_to_drink as u8, priorities_list).await?;
    } else {
        crate::bot_runner::write_character_log(name, char_id, &format!(
            "EXPEDITION_GOLD: SKIPPED - enabled:{} && in_range:{} && play_gold:'{}'==tavernPlayExpExpeditionGold:{}",
            enable_expeditions, is_in_range, play_gold_expeditions, play_gold_expeditions == "tavernPlayExpExpeditionGold"
        ));
    }

    Ok("".to_string())
}

pub async fn save_global_settings(settings: HashMap<String, Value>) -> Result<String, String>
{
    let file_path = get_global_settings_path();
    let global_settings = GlobalSettings { settings };
    let updated_content = serde_json::to_string_pretty(&global_settings).map_err(|e| e.to_string())?;
    fs::write(&file_path, updated_content).map_err(|e| e.to_string())?;
    Ok("Global settings saved successfully.".to_string())
}

pub async fn get_global_settings() -> Result<HashMap<String, Value>, String>
{
    let file_path = get_global_settings_path();
    let mut file_content = String::new();
    if let Ok(mut file) = OpenOptions::new().read(true).open(&file_path)
    {
        file.read_to_string(&mut file_content).map_err(|e| e.to_string())?;

        if !file_content.trim().is_empty()
        {
            let global_settings: GlobalSettings = serde_json::from_str(&file_content).map_err(|e| e.to_string())?;
            return Ok(global_settings.settings);
        }
    }
    Ok(HashMap::new())
}

pub fn get_u64_setting(settings: &HashMap<String, Value>, key: &str, default: u64) -> u64
{
    settings
        .get(key)
        .and_then(|v| v.as_u64()) // try converting if no settings available yet return default
        .unwrap_or(default)
}

pub fn load_all_character_settings() -> Result<Vec<CharacterSettings>, String>
{
    let file_path = get_character_settings_path();
    let mut file_content = String::new();
    let characters: Vec<CharacterSettings>;

    if let Ok(mut file) = OpenOptions::new().read(true).open(&file_path)
    {
        file.read_to_string(&mut file_content).map_err(|e| e.to_string())?;

        if !file_content.trim().is_empty()
        {
            characters = serde_json::from_str(&file_content).map_err(|e| e.to_string())?;
        } else {
            return Ok(Vec::new());
        }
    } else {
        return Err("File not found".to_string());
    }
    Ok(characters)
}

pub async fn debug_method(session: &mut SimpleSession) -> Result<(), Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let name = &gs.character.name;
    // check_and_swap_equipment(session).await?;
    Ok(())
}

pub async fn login_single_account(name: &str, pw: &str, single: bool, server: &str) -> Result<Vec<CharacterDisplay>, String>
{
    let mut session = match SimpleSession::login(name, pw, server).await
    {
        Ok(sessions) => sessions,
        Err(err) => return Err(format!("Failed to log in: {}", err)),
    };

    let mut gamestateDisplay = Vec::<CharacterDisplay>::new();
    clear_all_session_state_server(String::from(name), server);

    let gamestate = match session.send_command(Command::Update).await
    {
        Ok(state) => state.clone(),
        Err(err) => return Err(format!("Failed to update session: {}", err)),
    };

    let display = build_character_display(&mut session, server, String::from(""), name).await;
    gamestateDisplay.push(display);

    let session_state = SessionState {
        account_name: String::from(name).to_lowercase(),                 // der account name (username)
        character_id: gamestate.character.player_id.clone(),             // kann bleiben
        character_name: gamestate.character.name.clone().to_lowercase(), // name von dem character
        server: String::from(server).to_lowercase(),                     // der server (nur bei single accounts)
        session,
    };

    add_session_state(session_state);
    cache_character_settings();
    save_user_conf(String::from(name).to_lowercase(), String::from(pw), single, String::from(server).to_lowercase());
    Ok(gamestateDisplay)
}

pub async fn login(name: &str, pw: &str) -> Result<Vec<CharacterDisplay>, String>
{
    let sessions = match SimpleSession::login_sf_account(name, pw).await
    {
        Ok(sessions) => sessions,
        Err(err) => return Err(format!("Failed to log in: {}", err)),
    };

    let mut gamestateDisplays = Vec::<CharacterDisplay>::new();
    clear_all_session_state(String::from(name));
    for mut session in sessions
    {
        // delete_at
        let gamestate = match session.send_command(Command::Update).await
        {
            Ok(state) => state.clone(),
            Err(err) => continue, // return Err(format!("Failed to update session: {}", err)),
        };

        let display = build_character_display(&mut session, "", String::from(""), name).await;
        gamestateDisplays.push(display);

        let session_state = SessionState {
            account_name: String::from(name),
            character_id: gamestate.character.player_id.clone(),
            character_name: gamestate.character.name.clone(),
            server: String::from(name),
            session,
        };

        add_session_state(session_state);
    }

    cache_character_settings();
    save_user_conf(String::from(name), String::from(pw), false, String::from(""));
    Ok(gamestateDisplays)
}

pub fn save_settings(charname: String, enabled: bool) -> Result<(), String> { Ok(()) }

pub fn read_user_conf() -> Result<Vec<UserConfig>, String>
{
    let file_path = get_user_config_path();

    if file_path.exists()
    {
        let content = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
        let player_config: PlayerConfig = serde_json::from_str(&content).map_err(|err| err.to_string())?;
        Ok(player_config.accounts)
    } else {
        Ok(Vec::new())
    }
}

pub fn save_user_conf(accname: String, password: String, single: bool, server: String) -> Result<(), String>
{
    let file_path = get_user_config_path();

    let mut player_config = if let Ok(mut file) = fs::File::open(&file_path)
    {
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|err| err.to_string())?;
        serde_json::from_str::<PlayerConfig>(&contents).unwrap_or(PlayerConfig { accounts: Vec::new() })
    } else {
        PlayerConfig { accounts: Vec::new() }
    };

    if player_config.accounts.iter().any(|user| user.accname == accname && user.server == server)
    {
        return Err(format!("Account with name '{}' already exists.", accname));
    }

    let new_user_config = UserConfig { accname, password, single, server };
    player_config.accounts.push(new_user_config);

    let updated_contents = serde_json::to_string_pretty(&player_config).map_err(|err| err.to_string())?;
    fs::write(&file_path, updated_contents).map_err(|err| err.to_string())?;

    Ok(())
}

pub fn kill() { process::exit(0); }

pub fn get_app_version() -> String
{
    //
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn check_time_in_range(from: String, to: String) -> bool
{
    let from_time = NaiveTime::parse_from_str(&from, "%H:%M").unwrap();
    let to_time = NaiveTime::parse_from_str(&to, "%H:%M").unwrap();
    let now = Local::now().time();

    if from_time == to_time
    {
        return true;
    }

    if from_time <= to_time
    {
        now >= from_time && now <= to_time
    } else {
        now >= from_time || now <= to_time
    }
}

pub async fn get_current_character() -> Result<CurrentCharacterAccount, String>
{
    let char = get_current_character_obj();
    Ok(char)
}

pub fn write_name_of_failed_func(fnName: String, identifier: String) -> Result<(), String>
{
    return Ok(());
    let file_path = crate::paths::exe_relative_path("failedFuncs.txt");

    let mut file = OpenOptions::new().append(true).create(true).open(&file_path).map_err(|err| err.to_string())?;

    let now = Local::now();
    let datetime_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let content = format!("failed: {} [{}] Identifier: {}\n", fnName, datetime_str, identifier);

    file.write_all(content.as_bytes()).map_err(|err| err.to_string())?;

    Ok(())
}

pub fn debug_log(message: String) -> Result<(), String>
{
    println!("{}", message);
    Ok(())
}

pub async fn log_to_file(file_name: &str, text: &str) -> Result<(), Box<dyn Error>>
{
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let log_line = format!("[{}] {}\n", timestamp, text);

    let mut file = OpenOptions::new().append(true).create(true).open(file_name)?;
    file.write_all(log_line.as_bytes())?;
    Ok(())
}

pub fn init() { cache_character_settings(); }

pub fn clear_current_characters() { clear_current_character_list(); }

pub async fn perform_check_whether_user_is_allowed_to_start_bot_impl() -> Result<bool, bool>
{
    let result = perform_check_whether_user_is_allowed_to_start_bot().await;

    match result
    {
        Ok(true) => Ok(true),
        Ok(false) => Ok(false),
        Err(_) => Err(false),
    }
}
pub fn generate_hash_impl() -> String
{
    //
    return generate_hash();
}
