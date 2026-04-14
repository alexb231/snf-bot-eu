#![allow(warnings)]
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sf_api::{command::Command, gamestate::character::Class, SimpleSession};



const CLASS_KEYS: [&str; 12] = [
    "Warrior",
    "Mage",
    "Scout",
    "Assassin",
    "Battle Mage",
    "Berserker",
    "Demon Hunter",
    "Druid",
    "Bard",
    "Necromancer",
    "Paladin",
    "Plague Doctor",
];

const DEFAULT_MIN_LEVEL: u32 = 1;
const DEFAULT_MAX_LEVEL: u32 = 40;
const DEFAULT_PAGE_LIMIT: usize = 40;

#[derive(Serialize)]
struct Entry {
    server: String,
    player1: String,
    player2: String,
}

#[derive(Debug, Deserialize)]
struct UserConfigFile {
    accounts: Vec<UserConfig>,
}

#[derive(Debug, Deserialize)]
struct UserConfig {
    accname: String,
    password: String,
    server: String,
    #[allow(dead_code)]
    single: Option<bool>,
}

struct Args {
    config_path: PathBuf,
    output_path: PathBuf,
    min_level: u32,
    max_level: u32,
    page_limit: usize,
    servers: Option<HashSet<String>>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()?;

    runtime.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn Error>> {
    let args = parse_args(env::args().collect())?;
    let accounts = read_accounts(&args.config_path)?;
    let sessions_by_server = build_sessions(accounts, &args.servers).await?;

    if sessions_by_server.is_empty() {
        return Err("No server sessions found to process.".into());
    }

    let mut servers: Vec<String> = sessions_by_server.keys().cloned().collect();
    servers.sort();

    let mut class_map: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    for key in CLASS_KEYS {
        class_map.insert(key.to_string(), Vec::new());
    }

    for server in servers {
        let mut session = match sessions_by_server.get(&server).cloned() {
            Some(session) => session,
            None => continue,
        };

        let total_players = match session
            .send_command(Command::HallOfFamePage { page: 0 })
            .await
        {
            Ok(gs) => gs.hall_of_fames.players_total,
            Err(err) => {
                eprintln!("[{server}] Hall of Fame query failed: {err}");
                continue;
            }
        };

        if total_players == 0 {
            eprintln!("[{server}] No players found in Hall of Fame.");
            continue;
        }

        let mut found: HashMap<&'static str, Vec<String>> =
            CLASS_KEYS.iter().map(|k| (*k, Vec::new())).collect();

        let mut page = (total_players.saturating_sub(1) / 51) as usize;
        let mut pages_checked = 0;

        loop {
            let gs = match session.send_command(Command::HallOfFamePage { page }).await {
                Ok(gs) => gs,
                Err(err) => {
                    eprintln!("[{server}] Hall of Fame page {page} failed: {err}");
                    break;
                }
            };

            for player in &gs.hall_of_fames.players {
                if player.level < args.min_level || player.level > args.max_level {
                    continue;
                }
                let Some(key) = class_key(player.class) else {
                    continue;
                };
                let list = found.get_mut(key).expect("class key missing");
                if list.len() < 2 && !list.contains(&player.name) {
                    list.push(player.name.clone());
                }
            }

            pages_checked += 1;
            if pages_checked >= args.page_limit {
                break;
            }
            if found.values().all(|v| v.len() >= 2) {
                break;
            }
            if page == 0 {
                break;
            }
            page -= 1;
        }

        for key in CLASS_KEYS {
            let names = found.get(key).expect("class key missing");
            if names.is_empty() {
                eprintln!("[{server}] Missing {key} targets in level range.");
                continue;
            }
            let player1 = names[0].clone();
            let player2 = names.get(1).cloned().unwrap_or_else(|| player1.clone());
            if let Some(entries) = class_map.get_mut(key) {
                entries.push(Entry {
                    server: server.clone(),
                    player1,
                    player2,
                });
            }
        }
    }

    for entries in class_map.values_mut() {
        entries.sort_by(|a, b| a.server.cmp(&b.server));
    }

    let json = serde_json::to_string_pretty(&class_map)?;
    fs::write(&args.output_path, json)?;
    eprintln!("Wrote {}", args.output_path.display());

    Ok(())
}

fn class_key(class: Class) -> Option<&'static str> {
    match class {
        Class::Warrior => Some("Warrior"),
        Class::Mage => Some("Mage"),
        Class::Scout => Some("Scout"),
        Class::Assassin => Some("Assassin"),
        Class::BattleMage => Some("Battle Mage"),
        Class::Berserker => Some("Berserker"),
        Class::DemonHunter => Some("Demon Hunter"),
        Class::Druid => Some("Druid"),
        Class::Bard => Some("Bard"),
        Class::Necromancer => Some("Necromancer"),
        Class::Paladin => Some("Paladin"),
        Class::PlagueDoctor => Some("Plague Doctor"),
        _ => None,
    }
}

