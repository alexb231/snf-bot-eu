//! Bot Runner - Main orchestration loop
//!
//! This module replaces the JavaScript main loop from main.js.
//! It handles:
//! - Running the bot for all configured accounts
//! - Character cycling (round-robin)
//! - Command execution with cooldowns
//! - Session management and error recovery

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Local};
use sf_api::{command::Command, error::SFError, gamestate::character::Mount, gamestate::tavern::CurrentAction, SimpleSession};
use serde_json::Value;
use tokio::{
    sync::{broadcast, RwLock},
    task::JoinHandle,
};

use crate::{
    add_session_state,
    api::{BotStatusResponse, CharacterStatusInfo, CurrentCharacterInfo},
    character_cache::{get_character_identity, load_character_cache, save_character_cache, should_update_cache, CachedCharacter, CharacterIdentity},
    clear_all_session_state, fetch_character_setting, fetch_character_setting_by_identity, get_all_session_states, get_character_settings, get_session_state,
    helperUtils::skipFunction,
    overwrite_session_state,
    paths::exe_relative_path,
    set_current_character_obj,
    utils::{build_character_display, check_time_in_range, run_func, save_character_settings, CharacterDisplay},
    SessionState,
};

/// Account info for starting the bot
#[derive(Clone, Debug)]
pub struct AccountInfo
{
    pub accname: String,
    pub password: String,
    pub single: bool,
    pub server: String,
}

/// Character info from login
#[derive(Clone, Debug)]
pub struct CharacterInfo
{
    pub id: u32,
    pub name: String,
}

/// Maximum number of log lines to keep per character
const MAX_LOG_LINES: usize = 10000;

/// Get the log directory path (relative to EXE)
fn log_dir() -> std::path::PathBuf { exe_relative_path("logs") }

/// Write a log entry to the character-specific log file
/// Keeps only the last MAX_LOG_LINES entries
pub fn write_character_log(character_name: &str, character_id: u32, message: &str)
{
    let log_path = log_dir();

    // Ensure logs directory exists
    if let Err(e) = fs::create_dir_all(&log_path)
    {
        eprintln!("Failed to create logs directory: {}", e);
        return;
    }

    let filename = log_path.join(format!("{}_{}.log", character_name, character_id));
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_line = format!("[{}] {}", timestamp, message);

    // Read existing lines
    let mut lines: Vec<String> = if let Ok(content) = fs::read_to_string(&filename) { content.lines().map(|s| s.to_string()).collect() } else { Vec::new() };

    // Add new line
    lines.push(log_line);

    // Keep only last MAX_LOG_LINES
    if lines.len() > MAX_LOG_LINES
    {
        lines = lines.split_off(lines.len() - MAX_LOG_LINES);
    }

    // Write back
    let content = lines.join("\n") + "\n";
    if let Err(e) = fs::write(&filename, content)
    {
        eprintln!("Failed to write log file: {}", e);
    }
}

/// Read the log file for a character
pub fn read_character_log(character_name: &str, character_id: u32) -> Result<String, String>
{
    let filename = log_dir().join(format!("{}_{}.log", character_name, character_id));
    fs::read_to_string(&filename).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound
        {
            "Keine Logs vorhanden".to_string()
        }
        else
        {
            format!("Fehler beim Lesen: {}", e)
        }
    })
}

/// Commands to execute in order (from JS getFunctionNamesToExecute)
const COMMANDS_TO_EXECUTE: &[&str] = &[
    "cmd_play_expeditions_gold",
    "cmd_play_expeditions_exp",
    "cmd_city_guard",
    "cmd_upgrade_skill_points",
    "cmd_collect_daily_and_weekly_rewards",
    "cmd_play_dice",
    "cmd_accept_unlockables",
    "cmd_play_idle_game",
    "cmd_collect_fortress_resources",
    "cmd_use_toilet",
    "cmd_manage_inventory",
    "cmd_fight_demon_portal",
    "cmd_fight_guild_portal",
    "cmd_fight_dungeon_with_lowest_level",
    "cmd_arena_fight",
    "cmd_enchant_items",
    "cmd_play_expeditions_gold",
    "cmd_play_expeditions_exp",
    "cmd_start_searching_for_gem",
    "cmd_play_expeditions_gold",
    "cmd_play_expeditions_exp",
    "cmd_attack_fortress",
    "cmd_train_fortress_units",
    "cmd_perform_underworld_atk_suggested_enemy",
    "cmd_collect_underworld_resources",
    "cmd_build_underworld_perfect_order",
    "cmd_fight_pet_arena",
    "cmd_check_and_swap_equipment",
    "cmd_perform_daily_tasks",
    "cmd_buy_mount",
    "cmd_play_expeditions_gold",
    "cmd_play_expeditions_exp",
    "cmd_spin_lucky_wheel",
    "cmd_build_fortress_our_order",
    "cmd_sign_up_for_guild_attack_and_defense",
    "cmd_fight_hydra",
    "cmd_feed_all_pets",
    "cmd_collect_gifts_from_mail",
    "cmd_play_dice",
    "cmd_fight_pet_dungeon",
    "cmd_city_guard",
    "cmd_brew_potions_using_fruits",
    "cmd_level_up_uw_keeper",
    "cmd_play_hellevator",
    "cmd_fill_scrapbook",
    "cmd_complete",
];

