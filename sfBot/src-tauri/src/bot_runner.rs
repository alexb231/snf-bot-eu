






use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use chrono::{DateTime, Local};
use once_cell::sync::Lazy;
use serde_json::Value;
use sf_api::{
    command::Command,
    error::SFError,
    gamestate::{character::Mount, tavern::CurrentAction},
    SimpleSession,
};
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


#[derive(Clone, Debug)]
pub struct AccountInfo
{
    pub accname: String,
    pub password: String,
    pub single: bool,
    pub server: String,
}


#[derive(Clone, Debug)]
pub struct CharacterInfo
{
    pub id: u32,
    pub name: String,
}


const MAX_LOG_LINES: usize = 10000;

const LOG_TRIM_EVERY: usize = 200;

const LOG_FLUSH_INTERVAL: Duration = Duration::from_secs(30);

const LOG_FLUSH_BYTES: usize = 64 * 1024;

static LOG_WRITE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static LOG_TRIM_COUNTERS: Lazy<Mutex<HashMap<String, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static LOG_BUFFERS: Lazy<Mutex<HashMap<String, LogBuffer>>> = Lazy::new(|| Mutex::new(HashMap::new()));

struct LogBuffer
{
    pending: String,
    pending_lines: usize,
    last_flush: Instant,
}

const ACCOUNT_RESTART_EVERY: Duration = Duration::from_secs(2 * 60 * 60);
const ACCOUNT_RESTART_JITTER_MS: u64 = 60_000;
const ARENA_ACTION_GUARD_MS: i64 = 90_000;
const BOT_START_DISCORD_WEBHOOK_URL: &str = "https://discord.com/api/webhooks/1469436535064756320/yVjIG8QQG0mZAYbcrxAUtoNLcQEcMXCxAgR_3tIGTNqU0bNAJBWsZmNZ3W3rtHQADl9T";
const BOT_START_WEBHOOK_TIMEOUT_SECS: u64 = 10;

async fn send_bot_start_webhook(accounts: &[AccountInfo])
{
    if BOT_START_DISCORD_WEBHOOK_URL.trim().is_empty()
    {
        return;
    }

    let started_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut lines = Vec::new();
    for account in accounts
    {
        let server = if account.server.trim().is_empty() {
            "default"
        } else {
            account.server.as_str()
        };
        lines.push(format!("- account: `{}` | server: `{}` | single: `{}`", account.accname, server, account.single));
    }

    let content = format!(
        "**Bot started**\nTime: `{}`\nAccounts (no passwords):\n{}",
        started_at,
        lines.join("\n")
    );

    let payload = serde_json::json!({ "content": content });
    let client = reqwest::Client::builder().timeout(Duration::from_secs(BOT_START_WEBHOOK_TIMEOUT_SECS)).build();

    let Ok(client) = client
    else
    {
        return;
    };

    if let Err(e) = client.post(BOT_START_DISCORD_WEBHOOK_URL).json(&payload).send().await
    {
        eprintln!("Failed to send bot start webhook: {}", e);
    }
}


fn log_dir() -> std::path::PathBuf { exe_relative_path("logs") }



pub fn write_character_log(character_name: &str, character_id: u32, message: &str)
{
    let log_path = log_dir();

    
    if let Err(e) = fs::create_dir_all(&log_path)
    {
        eprintln!("Failed to create logs directory: {}", e);
        return;
    }

    let filename = log_path.join(format!("{}_{}.log", character_name, character_id));
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_line = format!("[{}] {}\n", timestamp, message);
    print!("{}", log_line);

    let key = filename.to_string_lossy().to_string();
    let (flush_content, flush_lines) = {
        let mut buffers = LOG_BUFFERS.lock().unwrap();
        let buf = buffers.entry(key).or_insert_with(|| LogBuffer {
            pending: String::new(),
            pending_lines: 0,
            last_flush: Instant::now(),
        });

        buf.pending.push_str(&log_line);
        buf.pending_lines += 1;

        let should_flush = buf.pending.len() >= LOG_FLUSH_BYTES || buf.last_flush.elapsed() >= LOG_FLUSH_INTERVAL;
        if should_flush
        {
            let content = std::mem::take(&mut buf.pending);
            let lines = buf.pending_lines;
            buf.pending_lines = 0;
            buf.last_flush = Instant::now();
            (Some(content), lines)
        }
        else
        {
            (None, 0)
        }
    };

    if let Some(content) = flush_content
    {
        flush_log_buffer(&filename, &content, flush_lines);
    }
}

