use std::{cmp, collections::HashMap, error::Error, sync::Mutex};

use chrono::{DateTime, Local};
use enum_map::EnumMap;
use num_bigint::BigInt;
use once_cell::sync::Lazy;
use sf_api::{
    command::{Command, IdleUpgradeAmount},
    gamestate::idle::{IdleBuilding, IdleBuildingType},
    misc::EnumMapGet,
    SimpleSession,
};

use crate::{bot_runner::write_character_log, fetch_character_setting, fortress::sleep_between_commands};

static TOILET_CYCLE_TRACKER: Lazy<Mutex<HashMap<String, DateTime<Local>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn sanitize_cycle_end(cycle_end: DateTime<Local>) -> DateTime<Local>
{
    if cycle_end <= Local::now()
    {
        return Local::now() + chrono::Duration::seconds(30);
    }
    cycle_end
}

fn build_idle_cycle_key(session: &SimpleSession, character_name: &str, character_id: u32) -> String
{
    let server = session.server_url().host_str().unwrap_or("unknown").to_ascii_lowercase();
    format!("{}_{}_{}", server, character_name.to_ascii_lowercase(), character_id)
}

pub async fn play_idle_game(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    if gs.idle_game.is_none()
    {
        return Ok(String::from(""));
    }
    let mut sacrifice_ratio: i32 = fetch_character_setting(&gs, "arenaManagerSacrificeAfterPercent").unwrap_or(30);
    if sacrifice_ratio < 0
    {
        sacrifice_ratio = 30;
    }
    let idle_game = match gs.idle_game.clone()
    {
        Some(game) => game,
        None => return Ok(String::from("")),
    };
    let sacrifice_after_toilet_cycle: bool = fetch_character_setting(&gs, "arenaManagerSacrificeAfterToiletCycle").unwrap_or(false);
    let current_runes = idle_game.current_runes.clone();
    let base = BigInt::from(10);
    let exponent = 151;
    let ingame_max_rune_limit = base.pow(exponent);

    if current_runes >= ingame_max_rune_limit
    {
        return Ok("".to_string());
    }

    let runes_available_for_sacrifice = idle_game.sacrifice_runes;

    if sacrifice_after_toilet_cycle
    {
        if let Some(cycle_end) = idle_game.buildings[IdleBuildingType::Toilet].cycle_end
        {
            let key = build_idle_cycle_key(session, &gs.character.name, gs.character.player_id);
            let sanitized_next = sanitize_cycle_end(cycle_end);
            let should_sacrifice_now = {
                let mut tracker = TOILET_CYCLE_TRACKER.lock().unwrap();
                match tracker.get(&key).cloned()
                {
                    None =>
                    {
                        tracker.insert(key, sanitized_next);
                        write_character_log(
                            &gs.character.name,
                            gs.character.player_id,
                            &format!("TOILET_CYCLE: baseline set (cycle_end={})", sanitized_next.format("%H:%M:%S")),
                        );
                        false
                    }
                    Some(prev_next) =>
                    {
                        if cycle_end < prev_next && Local::now() < prev_next
                        {
                            tracker.insert(key, sanitized_next);
                            write_character_log(
                                &gs.character.name,
                                gs.character.player_id,
                                &format!("TOILET_CYCLE: updated cycle_end (cycle_end={})", sanitized_next.format("%H:%M:%S")),
                            );
                            false
                        }
                        else if Local::now() >= prev_next
                        {
                            tracker.insert(key, sanitized_next);
                            true
                        }
                        else
                        {
                            false
                        }
                    }
                }
            };

            if should_sacrifice_now
            {
                write_character_log(
                    &gs.character.name,
                    gs.character.player_id,
                    "TOILET_CYCLE: completed -> IdleSacrifice",
                );
                if let Err(_err) = session.send_command(Command::IdleSacrifice).await
                {
                    return Ok(String::from("Server Error when IdleSacrifice"));
                }
                let result = format!("IdleSacrifice after toilet cycle. Runes: {}", current_runes);
                write_character_log(&gs.character.name, gs.character.player_id, &format!("ARENA_MANAGER: {}", result));
                return Ok(String::from(result));
            }
        }
    }

    if sacrifice_ratio > 0 && should_sacrifice(&current_runes, &runes_available_for_sacrifice, sacrifice_ratio)
    {
        if let Err(err) = session.send_command(Command::IdleSacrifice).await
        {
            
            
            return Ok(String::from("Server Error when IdleSacrifice"));
        }
        let result = format!("IdleSacrifice complete. Runes: {}", current_runes);
        write_character_log(&gs.character.name, gs.character.player_id, &format!("ARENA_MANAGER: {}", result));
        return Ok(String::from(result));
    }

    
    let mut collectedUpgrades: HashMap<String, u64> = HashMap::new();
    let mut finalMessage = String::new();
    collectedUpgrades.insert("Seat".to_string(), 0);
    collectedUpgrades.insert("PopcornStand".to_string(), 0);
    collectedUpgrades.insert("ParkingLot".to_string(), 0);
    collectedUpgrades.insert("Trap".to_string(), 0);
    collectedUpgrades.insert("Drinks".to_string(), 0);
    collectedUpgrades.insert("DeadlyTrap".to_string(), 0);
    collectedUpgrades.insert("VIPSeat".to_string(), 0);
    collectedUpgrades.insert("Snacks".to_string(), 0);
    collectedUpgrades.insert("StrayingMonsters".to_string(), 0);
    collectedUpgrades.insert("Toilet".to_string(), 0);
    collectedUpgrades.insert("Error".to_string(), 0);
    collectedUpgrades.insert("Done".to_string(), 0);
    collectedUpgrades.insert("Locked".to_string(), 0);

    for i in 1..=50
    {
        let mut res: HashMap<String, u64> = play_idle_game_impl(session).await?;
        if (res.is_empty())
        {
            collectedUpgrades.insert("Locked".to_string(), 1);
            break;
        }
        else if (res.contains_key("Error"))
        {
            collectedUpgrades.insert("Error".to_string(), 1);
            break;
        }
        else if (res.contains_key("Done"))
        {
            collectedUpgrades.insert("Done".to_string(), 1);
            break;
        }
        else
        {
            
            let mut returnedBuilding = res.iter().next().unwrap();
            let mut buildingName = returnedBuilding.0;
            let mut buildingNumberIncrease = returnedBuilding.1;
            let mut alreadyUpgraded = collectedUpgrades.get(buildingName).unwrap();
            let mut totalUpgrade = alreadyUpgraded + buildingNumberIncrease;

            collectedUpgrades.insert(buildingName.to_string(), totalUpgrade);
            if (buildingNumberIncrease >= &20000)
            {
                break; 
                       
            }
        }
        sleep_between_commands(30).await;
    }

    let mut message_started = false;
    let building_entries = [
        ("Seat", "Seat"),
        ("PopcornStand", "PopcornStand"),
        ("ParkingLot", "ParkingLot"),
        ("Drinks", "Drinks"),
        ("DeadlyTrap", "DeadlyTrap"),
        ("VIPSeat", "VIPSeat"),
        ("Snacks", "Snacks"),
        ("StrayingMonsters", "StrayingMonsters"),
        ("Toilet", "Toilet"),
    ];

    for (key, label) in building_entries
    {
        let count = collectedUpgrades.get(key).unwrap_or(&0);
        if *count > 0
        {
            if !message_started
            {
                finalMessage = String::from("Arena manager buildings upgraded by -> ");
                message_started = true;
            }
            let display = if *count >= 20000 { "MAX".to_string() } else { count.to_string() };
            finalMessage += format!("\t{}: {}", label, display).as_str();
        }
    }

    if (collectedUpgrades.get("Error").unwrap() > &0)
    {
        if !message_started
        {
            finalMessage = String::from("Arena manager -> ");
            message_started = true;
        }
        finalMessage += "An error has occured while communicating with the server\n";
    }
    if (collectedUpgrades.get("Done").unwrap() > &0)
    {
        let buildingType = get_building_type(collectedUpgrades.get("Done").unwrap());
        let buildingName = get_building_name(buildingType);
        
    }
    if (collectedUpgrades.get("Locked").unwrap() > &0)
    {
        if !message_started
        {
            finalMessage = String::from("Arena manager -> ");
            message_started = true;
        }
    }
    if message_started
    {
        finalMessage += "}";
        write_character_log(&gs.character.name, gs.character.player_id, &format!("ARENA_MANAGER: {}", finalMessage));
        return Ok(finalMessage);
    }
    Ok(String::from(""))
}