/// Cooldown definitions (command -> milliseconds)
fn get_cooldowns() -> HashMap<&'static str, u64>
{
    let mut cooldowns = HashMap::new();
    cooldowns.insert("cmd_play_expeditions_gold", 15_000);
    cooldowns.insert("cmd_play_expeditions_exp", 15_000);
    cooldowns.insert("cmd_city_guard", 60_000);
    cooldowns.insert("cmd_upgrade_skill_points", 15 * 60_000);
    cooldowns.insert("cmd_collect_daily_and_weekly_rewards", 60 * 60_000);
    cooldowns.insert("cmd_play_dice", 30_000);
    cooldowns.insert("cmd_accept_unlockables", 5 * 60_000);
    cooldowns.insert("cmd_play_idle_game", 60_000);
    cooldowns.insert("cmd_collect_fortress_resources", 30 * 60_000);
    cooldowns.insert("cmd_use_toilet", 10 * 60_000);
    cooldowns.insert("cmd_manage_inventory", 5 * 60_000);
    cooldowns.insert("cmd_fight_demon_portal", 60_000);
    cooldowns.insert("cmd_fight_guild_portal", 60_000);
    cooldowns.insert("cmd_fight_dungeon_with_lowest_level", 60_000);
    cooldowns.insert("cmd_arena_fight", 3 * 60_000);
    cooldowns.insert("cmd_enchant_items", 10 * 60_000);
    cooldowns.insert("cmd_start_searching_for_gem", 5 * 60_000);
    cooldowns.insert("cmd_attack_fortress", 60_000);
    cooldowns.insert("cmd_train_fortress_units", 5 * 60_000);
    cooldowns.insert("cmd_perform_underworld_atk_suggested_enemy", 60_000);
    cooldowns.insert("cmd_collect_underworld_resources", 30 * 60_000);
    cooldowns.insert("cmd_build_underworld_perfect_order", 5 * 60_000);
    cooldowns.insert("cmd_fight_pet_arena", 60_000);
    cooldowns.insert("cmd_check_and_swap_equipment", 10 * 60_000);
    cooldowns.insert("cmd_perform_daily_tasks", 60 * 60_000);
    cooldowns.insert("cmd_buy_mount", 60 * 60_000);
    cooldowns.insert("cmd_spin_lucky_wheel", 60_000);
    cooldowns.insert("cmd_build_fortress_our_order", 5 * 60_000);
    cooldowns.insert("cmd_sign_up_for_guild_attack_and_defense", 60 * 60_000);
    cooldowns.insert("cmd_fight_hydra", 60_000);
    cooldowns.insert("cmd_feed_all_pets", 60 * 60_000);
    cooldowns.insert("cmd_collect_gifts_from_mail", 5 * 60_000);
    cooldowns.insert("cmd_fight_pet_dungeon", 60_000);
    cooldowns.insert("cmd_brew_potions_using_fruits", 60 * 60_000);
    cooldowns.insert("cmd_level_up_uw_keeper", 5 * 60_000);
    cooldowns.insert("cmd_play_hellevator", 60_000);
    cooldowns.insert("cmd_fill_scrapbook", 10 * 60_000);
    // cmd_complete has no cooldown
    cooldowns
}

/// Bot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotState
{
    Stopped,
    Running,
    Paused,
}

/// Blacklist entry for a character
#[derive(Clone)]
struct BlacklistEntry
{
    expiry: DateTime<Local>,
}