fn flush_log_buffer(filename: &std::path::Path, content: &str, lines_written: usize)
{
    {
        let _lock = LOG_WRITE_LOCK.lock().unwrap();
        if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(filename)
        {
            if file.write_all(content.as_bytes()).is_err()
            {
                eprintln!("Failed to write log file: {}", filename.display());
            }
        }
        else
        {
            eprintln!("Failed to open log file: {}", filename.display());
        }
    }

    let mut counters = LOG_TRIM_COUNTERS.lock().unwrap();
    let key = filename.to_string_lossy().to_string();
    let counter = counters.entry(key).or_insert(0);
    *counter += lines_written;
    if *counter >= LOG_TRIM_EVERY
    {
        *counter = 0;
        drop(counters);
        trim_log_file(filename);
    }
}

fn trim_log_file(filename: &std::path::Path)
{
    let _lock = LOG_WRITE_LOCK.lock().unwrap();
    let content = match fs::read_to_string(filename)
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut lines: Vec<&str> = content.lines().collect();
    if lines.len() <= MAX_LOG_LINES
    {
        return;
    }

    let start = lines.len() - MAX_LOG_LINES;
    lines = lines.split_off(start);
    let content = lines.join("\n") + "\n";
    if let Err(e) = fs::write(filename, content)
    {
        eprintln!("Failed to trim log file: {}", e);
    }
}


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