pub async fn play_idle_game_impl(session: &mut SimpleSession) -> Result<HashMap<String, u64>, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;

    let idle_game = match &gs.idle_game
    {
        Some(game) => game,
        None => return Ok(HashMap::new()),
    };

    let buildings = &idle_game.buildings;

    let mut building_levels: HashMap<IdleBuildingType, u32> = HashMap::new();

    for (building_type, building_data) in buildings
    {
        building_levels.insert(building_type, building_data.level);
    }

    let (building, mut level_diff) = get_next_building_to_upgrade(&building_levels, buildings[IdleBuildingType::Toilet].level, buildings);
    let mut result: HashMap<String, u64> = HashMap::new();
    
    
    if (level_diff == 20000)
    {
        if (buildings[building].upgrade_cost < idle_game.current_money)
        {
            let upgradex_once = Command::IdleUpgrade {
                typ: building,
                amount: IdleUpgradeAmount::Max,
            };

            if let Err(err) = session.send_command(upgradex_once).await
            {
                result.insert(String::from("Error"), 20000);
                return (Ok(result));
            }
            result.insert(get_building_name(building), 20000);
            return (Ok(result));
        }
        else
        {
            result.insert(String::from("Done"), get_building_id(building) as u64);
            return (Ok(result));
        }
    }

    if (level_diff >= 100 && buildings[building].upgrade_cost_100x < idle_game.current_money)
    {
        let upgradex_hundredCommand = Command::IdleUpgrade {
            typ: building,
            amount: IdleUpgradeAmount::Hundred,
        };
        if let Err(err) = session.send_command(upgradex_hundredCommand).await
        {
            result.insert(String::from("Error"), 100);
            return (Ok(result));
        }
        result.insert(get_building_name(building), 100);
        return (Ok(result));
    }

    if (level_diff >= 25 && buildings[building].upgrade_cost_25x < idle_game.current_money)
    {
        
        
        let upgradex_twentyfive = Command::IdleUpgrade {
            typ: building,
            amount: IdleUpgradeAmount::TwentyFive,
        };
        if let Err(err) = session.send_command(upgradex_twentyfive).await
        {
            result.insert(String::from("Error"), 25);
            return (Ok(result));
        }
        result.insert(get_building_name(building), 25);
        return (Ok(result));
    }

    if (level_diff >= 10 && buildings[building].upgrade_cost_10x < idle_game.current_money)
    {
        let upgradex_ten = Command::IdleUpgrade {
            typ: building,
            amount: IdleUpgradeAmount::Ten,
        };
        if let Err(err) = session.send_command(upgradex_ten).await
        {
            result.insert(String::from("Error"), 10);
            return (Ok(result));
        }
        result.insert(get_building_name(building), 10);
        return (Ok(result));
    }

    if (level_diff >= 1 && buildings[building].upgrade_cost < idle_game.current_money)
    {
        
        
        
        
        let upgradex_once = Command::IdleUpgrade {
            typ: building,
            amount: IdleUpgradeAmount::One,
        };

        if let Err(err) = session.send_command(upgradex_once).await
        {
            result.insert(String::from("Error"), 1);
            return (Ok(result));
        }
        result.insert(get_building_name(building), 1);
        return (Ok(result));
    }
    else
    {
        result.insert(String::from("Done"), get_building_id(building) as u64);
        return (Ok(result));
    }
}