/// Bot runner that manages the main execution loop
pub struct BotRunner
{
    state: BotState,
    accounts: Vec<AccountInfo>,
    characters: Vec<CharacterDisplay>,
    current_character: Option<CurrentCharacterInfo>,
    blacklist: HashMap<String, BlacklistEntry>,
    cooldowns: HashMap<String, DateTime<Local>>, // key: "{char_id}_{command}"
    task_handles: Vec<JoinHandle<()>>,
    stop_signal: Option<broadcast::Sender<()>>,
}

impl BotRunner
{
    pub fn new() -> Self
    {
        Self {
            state: BotState::Stopped,
            accounts: Vec::new(),
            characters: Vec::new(),
            current_character: None,
            blacklist: HashMap::new(),
            cooldowns: HashMap::new(),
            task_handles: Vec::new(),
            stop_signal: None,
        }
    }

    /// Start the bot with the given accounts
    pub async fn start(&mut self, accounts: Vec<AccountInfo>) -> Result<(), String>
    {
        if self.state == BotState::Running
        {
            return Err("Bot is already running".to_string());
        }

        self.accounts = accounts.clone();
        self.state = BotState::Running;

        // Create stop signal channel
        let (stop_tx, _) = broadcast::channel(1);
        self.stop_signal = Some(stop_tx.clone());

        // Spawn a task for each account
        for account in accounts
        {
            let stop_rx = stop_tx.subscribe();
            let cooldowns = Arc::new(RwLock::new(self.cooldowns.clone()));
            let blacklist = Arc::new(RwLock::new(self.blacklist.clone()));

            let handle = tokio::spawn(async move {
                // Login the account first, then run the loop
                match login_and_run_account(account, stop_rx, cooldowns, blacklist).await
                {
                    Ok(_) => println!("Account loop finished"),
                    Err(e) => println!("Account loop error: {}", e),
                }
            });

            self.task_handles.push(handle);
        }

        Ok(())
    }

    /// Stop the bot
    pub async fn stop(&mut self)
    {
        if let Some(stop_tx) = self.stop_signal.take()
        {
            let _ = stop_tx.send(());
        }

        // Wait for all tasks to finish
        for handle in self.task_handles.drain(..)
        {
            let _ = handle.await;
        }

        self.state = BotState::Stopped;
        self.current_character = None;
    }

    /// Pause the bot
    pub fn pause(&mut self)
    {
        if self.state == BotState::Running
        {
            self.state = BotState::Paused;
        }
    }

    /// Resume the bot
    pub fn resume(&mut self)
    {
        if self.state == BotState::Paused
        {
            self.state = BotState::Running;
        }
    }

    /// Get current status
    pub fn get_status(&self) -> BotStatusResponse
    {
        // Get current character from global state
        let current_char = crate::get_current_character_obj();
        let current_character = current_char.list.first().map(|c| CurrentCharacterInfo {
            account: c.accname.clone(),
            name: c.name.clone(),
            id: c.id,
            current_action: c.ziel.clone(),
        });

        // Get all characters from session states
        let session_states = crate::get_all_session_states();
        let mut characters: Vec<CharacterStatusInfo> = Vec::new();

    for ss in session_states
    {
        // Try to get character info from the session
        if let Some(gs) = ss.session.game_state()
        {
            // Check if character is active in settings
            let is_active: bool = crate::fetch_character_setting(gs, "settingCharacterActive").unwrap_or(false);
            let guild_name = gs.guild.as_ref().map(|g| g.name.clone()).unwrap_or_default();
            let petfights = crate::pet_management::get_pets_left_for_pet_arena(gs).len() as u8;
            let current_action = format_current_action(&gs.tavern.current_action);

            characters.push(CharacterStatusInfo {
                id: gs.character.player_id,
                name: gs.character.name.clone(),
                server: ss.server.clone(),
                account: ss.account_name.clone(),
                lvl: gs.character.level as u32,
                guild: guild_name,
                gold: gs.character.silver,
                mushrooms: gs.character.mushrooms as i32,
                beers: gs.tavern.beer_drunk as u32,
                fights: gs.arena.fights_for_xp as u32,
                alu: gs.tavern.thirst_for_adventure_sec,
                petfights,
                dicerolls: gs.tavern.dice_game.remaining,
                current_action,
                is_active,
            });
        }
    }

        BotStatusResponse {
            running: self.state == BotState::Running,
            paused: self.state == BotState::Paused,
            accounts: self.accounts.iter().map(|a| a.accname.clone()).collect(),
            current_character,
            characters,
        }
    }