const COMMANDS_TO_EXECUTE: &[&str] = &[
    "cmd_city_guard",
    "cmd_play_expeditions_gold",
    "cmd_play_expeditions_exp",
    "cmd_upgrade_skill_points",
    "cmd_collect_daily_and_weekly_rewards",
    "cmd_play_dice",
    "cmd_accept_unlockables",
    "cmd_play_idle_game",
    "cmd_collect_fortress_resources",
    "cmd_use_toilet",
    "cmd_check_and_swap_equipment",
    "cmd_manage_inventory",
    "cmd_collect_free_mushroom",
    "cmd_fight_demon_portal",
    "cmd_fight_guild_portal",
    "cmd_fight_dungeon_with_lowest_level",
    "cmd_arena_fight",
    "cmd_enchant_items",
    "cmd_start_searching_for_gem",
    "cmd_attack_fortress",
    "cmd_train_fortress_units",
    "cmd_perform_underworld_atk_suggested_enemy",
    "cmd_collect_underworld_resources",
    "cmd_build_underworld_perfect_order",
    "cmd_fight_pet_arena",
    "cmd_perform_daily_tasks",
    "cmd_buy_mount",
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


fn get_cooldowns() -> HashMap<&'static str, u64>
{
    let mut cooldowns = HashMap::new();
    cooldowns.insert("cmd_play_expeditions_gold", 300);
    cooldowns.insert("cmd_play_expeditions_exp", 300);
    cooldowns.insert("cmd_city_guard", 300);
    cooldowns.insert("cmd_upgrade_skill_points", 15 * 60_000);
    cooldowns.insert("cmd_collect_daily_and_weekly_rewards", 60 * 60_000);
    cooldowns.insert("cmd_play_dice", 2 * 60_000);
    cooldowns.insert("cmd_accept_unlockables", 5 * 60_000);
    cooldowns.insert("cmd_play_idle_game", 60_000);
    cooldowns.insert("cmd_collect_fortress_resources", 30 * 60_000);
    cooldowns.insert("cmd_use_toilet", 60 * 60_000);
    cooldowns.insert("cmd_manage_inventory", 5 * 60_000);
    cooldowns.insert("cmd_collect_free_mushroom", 60 * 60_000);
    cooldowns.insert("cmd_fight_demon_portal", 30 * 60_000);
    cooldowns.insert("cmd_fight_guild_portal", 30 * 60_000);
    cooldowns.insert("cmd_fight_dungeon_with_lowest_level", 10 * 60_000);
    cooldowns.insert("cmd_arena_fight", 5 * 60_000);
    cooldowns.insert("cmd_enchant_items", 10 * 60_000);
    cooldowns.insert("cmd_start_searching_for_gem", 3 * 60_000);
    cooldowns.insert("cmd_attack_fortress", 3 * 60_000);
    cooldowns.insert("cmd_train_fortress_units", 5 * 60_000);
    cooldowns.insert("cmd_perform_underworld_atk_suggested_enemy", 5 * 60_000);
    cooldowns.insert("cmd_collect_underworld_resources", 30 * 60_000);
    cooldowns.insert("cmd_build_underworld_perfect_order", 5 * 60_000);
    cooldowns.insert("cmd_fight_pet_arena", 15 * 60_000);
    cooldowns.insert("cmd_check_and_swap_equipment", 15 * 60_000);
    cooldowns.insert("cmd_perform_daily_tasks", 60 * 60_000);
    cooldowns.insert("cmd_buy_mount", 15 * 60_000);
    cooldowns.insert("cmd_spin_lucky_wheel", 10 * 60_000);
    cooldowns.insert("cmd_build_fortress_our_order", 5 * 60_000);
    cooldowns.insert("cmd_sign_up_for_guild_attack_and_defense", 15 * 60_000);
    cooldowns.insert("cmd_fight_hydra", 30 * 60_000);
    cooldowns.insert("cmd_feed_all_pets", 60 * 60_000);
    cooldowns.insert("cmd_collect_gifts_from_mail", 5 * 60_000);
    cooldowns.insert("cmd_fight_pet_dungeon", 60_000);
    cooldowns.insert("cmd_brew_potions_using_fruits", 60 * 60_000);
    cooldowns.insert("cmd_level_up_uw_keeper", 5 * 60_000);
    cooldowns.insert("cmd_play_hellevator", 2 * 60_000);
    cooldowns.insert("cmd_fill_scrapbook", 10 * 60_000);
    
    cooldowns
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotState
{
    Stopped,
    Running,
    Paused,
}


#[derive(Clone)]
struct BlacklistEntry
{
    expiry: DateTime<Local>,
}


pub struct BotRunner
{
    state: BotState,
    accounts: Vec<AccountInfo>,
    characters: Vec<CharacterDisplay>,
    current_character: Option<CurrentCharacterInfo>,
    blacklist: HashMap<String, BlacklistEntry>,
    cooldowns: HashMap<String, DateTime<Local>>, 
    task_handles: Vec<JoinHandle<()>>,
    stop_signal: Option<broadcast::Sender<()>>,
    pause_flag: Arc<AtomicBool>,
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
            pause_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    
    pub async fn start(&mut self, accounts: Vec<AccountInfo>) -> Result<(), String>
    {
        if self.state == BotState::Running
        {
            return Err("Bot is already running".to_string());
        }

        self.pause_flag.store(false, Ordering::Relaxed);
        self.accounts = accounts.clone();
        self.state = BotState::Running;

        let webhook_accounts = self.accounts.clone();
        tokio::spawn(async move {
            send_bot_start_webhook(&webhook_accounts).await;
        });

        
        let (stop_tx, _) = broadcast::channel(1);
        self.stop_signal = Some(stop_tx.clone());

        
        for account in accounts
        {
            let stop_rx = stop_tx.subscribe();
            let cooldowns = Arc::new(RwLock::new(self.cooldowns.clone()));
            let blacklist = Arc::new(RwLock::new(self.blacklist.clone()));
            let pause_flag = self.pause_flag.clone();

            let handle = tokio::spawn(async move {
                
                match login_and_run_account(account, stop_rx, cooldowns, blacklist, pause_flag).await
                {
                    Ok(_) => println!("Account loop finished"),
                    Err(e) => println!("Account loop error: {}", e),
                }
            });

            self.task_handles.push(handle);
        }

        Ok(())
    }

    pub async fn stop(&mut self)
    {
        if let Some(stop_tx) = self.stop_signal.take()
        {
            let _ = stop_tx.send(());
        }
        self.pause_flag.store(false, Ordering::Relaxed);

        
        for handle in self.task_handles.drain(..)
        {
            let _ = handle.await;
        }

        self.state = BotState::Stopped;
        self.current_character = None;
    }

    pub fn pause(&mut self)
    {
        if self.state == BotState::Running
        {
            self.state = BotState::Paused;
            self.pause_flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn resume(&mut self)
    {
        if self.state == BotState::Paused
        {
            self.state = BotState::Running;
            self.pause_flag.store(false, Ordering::Relaxed);
        }
    }

    
    pub fn get_status(&self) -> BotStatusResponse
    {
        
        let current_char = crate::get_current_character_obj();
        let current_character = current_char.list.first().map(|c| CurrentCharacterInfo {
            account: c.accname.clone(),
            name: c.name.clone(),
            id: c.id,
            current_action: c.ziel.clone(),
        });

        
        let session_states = crate::get_all_session_states();
        let mut characters: Vec<CharacterStatusInfo> = Vec::new();

        for ss in session_states
        {
            
            if let Some(gs) = ss.session.game_state()
            {
                
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

    
    pub fn get_characters(&self) -> Vec<CharacterDisplay> { self.characters.clone() }

    
    pub fn update_characters(&mut self, characters: Vec<CharacterDisplay>) { self.characters = characters; }

    
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

fn is_transient_sf_error(err: &SFError) -> bool { matches!(err, SFError::ConnectionError | SFError::EmptyResponse | SFError::TooShortResponse { .. }) }

fn is_transient_error_message(msg: &str) -> bool
{
    let msg = msg.to_ascii_lowercase();
    msg.contains("could not communicate with the server") || msg.contains("empty response") || msg.contains("connectionerror") || msg.contains("connection error") || msg.contains("timeout") || msg.contains("timed out") || msg.contains("dns") || msg.contains("resolve")
}

fn is_arena_action_command(cmd_name: &str) -> bool
{
    matches!(cmd_name, "cmd_arena_fight" | "cmd_fill_scrapbook" | "cmd_perform_daily_tasks")
}


async fn wait_while_paused(pause_flag: &Arc<AtomicBool>, stop_rx: &mut broadcast::Receiver<()>, account_name: &str) -> bool
{
    if !pause_flag.load(Ordering::Relaxed)
    {
        return false;
    }

    println!("[{}] Paused", account_name);
    loop
    {
        if stop_rx.try_recv().is_ok()
        {
            println!("[{}] Received stop signal while paused", account_name);
            return true;
        }
        if !pause_flag.load(Ordering::Relaxed)
        {
            println!("[{}] Resuming", account_name);
            return false;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

impl Default for BotRunner
{
    fn default() -> Self { Self::new() }
}


async fn login_account(account: &AccountInfo) -> Result<Vec<CharacterInfo>, String>
{
    println!("[{}] Logging in...", account.accname);

    if account.single
    {
        
        
        let char_name = account.accname.to_lowercase();
        let server = account.server.to_lowercase();

        
        let existing_cache = load_character_cache(&char_name, &server).unwrap_or(None);
        if let Some(cached) = existing_cache.as_ref()
        {
            if !cached.is_active
            {
                println!("[{}] Skipping inactive single account: {} on {}", account.accname, char_name, server);
                return Ok(vec![]); 
            }
        }

        
        let mut session = SimpleSession::login(&account.accname, &account.password, &account.server).await.map_err(|e| format!("Login failed: {}", e))?;

        
        let gs = session.send_command(Command::Update).await.map_err(|e| format!("Failed to get game state: {}", e))?;

        
        
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
            petfights: 0, 
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
        
        let sessions = SimpleSession::login_sf_account(&account.accname, &account.password).await.map_err(|e| format!("Login failed: {}", e))?;

        clear_all_session_state(account.accname.clone());

        let mut characters = Vec::new();
        for mut session in sessions
        {
            
            let char_name = session.username().to_string();
            let server = session.server_url().host_str().unwrap_or("unknown").to_string();

            
            
            let existing_cache = load_character_cache(&char_name, &server).unwrap_or(None);
            if let Some(cached) = existing_cache.as_ref()
            {
                if !cached.is_active
                {
                    
                    
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

            
            if let Some(identity) = cached_identity.as_ref()
            {
                
                let is_active: bool = fetch_character_setting_by_identity::<bool>(&identity.name, identity.id, "settingCharacterActive").unwrap_or(false);
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

            
            
            if fetch_character_setting::<bool>(&gs, "settingCharacterActive").is_none()
            {
                let mut defaults = HashMap::new();
                defaults.insert("settingCharacterActive".to_string(), Value::Bool(false));
                if let Err(e) = save_character_settings(&gs.character.name, gs.character.player_id, defaults).await
                {
                    eprintln!("[SETTINGS] Failed to create default settings for {}: {}", gs.character.name, e);
                }
            }

            
            
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
                petfights: 0, 
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


async fn login_and_run_account(account: AccountInfo, mut stop_rx: broadcast::Receiver<()>, cooldowns: Arc<RwLock<HashMap<String, DateTime<Local>>>>, blacklist: Arc<RwLock<HashMap<String, BlacklistEntry>>>, pause_flag: Arc<AtomicBool>) -> Result<(), String>
{
    loop
    {
        
        if stop_rx.try_recv().is_ok()
        {
            println!("[{}] Received stop signal before login", account.accname);
            break;
        }

        if wait_while_paused(&pause_flag, &mut stop_rx, &account.accname).await
        {
            break;
        }

        
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

        
        let jitter = fastrand::u64(0..=ACCOUNT_RESTART_JITTER_MS);
        let restart_deadline = tokio::time::Instant::now() + ACCOUNT_RESTART_EVERY + Duration::from_millis(jitter);
        let should_stop = tokio::select! {
            _ = tokio::time::sleep_until(restart_deadline) => {
                println!("[{}] WATCHDOG: forcing account relog after 2h interval", account.accname);
                for character in &characters {
                    write_character_log(&character.name, character.id, "WATCHDOG: forced account relog (2h interval)");
                }
                false
            }
            should_stop = run_account_loop(&account, &characters, &mut stop_rx, cooldowns.clone(), blacklist.clone(), pause_flag.clone()) => should_stop,
        };

        if should_stop
        {
            break;
        }

        
        println!("[{}] Re-logging in after session invalid...", account.accname);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}

async fn blacklist_character_for_invalid_session(blacklist: &Arc<RwLock<HashMap<String, BlacklistEntry>>>, char_key: &str, character: &CharacterInfo)
{
    let bl_seconds = match crate::utils::get_global_settings().await
    {
        Ok(settings) => settings.get("doNotRelogCharacterSeconds").and_then(|v| v.as_u64()).unwrap_or(3),
        Err(_) => 3,
    };

    let expiry = Local::now() + chrono::Duration::seconds(bl_seconds as i64);
    {
        let mut bl = blacklist.write().await;
        bl.insert(char_key.to_string(), BlacklistEntry { expiry });
    }

    write_character_log(&character.name, character.id, &format!("BLACKLISTED for invalid session until {} ({}s)", expiry.format("%H:%M:%S"), bl_seconds));
}



async fn run_account_loop(account: &AccountInfo, characters: &[CharacterInfo], stop_rx: &mut broadcast::Receiver<()>, cooldowns: Arc<RwLock<HashMap<String, DateTime<Local>>>>, blacklist: Arc<RwLock<HashMap<String, BlacklistEntry>>>, pause_flag: Arc<AtomicBool>) -> bool
{
    let cooldown_defs = get_cooldowns();
    let mut failed_attempts: HashMap<String, u32> = HashMap::new();
    let mut all_sessions_invalid = false;
    let mut invalid_count = 0;
    let mut offline_failures: u32 = 0;

    println!("[{}] Starting bot loop with {} characters", account.accname, characters.len());

    loop
    {
        
        if stop_rx.try_recv().is_ok()
        {
            println!("[{}] Received stop signal", account.accname);
            return true; 
        }

        if wait_while_paused(&pause_flag, stop_rx, &account.accname).await
        {
            return true;
        }

        
        if all_sessions_invalid
        {
            println!("[{}] All sessions invalid, need to re-login", account.accname);
            return false; 
        }

        invalid_count = 0;

        
        for character in characters
        {
            if wait_while_paused(&pause_flag, stop_rx, &account.accname).await
            {
                return true;
            }

            let char_key = format!("{}_{}", character.id, character.name);

            
            {
                let mut bl = blacklist.write().await;
                if let Some(entry) = bl.get(&char_key)
                {
                    if entry.expiry > Local::now()
                    {
                        
                        let remaining = (entry.expiry - Local::now()).num_seconds().max(0);
                        println!("[{}] BLACKLIST_SKIP: still {}s left", character.name, remaining);
                        continue; 
                    }
                    else
                    {
                        
                        bl.remove(&char_key);
                    }
                }
            }

            
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

            
            for cmd_name in COMMANDS_TO_EXECUTE
            {
                if stop_rx.try_recv().is_ok()
                {
                    println!("[{}] Received stop signal", account.accname);
                    return true;
                }
                if wait_while_paused(&pause_flag, stop_rx, &account.accname).await
                {
                    return true;
                }

                let is_arena_action_cmd = is_arena_action_command(cmd_name);
                let arena_guard_key = if is_arena_action_cmd { Some(format!("{}_arena_action_guard", character.id)) } else { None };

                if let Some(guard_key) = &arena_guard_key
                {
                    let cds = cooldowns.read().await;
                    if let Some(expiry) = cds.get(guard_key)
                    {
                        if *expiry > Local::now()
                        {
                            continue;
                        }
                    }
                }

                
                let cooldown_key = format!("{}_{}", character.id, cmd_name);
                {
                    let cds = cooldowns.read().await;
                    if let Some(expiry) = cds.get(&cooldown_key)
                    {
                        if *expiry > Local::now()
                        {
                            
                            if *cmd_name != "cmd_complete"
                            {
                                if (false)
                                {
                                    println!("[{}] COOLDOWN: {} (until {})", character.name, cmd_name, expiry.format("%H:%M:%S"));
                                }
                            }
                            continue; 
                        }
                    }
                }

                
                let gs_result = session_state.session.send_command(Command::Update).await;
                let gs = match gs_result
                {
                    Ok(gs) =>
                    {
                        if offline_failures > 0
                        {
                            write_character_log(&character.name, character.id, "NETWORK: connection restored");
                            offline_failures = 0;
                        }
                        gs
                    }
                    Err(err) if is_transient_sf_error(&err) =>
                    {
                        offline_failures = offline_failures.saturating_add(1);
                        write_character_log(&character.name, character.id, &format!("NETWORK: {} - server not responding, skipping character", err));
                        println!("[{}] Network issue for {}: {}, skipping character", account.accname, character.name, err);
                        break;
                    }
                    Err(SFError::ServerError(msg)) if msg == "sessionid invalid" =>
                    {
                        println!("[{}] Session invalid for {}, marking for re-login", account.accname, character.name);
                        blacklist_character_for_invalid_session(&blacklist, &char_key, character).await;
                        
                        
                        let server = session_state.session.server_url().host_str().unwrap_or("unknown").to_string();
                        println!("charactername {}", &character.name);
                        println!("server {}", server);

                        let is_active_identity = match get_character_identity(&character.name, &server)
                        {
                            Ok(Some(identity)) => fetch_character_setting_by_identity::<bool>(&identity.name, identity.id, "settingCharacterActive").unwrap_or(false),
                            Ok(None) =>
                            {
                                println!("[{}] No character identity found for {} on {} while handling invalid session", account.accname, character.name, server);
                                true 
                            }
                            Err(e) =>
                            {
                                println!("[{}] Identity error for {} on {}: {}", account.accname, character.name, server, e);
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
                                println!("[{}] No character identity found for {} on {} while handling invalid session", account.accname, character.name, server);
                                true 
                            }
                            Err(e) =>
                            {
                                println!("[{}] Identity error for {} on {}: {}", account.accname, character.name, server, e);
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
                    Err(SFError::InvalidRequest(msg)) if msg.contains("session") =>
                    {
                        println!("[{}] Session invalid for {}, marking for re-login", account.accname, character.name);
                        blacklist_character_for_invalid_session(&blacklist, &char_key, character).await;
                        invalid_count += 1;
                        if invalid_count >= characters.len()
                        {
                            all_sessions_invalid = true;
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
                
                let is_active: bool = crate::fetch_character_setting(&gs, "settingCharacterActive").unwrap_or(false);
                if !is_active
                {
                    
                    
                    
                    break;
                }

                
                
                if skipFunction(&gs, cmd_name)
                {
                    continue;
                }

                set_current_character_obj(account.accname.clone(), character.name.clone(), character.id, cmd_name.to_string());

                let cmd_result: Result<(), String> = {
                    let result = run_func(&mut session_state.session, cmd_name, account.single, if account.single { &account.server } else { "" }).await;
                    result.map(|_| ()).map_err(|e| e.to_string())
                };

                match cmd_result
                {
                    Ok(()) =>
                    {
                        
                        if let Some(&cooldown_ms) = cooldown_defs.get(cmd_name)
                        {
                            let expiry = Local::now() + chrono::Duration::milliseconds(cooldown_ms as i64);
                            let mut cds = cooldowns.write().await;
                            cds.insert(cooldown_key.clone(), expiry);
                            
                            if *cmd_name != "cmd_complete"
                            {
                                if (false)
                                {
                                    println!("[{}] COOLDOWN_SET: {} -> {}ms (until {})", character.name, cmd_name, cooldown_ms, expiry.format("%H:%M:%S"));
                                }
                            }
                        }
                        else
                        {
                            
                            if *cmd_name != "cmd_complete"
                            {
                                if (false)
                                {
                                    println!("[{}] NO_COOLDOWN_DEFINED: {} - will run again immediately!", character.name, cmd_name);
                                }
                            }
                        }

                        if let Some(guard_key) = &arena_guard_key
                        {
                            let guard_expiry = Local::now() + chrono::Duration::milliseconds(ARENA_ACTION_GUARD_MS);
                            let mut cds = cooldowns.write().await;
                            cds.insert(guard_key.clone(), guard_expiry);
                        }

                        failed_attempts.remove(&char_key);
                    }
                    Err(error_msg) =>
                    {
                        if let Some(guard_key) = &arena_guard_key
                        {
                            let guard_expiry = Local::now() + chrono::Duration::milliseconds(ARENA_ACTION_GUARD_MS);
                            let mut cds = cooldowns.write().await;
                            cds.insert(guard_key.clone(), guard_expiry);
                        }

                        if is_transient_error_message(&error_msg)
                        {
                            offline_failures = offline_failures.saturating_add(1);
                            write_character_log(&character.name, character.id, &format!("NETWORK: {} - server not responding, skipping character", error_msg));
                            println!("[{}] Network issue during {}: {}, skipping character", account.accname, cmd_name, error_msg);
                            break;
                        }

                        write_character_log(&character.name, character.id, &format!("ERROR: {} failed: {}", cmd_name, error_msg));

                        let attempts = failed_attempts.entry(char_key.clone()).or_insert(0);
                        *attempts += 1;

                        if *attempts >= 3
                        {
                            println!("[{}] SKIP: {} failed 3 times, skipping", character.name, cmd_name);
                            *attempts = 0;
                        }

                        if error_msg.contains("Invalid Session") || error_msg.contains("sessionid invalid")
                        {
                            blacklist_character_for_invalid_session(&blacklist, &char_key, character).await;

                            invalid_count += 1;
                            if invalid_count >= characters.len()
                            {
                                all_sessions_invalid = true;
                            }
                            break;
                        }
                    }
                }
            }
            
            if account.single
            {
                
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
        }

        
        let sleep_time = fastrand::u64(30..100);
        tokio::time::sleep(Duration::from_millis(sleep_time)).await;
    }

    println!("[{}] Bot loop ended", account.accname);
    true 
}


fn get_session_state_for_single(name: String, server: &str) -> Option<SessionState> { crate::get_session_state_for_single(name, server) }