fn should_sacrifice(current_runes: &BigInt, runes_available_for_sacrifice: &BigInt, percentage: i32) -> bool
{
    let target_runes = (current_runes * BigInt::from(percentage)) / BigInt::from(100);

    
    if current_runes < &BigInt::from(20)
    {
        return runes_available_for_sacrifice >= &BigInt::from(20);
    }

    runes_available_for_sacrifice >= &target_runes
}

pub fn get_next_building_to_upgrade(building_levels: &HashMap<IdleBuildingType, u32>, number: u32, buildings: &EnumMap<IdleBuildingType, IdleBuilding>) -> (IdleBuildingType, u32)
{
    
    let start_list_string = "1:50,2:25,1:100,2:50,1:200,2:100,3:25,1:250,2:175,3:100,4:25,2:250,1:325,4:50,3:175,4:100,5:25,3:250,1:400,2:325,5:50,4:175,5:100,1:500,4:250,6:25,2:500,3:325,6:50,1:575,3:400,4:325,5:175,6:100,7:25,5:250,3:500,7:50,4:400,5:325,6:175,7:100,8:25,6:250,4:500,1:650,2:575,\
                             5:400,6:325,7:175,8:100,9:25,7:250,5:500,1:725,2:650,3:575,6:400,7:325,8:175,9:100,10:25,8:250,6:500,10:50,1:800,2:725,3:650,4:575,7:400,8:325,9:175,10:100,9:250,7:500,10:125,1:1000,10:175,2:925,3:850,4:775,5:700,8:400,9:325,10:250,8:500,1:1075,2:1000,3:875,4:800,
                             \
                             5:725,6:550,9:400,10:325,3:1000,9:500,1:1150,2:1075,4:875,5:800,6:725,7:650,6:575,5:550,10:500,1:1225,2:1150,3:1075,4:1000,5:1000,6:1000,10:575,9:650,8:725,1:1300,2:1225,3:1150,4:1075,7:1000,10:650,9:725,8:800,1:1375,2:1300,3:1225,4:1150,5:1075,8:1000,10:725,9:800,1:\
                             1450,2:1375,3:1300,4:1225,4:1150,5:1075,9:1000,10:800,1:1525,2:1450,3:1375,4:1300,5:1225,6:1150,7:1075,10:1000";
    let start_list = parse_list(start_list_string);

    
    let second_list_string = "1:2,2:1,3:1,4:1,5:1,6:1,7:1,8:1,9:1,10:1,1:1000,2:1000,3:1000,4:1000,5:1000,6:1000,7:1000,8:1000,9:1000,10:1000,10:1500,9:1575,8:1650,7:1725,9:1800,5:1875,4:1950,3:2025,2:2100,1:2500,10:1600,9:1675,8:1750,7:1825,6:1900,5:1975,4:2050,3:2125,2:2500,10:1700,9:1775,8:\
                              1850,7:1925,6:2000,5:2075,4:2150,3:2500,10:1800,9:1875,8:1950,7:2025,6:2100,5:2175,4:2500,10:1900,9:1975,8:2050,7:2125,6:2200,5:2500,10:2000,9:2075,8:2150,7:2225,6:2500,10:2100,9:2175,8:2250,7:2500,10:2200,9:2275,8:2500,10:2300,9:2500,10:2500";
    let second_list = parse_list(second_list_string);

    
    let third_list_string = "1:2,2:1,3:1,4:1,5:1,6:1,7:1,8:1,9:1,10:1,1:3500,2:3500,3:3500,4:3500,5:3500,6:3500,7:3500,8:3500,9:3500,10:3500,10:4000,9:4075,8:4150,7:4225,9:4300,5:4375,4:4450,3:4525,2:4600,1:5000,10:4100,9:4175,8:4250,7:4325,6:4400,5:4475,4:4550,3:4625,2:5000,10:4200,9:4275,8:4350,\
                             7:4425,6:4500,5:4575,4:4650,3:5000,10:4300,9:4375,8:4450,7:4525,6:4600,5:4675,4:5000,10:4400,9:4475,8:4550,7:4625,6:4700,5:5000,10:4500,9:4575,8:4650,7:4725,6:5000,10:4600,9:4675,8:4750,7:5000,10:4700,9:4775,8:5000,10:4800,9:5000,10:5000";
    let third_list = parse_list(third_list_string);

    
    let fourth_list_string = "1:2,2:1,3:1,4:1,5:1,6:1,7:1,8:1,9:1,10:1,1:8500,2:8500,3:8500,4:8500,5:8500,6:8500,7:8500,8:8500,9:8500,10:8500,10:9000,9:9075,8:9150,7:9225,9:9300,5:9375,4:9450,3:9525,2:9600,1:10000,10:9100,9:9175,8:9250,7:9325,6:9400,5:9475,4:9550,3:9625,2:10000,10:9200,9:9275,8:\
                              9350,7:9425,6:9500,5:9575,4:9650,3:10000,10:9300,9:9375,8:9450,7:9525,6:9600,5:9675,4:10000,10:9400,9:9475,8:9550,7:9625,6:9700,5:10000,10:9500,9:9575,8:9650,7:9725,6:10000,10:9600,9:9675,8:9750,7:10000,10:9700,9:9775,8:10000,10:9800,9:10000,10:10000";
    let fourth_list = parse_list(fourth_list_string);

    let end_list_string = "11:999999";
    let end_list = parse_list(end_list_string);
    
    let attraction_list = if number < 1000
    {
        start_list
    }
    else if number < 2500
    {
        second_list
    }
    else if number < 5000
    {
        third_list
    }
    else if number < 10000
    {
        fourth_list
    }
    else
    {
        
        end_list
    };

    if (attraction_list == parse_list(end_list_string))
    {
        
        
        
        let toiletLvl = buildings.get(IdleBuildingType::Toilet).level;
        let strayingMonsterLvl = buildings.get(IdleBuildingType::StrayingMonsters).level;
        let snacksLvl = buildings.get(IdleBuildingType::Snacks).level;
        let vipSeatLvl = buildings.get(IdleBuildingType::VIPSeat).level;
        let trapLvl = buildings.get(IdleBuildingType::DeadlyTrap).level;
        let min_level = cmp::min(cmp::min(toiletLvl, strayingMonsterLvl), cmp::min(cmp::min(snacksLvl, vipSeatLvl), trapLvl));
        return if toiletLvl == min_level
        {
            (IdleBuildingType::Toilet, 20000)
        }
        else if strayingMonsterLvl == min_level
        {
            (IdleBuildingType::StrayingMonsters, 20000)
        }
        else if snacksLvl == min_level
        {
            (IdleBuildingType::Snacks, 20000)
        }
        else if vipSeatLvl == min_level
        {
            (IdleBuildingType::VIPSeat, 20000)
        }
        else if trapLvl == min_level
        {
            (IdleBuildingType::DeadlyTrap, 20000)
        }
        else
        {
            (IdleBuildingType::Toilet, 20000)
        };
    }

    let mut max_diff = 0;
    let mut next_building_type = IdleBuildingType::Seat;

    for (building, level) in attraction_list
    {
        let building_type = match building
        {
            1 => IdleBuildingType::Seat,
            2 => IdleBuildingType::PopcornStand,
            3 => IdleBuildingType::ParkingLot,
            4 => IdleBuildingType::Trap,
            5 => IdleBuildingType::Drinks,
            6 => IdleBuildingType::DeadlyTrap,
            7 => IdleBuildingType::VIPSeat,
            8 => IdleBuildingType::Snacks,
            9 => IdleBuildingType::StrayingMonsters,
            10 => IdleBuildingType::Toilet,
            _ => continue,
        };

        let current_level = *building_levels.get(&building_type).unwrap_or(&0);
        let mut diff = (level as i32 - current_level as i32);
        if diff > 0
        {
            return (building_type, diff as u32);
        }

        if diff > max_diff
        {
            max_diff = diff;
            next_building_type = building_type;
        }
    }
    (next_building_type, max_diff as u32)
}