    /// Get logged-in characters
    pub fn get_characters(&self) -> Vec<CharacterDisplay> { self.characters.clone() }

    /// Update character list
    pub fn update_characters(&mut self, characters: Vec<CharacterDisplay>) { self.characters = characters; }

    /// Set current character being processed
    pub fn set_current_character(&mut self, info: CurrentCharacterInfo) { self.current_character = Some(info); }
}

fn format_current_action(action: &CurrentAction) -> String
{
    match action
    {
        CurrentAction::Idle => "Idle".to_string(),
        CurrentAction::CityGuard { .. } => "City Guard".to_string(),
        CurrentAction::Quest { .. } => "Quest".to_string(),
        CurrentAction::Expedition => "Expedition".to_string(),
        CurrentAction::Unknown(_) => "Unknown".to_string(),
    }
}

impl Default for BotRunner
{
    fn default() -> Self { Self::new() }
}

/// Login to an account and get its characters
async fn login_account(account: &AccountInfo) -> Result<Vec<CharacterInfo>, String>
{
    println!("[{}] Logging in...", account.accname);

    if account.single
    {
        // Single server login - check cache first
        // For single accounts, the username is the character name
        let char_name = account.accname.to_lowercase();
        let server = account.server.to_lowercase();

        // Check if character is inactive in cache - if so, skip entirely
        let existing_cache = load_character_cache(&char_name, &server).unwrap_or(None);
        if let Some(cached) = existing_cache.as_ref()
        {
            if !cached.is_active
            {
                println!("[{}] Skipping inactive single account: {} on {}", account.accname, char_name, server);
                return Ok(vec![]); // Return empty - character is inactive
            }
        }

        // Character is active or not in cache - proceed with login
        let mut session = SimpleSession::login(&account.accname, &account.password, &account.server).await.map_err(|e| format!("Login failed: {}", e))?;

        // Get game state to get character info
        let gs = session.send_command(Command::Update).await.map_err(|e| format!("Failed to get game state: {}", e))?;

        // If no settings exist yet, create a default inactive entry so future runs can skip
        if fetch_character_setting::<bool>(&gs, "settingCharacterActive").is_none()
        {
            let mut defaults = HashMap::new();
            defaults.insert("settingCharacterActive".to_string(), Value::Bool(false));
            if let Err(e) = save_character_settings(&gs.character.name, gs.character.player_id, defaults).await
            {
                eprintln!("[SETTINGS] Failed to create default settings for {}: {}", gs.character.name, e);
            }
        }

        let char_info = CharacterInfo {
            id: gs.character.player_id,
            name: gs.character.name.clone(),
        };

        // Save to character cache
        // Default to inactive if setting is missing so later runs can skip unless explicitly activated
        let is_active: bool = fetch_character_setting(&gs, "settingCharacterActive").unwrap_or(false);
        let mount_str = match &gs.character.mount
        {
            Some(Mount::Cow) => "Cow",
            Some(Mount::Horse) => "Horse",
            Some(Mount::Tiger) => "Tiger",
            Some(Mount::Dragon) => "Dragon",
            None => "None",
        };
        let guild_name = gs.guild.as_ref().map(|g| g.name.clone()).unwrap_or_default();

        let cached = CachedCharacter {
            id: gs.character.player_id,
            name: gs.character.name.clone().to_lowercase(),
            lvl: gs.character.level,
            alu: gs.tavern.thirst_for_adventure_sec / 60,
            guild: guild_name,
            beers: gs.tavern.beer_drunk,
            mushrooms: gs.character.mushrooms,
            hourglasses: gs.tavern.quicksand_glasses,
            gold: gs.character.silver / 100,
            luckycoins: gs.specials.wheel.lucky_coins,
            fights: gs.arena.fights_for_xp,
            luckyspins: gs.specials.wheel.spins_today,
            petfights: 0, // Would need pet calculation
            dicerolls: gs.tavern.dice_game.remaining,
            server: account.server.clone(),
            is_active,
            mount: mount_str.to_string(),
            account: account.accname.clone(),
            cached_at: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        };

        if should_update_cache(existing_cache.as_ref())
        {
            if let Err(e) = save_character_cache(&cached)
            {
                eprintln!("[CACHE] Failed to save cache for {}: {}", gs.character.name, e);
            }
        }

        // Store session - for single accounts, clear by server to avoid duplicates
        crate::clear_session_state_by_server(&account.server);
        add_session_state(SessionState {
            account_name: account.accname.clone(),
            character_name: gs.character.name.clone().to_lowercase(),
            character_id: gs.character.player_id,
            server: account.server.clone(),
            session,
        });

        println!("[{}] Logged in character: {} (ID: {})", account.accname, char_info.name, char_info.id);
        Ok(vec![char_info])
    }
    else
    {
        // SSO login - multiple characters
        let sessions = SimpleSession::login_sf_account(&account.accname, &account.password).await.map_err(|e| format!("Login failed: {}", e))?;

        clear_all_session_state(account.accname.clone());

        let mut characters = Vec::new();
        for mut session in sessions
        {
            // Extract character name and server from session BEFORE any command
            let char_name = session.username().to_string();
            let server = session.server_url().host_str().unwrap_or("unknown").to_string();

            // If we already have a cache entry and it is marked inactive, skip without logging in
            let existing_cache = load_character_cache(&char_name, &server).unwrap_or(None);
            if let Some(cached) = existing_cache.as_ref()
            {
                if !cached.is_active
                {
                    // println!("[{}] Skipping inactive cached character: {} on {}", account.accname, char_name, server);
                    continue;
                }
            }

            let cached_identity: Option<CharacterIdentity> = match get_character_identity(&char_name, &server)
            {
                Ok(identity_opt) => identity_opt,
                Err(e) =>
                {
                    eprintln!("[CACHE] get_character_identity failed for {} on {}: {}", char_name, server, e);
                    None
                }
            };

            // Check if character is inactive in cache - if so, skip the Update
            if let Some(identity) = cached_identity.as_ref()
            {
                // Default to inactive when no setting is present
                let is_active: bool =
                    fetch_character_setting_by_identity::<bool>(&identity.name, identity.id, "settingCharacterActive").unwrap_or(false);
                if !is_active
                {
                    continue;
                }
                let misc_dont_perform_actions_from: String = fetch_character_setting_by_identity(&identity.name, identity.id, "miscDontPerformActionsFrom").unwrap_or("00:00".to_string());
                let misc_dont_perform_actions_to: String = fetch_character_setting_by_identity(&identity.name, identity.id, "miscDontPerformActionsTo").unwrap_or("00:01".to_string());

                if (check_time_in_range(misc_dont_perform_actions_from.clone(), misc_dont_perform_actions_to.clone()))
                {
                    continue;
                }
            }

            // Character is active or not in cache - proceed with Update
            let gs = match session.send_command(Command::Update).await
            {
                Ok(gs) => gs,
                Err(e) =>
                {
                    println!("[{}] Failed to get game state for {}: {}", account.accname, char_name, e);
                    continue;
                }
            };

            let char_info = CharacterInfo {
                id: gs.character.player_id,
                name: gs.character.name.clone(),
            };

            // If no settings exist yet, create a default inactive entry so future runs can skip
            if fetch_character_setting::<bool>(&gs, "settingCharacterActive").is_none()
            {
                let mut defaults = HashMap::new();
                defaults.insert("settingCharacterActive".to_string(), Value::Bool(false));
                if let Err(e) = save_character_settings(&gs.character.name, gs.character.player_id, defaults).await
                {
                    eprintln!("[SETTINGS] Failed to create default settings for {}: {}", gs.character.name, e);
                }
            }

            // Save to character cache (for active characters)
            // Default to inactive if setting missing
            let is_active: bool = fetch_character_setting(&gs, "settingCharacterActive").unwrap_or(false);
            let mount_str = match &gs.character.mount
            {
                Some(Mount::Cow) => "Cow",
                Some(Mount::Horse) => "Horse",
                Some(Mount::Tiger) => "Tiger",
                Some(Mount::Dragon) => "Dragon",
                None => "None",
            };
            let guild_name = gs.guild.as_ref().map(|g| g.name.clone()).unwrap_or_default();

            let cached = CachedCharacter {
                id: gs.character.player_id,
                name: gs.character.name.clone(),
                lvl: gs.character.level,
                alu: gs.tavern.thirst_for_adventure_sec / 60,
                guild: guild_name,
                beers: gs.tavern.beer_drunk,
                mushrooms: gs.character.mushrooms,
                hourglasses: gs.tavern.quicksand_glasses,
                gold: gs.character.silver / 100,
                luckycoins: gs.specials.wheel.lucky_coins,
                fights: gs.arena.fights_for_xp,
                luckyspins: gs.specials.wheel.spins_today,
                petfights: 0, // Would need pet calculation
                dicerolls: gs.tavern.dice_game.remaining,
                server: server.clone(),
                is_active,
                mount: mount_str.to_string(),
                account: account.accname.clone(),
                cached_at: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            };

            if should_update_cache(existing_cache.as_ref())
            {
                if let Err(e) = save_character_cache(&cached)
                {
                    eprintln!("[CACHE] Failed to save cache for {}: {}", gs.character.name, e);
                }
            }

            println!("[{}] Logged in character: {} (ID: {})", account.accname, char_info.name, char_info.id);

            add_session_state(SessionState {
                account_name: account.accname.clone(),
                character_name: gs.character.name.clone(),
                character_id: gs.character.player_id,
                server,
                session,
            });

            characters.push(char_info);
        }

        Ok(characters)
    }
}