fn parse_args(args: Vec<String>) -> Result<Args, Box<dyn Error>> {
    let mut config_path = None;
    let mut output_path = None;
    let mut min_level = DEFAULT_MIN_LEVEL;
    let mut max_level = DEFAULT_MAX_LEVEL;
    let mut page_limit = DEFAULT_PAGE_LIMIT;
    let mut servers: Option<HashSet<String>> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                config_path = args.get(i).map(PathBuf::from);
            }
            "--output" => {
                i += 1;
                output_path = args.get(i).map(PathBuf::from);
            }
            "--min-level" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    min_level = v.parse()?;
                }
            }
            "--max-level" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    max_level = v.parse()?;
                }
            }
            "--page-limit" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    page_limit = v.parse()?;
                }
            }
            "--servers" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    let set: HashSet<String> =
                        v.split(',').map(|s| s.trim().to_lowercase()).collect();
                    servers = Some(set);
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                return Err(format!("Unknown argument: {other}").into());
            }
        }
        i += 1;
    }

    let config_path = config_path.unwrap_or_else(|| PathBuf::from("charstoFightAcc.json"));
    let output_path = output_path.unwrap_or_else(|| PathBuf::from("charsToFight.json"));

    Ok(Args {
        config_path,
        output_path,
        min_level,
        max_level,
        page_limit,
        servers,
    })
}

fn print_usage() {
    eprintln!(
        "Usage: refresh_chars_to_fight [--config PATH] [--output PATH] \
        [--min-level N] [--max-level N] [--page-limit N] [--servers s1,s2]"
    );
}

fn read_accounts(path: &Path) -> Result<Vec<UserConfig>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    if let Ok(wrapper) = serde_json::from_str::<UserConfigFile>(&content) {
        return Ok(wrapper.accounts);
    }
    let direct = serde_json::from_str::<Vec<UserConfig>>(&content)?;
    Ok(direct)
}

async fn build_sessions(
    accounts: Vec<UserConfig>,
    allowed_servers: &Option<HashSet<String>>,
) -> Result<HashMap<String, SimpleSession>, Box<dyn Error>> {
    let mut sessions_by_server: HashMap<String, SimpleSession> = HashMap::new();
    let mut direct_accounts = Vec::new();

    for acc in accounts {
        let server = acc.server.trim().to_lowercase();
        if server.is_empty() {
            eprintln!("[SSO] Logging in as {}", acc.accname);
            match SimpleSession::login_sf_account(&acc.accname, &acc.password).await {
                Ok(sessions) => {
                    for session in sessions {
                        let server_host = server_from_session(&session)?;
                        if let Some(allowed) = allowed_servers {
                            if !allowed.contains(&server_host) {
                                continue;
                            }
                        }
                        if sessions_by_server.contains_key(&server_host) {
                            eprintln!("[{server_host}] Duplicate session, keeping existing");
                            continue;
                        }
                        sessions_by_server.insert(server_host, session);
                    }
                }
                Err(err) => {
                    eprintln!("[SSO] Login failed for {}: {err}", acc.accname);
                }
            }
        } else {
            direct_accounts.push(acc);
        }
    }

    for acc in direct_accounts {
        let server = acc.server.trim().to_lowercase();
        if let Some(allowed) = allowed_servers {
            if !allowed.contains(&server) {
                continue;
            }
        }
        if sessions_by_server.contains_key(&server) {
            continue;
        }

        eprintln!("[{server}] Logging in as {}", acc.accname);
        match SimpleSession::login(&acc.accname, &acc.password, &server).await {
            Ok(session) => {
                sessions_by_server.insert(server, session);
            }
            Err(err) => {
                eprintln!("[{server}] Login failed: {err}");
            }
        }
    }

    Ok(sessions_by_server)
}

fn server_from_session(session: &SimpleSession) -> Result<String, Box<dyn Error>> {
    let url = session.server_url();
    let host = url
        .host_str()
        .ok_or("Session server URL missing host")?
        .to_lowercase();
    Ok(host)
}