pub fn parse_list(input: &str) -> Vec<(u32, u32)>
{
    input
        .split(',')
        .filter_map(|entry| {
            let mut parts = entry.split(':');
            let building = parts.next()?.parse::<u32>().ok()?;
            let breakpoint = parts.next()?.parse::<u32>().ok()?;
            Some((building, breakpoint))
        })
        .collect()
}

pub fn get_building_id(building: IdleBuildingType) -> u32
{
    match building
    {
        IdleBuildingType::Seat => 1,
        IdleBuildingType::PopcornStand => 2,
        IdleBuildingType::ParkingLot => 3,
        IdleBuildingType::Trap => 4,
        IdleBuildingType::Drinks => 5,
        IdleBuildingType::DeadlyTrap => 6,
        IdleBuildingType::VIPSeat => 7,
        IdleBuildingType::Snacks => 8,
        IdleBuildingType::StrayingMonsters => 9,
        IdleBuildingType::Toilet => 10,
    }
}

pub fn get_building_type(id: &u64) -> IdleBuildingType
{
    match id
    {
        1 => IdleBuildingType::Seat,
        2 => IdleBuildingType::PopcornStand,
        3 => IdleBuildingType::ParkingLot,
        4 => IdleBuildingType::Trap,
        5 => IdleBuildingType::Drinks,
        6 => IdleBuildingType::DeadlyTrap,
        7 => IdleBuildingType::VIPSeat,
        8 => IdleBuildingType::Snacks,
        9 => IdleBuildingType::StrayingMonsters,
        10 => IdleBuildingType::Toilet,
        _ => IdleBuildingType::StrayingMonsters,
    }
}

pub fn get_building_name(building: IdleBuildingType) -> String
{
    match building
    {
        IdleBuildingType::Seat => "Seat".to_string(),
        IdleBuildingType::PopcornStand => "PopcornStand".to_string(),
        IdleBuildingType::ParkingLot => "ParkingLot".to_string(),
        IdleBuildingType::Trap => "Trap".to_string(),
        IdleBuildingType::Drinks => "Drinks".to_string(),
        IdleBuildingType::DeadlyTrap => "DeadlyTrap".to_string(),
        IdleBuildingType::VIPSeat => "VIPSeat".to_string(),
        IdleBuildingType::Snacks => "Snacks".to_string(),
        IdleBuildingType::StrayingMonsters => "StrayingMonsters".to_string(),
        IdleBuildingType::Toilet => "Toilet".to_string(),
    }
}