/// Login and run the bot loop for an account
async fn login_and_run_account(account: AccountInfo, mut stop_rx: broadcast::Receiver<()>, cooldowns: Arc<RwLock<HashMap<String, DateTime<Local>>>>, blacklist: Arc<RwLock<HashMap<String, BlacklistEntry>>>) -> Result<(), String>
{
    loop
    {
        // Check for stop signal
        if stop_rx.try_recv().is_ok()
        {
            println!("[{}] Received stop signal before login", account.accname);
            break;
        }

        // Login
        let characters = match login_account(&account).await
        {
            Ok(chars) => chars,
            Err(e) =>
            {
                println!("[{}] Login failed: {}, retrying in 30 seconds", account.accname, e);
                tokio::time::sleep(Duration::from_secs(30)).await;
                continue;
            }
        };

        if characters.is_empty()
        {
            println!("[{}] No characters found, retrying in 30 seconds", account.accname);
            tokio::time::sleep(Duration::from_secs(30)).await;
            continue;
        }

        // Run the main loop - it will return when session becomes invalid
        let should_stop = run_account_loop(&account, &characters, &mut stop_rx, cooldowns.clone(), blacklist.clone()).await;

        if should_stop
        {
            break;
        }

        // If we get here, session was invalid - wait a bit and re-login
        println!("[{}] Re-logging in after session invalid...", account.accname);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}

/// Main loop for a single account (replaces JS startSfAccount)
/// Returns true if should stop completely, false if should re-login
async fn run_account_loop(
    account: &AccountInfo,
    characters: &[CharacterInfo],
    stop_rx: &mut broadcast::Receiver<()>,
    cooldowns: Arc<RwLock<HashMap<String, DateTime<Local>>>>,
    blacklist: Arc<RwLock<HashMap<String, BlacklistEntry>>>,
) -> bool
{
    let cooldown_defs = get_cooldowns();
    let mut failed_attempts: HashMap<String, u32> = HashMap::new();
    let mut all_sessions_invalid = false;
    let mut invalid_count = 0;

    println!("[{}] Starting bot loop with {} characters", account.accname, characters.len());

    loop
    {
        // Check for stop signal
        if stop_rx.try_recv().is_ok()
        {
            println!("[{}] Received stop signal", account.accname);
            return true; // Stop completely
        }

        // If all sessions became invalid, return to re-login
        if all_sessions_invalid
        {
            println!("[{}] All sessions invalid, need to re-login", account.accname);
            return false; // Re-login
        }

        invalid_count = 0;

        // Process each character
        for character in characters
        {
            let char_key = format!("{}_{}", character.id, character.name);

            // Check blacklist
            {
                let mut bl = blacklist.write().await;
                if let Some(entry) = bl.get(&char_key)
                {
                    if entry.expiry > Local::now()
                    {
                        // Log skip while blacklisted (show remaining seconds)
                        let remaining = (entry.expiry - Local::now()).num_seconds().max(0);
                        write_character_log(&character.name, character.id, &format!("BLACKLIST_SKIP: still {}s left", remaining));
                        continue; // Still blacklisted
                    }
                    else
                    {
                        // Clean up expired entries
                        bl.remove(&char_key);
                    }
                }
            }

            // Get session
            let session_state = if account.single
            {
                get_session_state_for_single(character.name.clone(), &account.server)
            }
            else
            {
                get_session_state(character.name.clone(), character.id)
            };

            let mut session_state = match session_state
            {
                Some(s) => s,
                None =>
                {
                    println!("[{}] No session for character {}, skipping", account.accname, character.name);
                    continue;
                }
            };

            // Execute commands for this character
            for cmd_name in COMMANDS_TO_EXECUTE
            {
                // Check cooldown first (before getting game state)
                let cooldown_key = format!("{}_{}", character.id, cmd_name);
                {
                    let cds = cooldowns.read().await;
                    if let Some(expiry) = cds.get(&cooldown_key)
                    {
                        if *expiry > Local::now()
                        {
                            // Log cooldown skips (except cmd_complete)
                            if *cmd_name != "cmd_complete"
                            {
                                write_character_log(&character.name, character.id, &format!("COOLDOWN: {} (until {})", cmd_name, expiry.format("%H:%M:%S")));
                            }
                            continue; // On cooldown
                        }
                    }
                }

                // Get game state
                let gs_result = session_state.session.send_command(Command::Update).await;
                let gs = match gs_result
                {
                    Ok(gs) => gs,
                    Err(SFError::ServerError(msg)) if msg == "sessionid invalid" =>
                    {
                        println!("[{}] Session invalid for {}, marking for re-login", account.accname, character.name);
                        // Only count invalid sessions for active characters (based on character identity + settings)
                        let server = session_state.session.server_url().host_str().unwrap_or("unknown").to_string();
                        println!("charactername {}",&character.name);
                        println!("server {}",server);

                        let is_active_identity = match get_character_identity(&character.name, &server)
                        {
                            Ok(Some(identity)) =>
                            {
                                fetch_character_setting_by_identity::<bool>(&identity.name, identity.id, "settingCharacterActive").unwrap_or(false)
                            }
                            Ok(None) =>
                            {
                                println!(
                                    "[{}] No character identity found for {} on {} while handling invalid session",
                                    account.accname, character.name, server
                                );
                                true // default: count as active if no identity
                            }
                            Err(e) =>
                            {
                                println!(
                                    "[{}] Identity error for {} on {}: {}",
                                    account.accname, character.name, server, e
                                );
                                true
                            }
                        };

                        let is_active_identity2 = match get_character_identity(&character.name, &server)
                        {
                            Ok(Some(identity)) =>
                            {
                                let is_active = fetch_character_setting_by_identity::<bool>(&identity.name, identity.id, "settingCharacterActive").unwrap_or(false);
                                if !is_active
                                {
                                    false
                                }
                                else
                                {
                                    let misc_dont_perform_actions_from: String = fetch_character_setting_by_identity(&identity.name, identity.id, "miscDontPerformActionsFrom").unwrap_or("00:00".to_string());
                                    let misc_dont_perform_actions_to: String = fetch_character_setting_by_identity(&identity.name, identity.id, "miscDontPerformActionsTo").unwrap_or("00:01".to_string());
                                    println!("time is in range");
                                    !check_time_in_range(misc_dont_perform_actions_from, misc_dont_perform_actions_to)
                                }
                            }
                            Ok(None) =>
                            {
                                println!(
                                    "[{}] No character identity found for {} on {} while handling invalid session",
                                    account.accname, character.name, server
                                );
                                true // default: count as active if no identity
                            }
                            Err(e) =>
                            {
                                println!(
                                    "[{}] Identity error for {} on {}: {}",
                                    account.accname, character.name, server, e
                                );
                                true
                            }
                        };

                        if is_active_identity
                        {
                            invalid_count += 1;
                            if invalid_count >= characters.len()
                            {
                                all_sessions_invalid = true;
                            }
                        }
                        break;
                    }
                    Err(e) =>
                    {
                        let error_str = e.to_string();
                        println!("[{}] Error getting game state for {}: {}", account.accname, character.name, error_str);
                        if error_str.contains("invalid") || error_str.contains("session")
                        {
                            invalid_count += 1;
                            if invalid_count >= characters.len()
                            {
                                all_sessions_invalid = true;
                            }
                        }
                        continue;
                    }
                };

                // Check if character is active in settings
                let is_active: bool = crate::fetch_character_setting(&gs, "settingCharacterActive").unwrap_or(false);
                if !is_active
                {
                    // Character is not active, skip all commands for this character
                    // write_character_log(&character.name, character.id, "CHARACTER INACTIVE - skipping all commands");
                    break;
                }

                // Check if command should be skipped based on settings
                // Note: skipFunction now logs detailed SKIP_REASON directly
                if skipFunction(&gs, cmd_name)
                {
                    continue;
                }

                // Log command execution (only for active characters after skip check)
                println!("[{}] Executing command: {}", character.name, cmd_name);

                // Track current action
                set_current_character_obj(account.accname.clone(), character.name.clone(), character.id, cmd_name.to_string());

                // Log to character-specific file (skip internal commands)
                if *cmd_name != "cmd_complete"
                {
                    write_character_log(&character.name, character.id, &format!("executing: {}", cmd_name));
                }
                // Execute the command
                let cmd_result: Result<(), String> = {
                    let result = run_func(&mut session_state.session, cmd_name, account.single, if account.single { &account.server } else { "" }).await;
                    result.map(|_| ()).map_err(|e| e.to_string())
                };

                match cmd_result
                {
                    Ok(()) =>
                    {
                        // Success - set cooldown
                        if let Some(&cooldown_ms) = cooldown_defs.get(cmd_name)
                        {
                            let expiry = Local::now() + chrono::Duration::milliseconds(cooldown_ms as i64);
                            let mut cds = cooldowns.write().await;
                            cds.insert(cooldown_key.clone(), expiry);
                            // Log cooldown set
                            if *cmd_name != "cmd_complete"
                            {
                                write_character_log(&character.name, character.id, &format!("COOLDOWN_SET: {} -> {}ms (until {})", cmd_name, cooldown_ms, expiry.format("%H:%M:%S")));
                            }
                        }
                        else
                        {
                            // No cooldown defined for this command!
                            if *cmd_name != "cmd_complete"
                            {
                                write_character_log(&character.name, character.id, &format!("NO_COOLDOWN_DEFINED: {} - will run again immediately!", cmd_name));
                            }
                        }
                        failed_attempts.remove(&char_key);
                    }
                    Err(error_msg) =>
                    {
                        write_character_log(&character.name, character.id, &format!("ERROR: {} failed: {}", cmd_name, error_msg));

                        let attempts = failed_attempts.entry(char_key.clone()).or_insert(0);
                        *attempts += 1;

                        if *attempts >= 3
                        {
                            write_character_log(&character.name, character.id, &format!("SKIP: {} failed 3 times, skipping", cmd_name));
                            *attempts = 0;
                        }

                        if error_msg.contains("Invalid Session") || error_msg.contains("sessionid invalid")
                        {
                            // Blacklist this character for a configured cooloff
                            let bl_seconds = match crate::utils::get_global_settings().await {
                                Ok(settings) => settings
                                    .get("doNotRelogCharacterSeconds")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(3),
                                Err(_) => 3,
                            };

                            let expiry = Local::now() + chrono::Duration::seconds(bl_seconds as i64);
                            {
                                let mut bl = blacklist.write().await;
                                bl.insert(char_key.clone(), BlacklistEntry { expiry });
                            }
                            write_character_log(
                                &character.name,
                                character.id,
                                &format!(
                                    "BLACKLISTED for invalid session until {} ({}s)",
                                    expiry.format("%H:%M:%S"),
                                    bl_seconds
                                ),
                            );

                            invalid_count += 1;
                            if invalid_count >= characters.len()
                            {
                                all_sessions_invalid = true;
                            }
                            break;
                        }
                    }
                }

                // Small delay between commands
                tokio::time::sleep(Duration::from_millis(fastrand::u64(50..150))).await;
            }

            // Update session state
            if account.single
            {
                // For single accounts
                crate::find_and_remove_single_session_state(character.name.to_lowercase(), &account.server.to_lowercase());
                add_session_state(SessionState {
                    account_name: account.accname.clone(),
                    character_name: character.name.to_lowercase(),
                    character_id: character.id,
                    server: account.server.clone(),
                    session: session_state.session.clone(),
                });
            }
            else
            {
                // Extract server from session URL for normal SSO accounts
                let server = session_state.session.server_url().host_str().unwrap_or("unknown").to_string();

                overwrite_session_state(account.accname.clone(), character.name.clone(), character.id, session_state.session.clone());
                add_session_state(SessionState {
                    account_name: account.accname.clone(),
                    character_name: character.name.clone(),
                    character_id: character.id,
                    server,
                    session: session_state.session,
                });
            }

            // Small delay between characters
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Sleep between full cycles (30-100ms as in JS)
        let sleep_time = fastrand::u64(30..100);
        tokio::time::sleep(Duration::from_millis(sleep_time)).await;
    }

    println!("[{}] Bot loop ended", account.accname);
    true // Stop completely (only reached via break from stop signal)
}

// Helper function to get session for single accounts
fn get_session_state_for_single(name: String, server: &str) -> Option<SessionState> { crate::get_session_state_for_single(name, server) }
