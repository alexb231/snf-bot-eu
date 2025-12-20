#![allow(warnings)]

use std::{borrow::Borrow, fmt::Debug, time::Duration};

use chrono::{DateTime, Local, TimeDelta};
use enum_map::EnumMap;
use sf_api::{
    command::Command,
    error::SFError,
    gamestate::{
        fortress::{
            Fortress, FortressBuildingType, FortressResourceType,
            FortressResourceType::{Experience, Stone, Wood},
            FortressUnitType,
        },
        GameState,
    },
    SimpleSession,
};
use tokio::time::sleep;

use crate::{
    bot_runner::write_character_log,
    fetch_character_setting,
    utils::check_time_in_range,
};

pub async fn sleep_between_commands(ms: u64) { sleep(Duration::from_millis(ms)).await; }

pub async fn collect_fortress_resources(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gamestate = session.send_command(Command::Update).await?.clone();
    let fortress_option = gamestate.fortress.clone();
    if (!check_if_fortress_available(session).await)
    {
        // noch nicht freigeschaltet weil unter lvl 25
        return Ok(String::from(""));
    }
    let collect_wood: bool = fetch_character_setting(&gamestate, "collectWood").unwrap_or(false);
    let collect_stone: bool = fetch_character_setting(&gamestate, "collectStone").unwrap_or(false);
    let collect_exp: bool = fetch_character_setting(&gamestate, "collectExp").unwrap_or(false);
    let collect_resources_from: String = fetch_character_setting(&gamestate, "fortressCollectTimeFrom").unwrap_or("00:00".to_string());
    let collect_resources_to: String = fetch_character_setting(&gamestate, "fortressCollectTimeTo").unwrap_or("00:00".to_string());
    let is_in_range = check_time_in_range(collect_resources_from, collect_resources_to);

    let mut result = String::from("");
    let mut exp_available = false;
    let mut wood_available = false;
    let mut stone_available = false;

    if (!is_in_range)
    {
        return Ok(result);
    }

    if check_if_building_is_available(&gamestate, FortressBuildingType::Academy)
    {
        exp_available = true;
    }
    if check_if_building_is_available(&gamestate, FortressBuildingType::WoodcuttersHut)
    {
        wood_available = true;
    }
    if check_if_building_is_available(&gamestate, FortressBuildingType::Quarry)
    {
        stone_available = true;
    }

    let fortress = match fortress_option
    {
        Some(fortress_option) => fortress_option,
        None => return Ok(String::from("")), // hat keinen fortress
    };

    fn duration_since(last_updated: DateTime<Local>) -> TimeDelta
    {
        let now: DateTime<Local> = Local::now();
        now.signed_duration_since(last_updated)
    }

    let buildings = fortress.buildings;
    let mut collected_entries: Vec<String> = Vec::new();

    if stone_available && collect_stone
    {
        let collectable = fortress.resources[Stone].production.last_collectable;
        let _ = session.send_command(Command::FortressGather { resource: Stone }).await?;
        result += "Stone ";
        collected_entries.push(format!("Stone: {}", collectable));
    }
    if wood_available && collect_wood
    {
        let collectable = fortress.resources[Wood].production.last_collectable;
        let _ = session.send_command(Command::FortressGather { resource: Wood }).await?;
        result += "Wood ";
        collected_entries.push(format!("Wood: {}", collectable));
    }
    if exp_available && collect_exp
    {
        let collectable = fortress.resources[Experience].production.last_collectable;
        let _ = session.send_command(Command::FortressGather { resource: Experience }).await?;
        result += "Experience ";
        collected_entries.push(format!("Experience: {}", collectable));
    }

    let mut finalMessage = String::from("");
    if (result != "")
    {
        finalMessage += "Collected fortress resources: ";
        finalMessage += &result;
    }
    if !collected_entries.is_empty()
    {
        write_character_log(
            &gamestate.character.name,
            gamestate.character.player_id,
            &format!("FORTRESS: Collected {}", collected_entries.join(", ")),
        );
    }
    return Ok(String::from(""));
}

async fn check_if_fortress_available(session: &mut SimpleSession) -> bool
{
    let gamestate = match session.send_command(Command::Update).await
    {
        Ok(state) => state,
        Err(_) =>
        {
            return false;
        }
    };

    match gamestate.fortress
    {
        Some(_) if gamestate.character.level >= 25 => true,
        _ => false,
    }
}

pub fn check_if_building_is_available(gs: &GameState, building: FortressBuildingType) -> bool
{
    if let Some(fortress) = gs.fortress.clone()
    {
        let building_data = &fortress.buildings[building];
        let currently_building = fortress.building_upgrade.target;
        if (currently_building == Option::from(building))
        {
            return false;
        }

        if building_data.level > 0
        {
            return true;
        }
    }

    false
}

pub async fn train_fortress_units(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gamestate = session.send_command(Command::Update).await?.clone();
    let train_soldiers: bool = fetch_character_setting(&gamestate, "fortessTrainSoldiers").unwrap_or(false);
    let train_archers: bool = fetch_character_setting(&gamestate, "fortessTrainArchers").unwrap_or(false);
    let train_mages: bool = fetch_character_setting(&gamestate, "fortessTrainMages").unwrap_or(false);

    let fortress_option = gamestate.fortress.clone();
    if !check_if_fortress_available(session).await
    {
        return Ok(String::from(""));
    }

    let fortress = match fortress_option
    {
        Some(f) => f,
        None =>
        {
            return Ok(String::from(""));
        }
    };

    if (check_whether_to_pause_unit_training(fortress.clone()))
    {
        return Ok(String::from(""));
    }

    let units = fortress.units.clone(); // Clone the units to avoid holding a borrow on `session`
    let current_resources = fortress.resources.clone(); // Clone resources for use in the loop

    for (key, value) in units
    {
        let current_unit_name = match key
        {
            FortressUnitType::Soldier => "Soldier",
            FortressUnitType::Archer => "Archer",
            _ => "Magician",
        };

        if (current_unit_name == "Soldier" && !train_soldiers)
        {
            continue;
        }

        if (current_unit_name == "Archer" && !train_archers)
        {
            continue;
        }

        if (current_unit_name == "Magician" && !train_mages)
        {
            continue;
        }

        let count = value.count;
        let limit = value.limit;
        if count >= limit
        {
            continue;
        }

        let in_training = value.in_training;
        if count + in_training >= limit
        {
            continue;
        }

        // Check if there are enough resources
        let training_info = &value.training;
        let upgrade_cost = &training_info.cost;
        let upgrade_wood = upgrade_cost.wood;
        let upgrade_stone = upgrade_cost.stone;

        let current_wood = current_resources[FortressResourceType::Wood].current;
        let current_stone = current_resources[FortressResourceType::Stone].current;

        if current_wood < upgrade_wood || current_stone < upgrade_stone
        {
            continue;
        }

        // Calculate the maximum amount to train based on resources and limits
        let maximum_amount_to_train = (limit - (count + in_training)) as u64;
        let mut amount_to_train = 0;
        for i in 1..=maximum_amount_to_train
        {
            if current_wood >= upgrade_wood * i && current_stone >= upgrade_stone * i
            {
                amount_to_train = i;
            }
            else
            {
                break;
            }
        }

        if amount_to_train > 0
        {
            // Send the command to train units
            session.send_command(Command::FortressBuildUnit { unit: key, count: amount_to_train as u32 }).await?;
            write_character_log(
                &gamestate.character.name,
                gamestate.character.player_id,
                &format!("FORTRESS: Training {} x {}", amount_to_train, current_unit_name),
            );
        }
    }

    Ok(String::from(""))
}
pub async fn start_searching_for_gem(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    // inventory management -> hab ich genug geld um den minimal spargebrag zu
    // erhalten
    // TODO add time when to stop searching like an overnight pause

    let gamestate = session.send_command(Command::Update).await?;
    let gold_to_keep: i64 = fetch_character_setting(&gamestate, "itemsInventoryMinGoldSaved").unwrap_or(0) * 100;
    let mut character_silver = gamestate.character.silver;
    let ignore_min_gold: bool = fetch_character_setting(&gamestate, "itemsInventoryMinGoldSavedIgnoreGemMine").unwrap_or(false);
    if (!ignore_min_gold)
    {
        if (gold_to_keep > character_silver as i64)
        {
            character_silver = 0;
        }
        else
        {
            character_silver = (character_silver as i64 - gold_to_keep) as u64;
        }
    }

    let fortress_option = gamestate.fortress.clone();

    let fortress = match fortress_option
    {
        Some(f) => f,
        None =>
        {
            return Ok(String::from(""));
        }
    };
    if fortress.buildings[FortressBuildingType::GemMine].level == 0
    {
        return Ok(String::from(""));
    }

    let gem_search_info = fortress.gem_search;

    if let Some(finish_time) = gem_search_info.finish
    {
        if (Local::now() > finish_time)
        {
            if (gamestate.character.inventory.count_free_slots() <= 0)
            {
                return Ok(String::from(""));
            }

            let result = session.send_command(Command::FortressGemStoneSearchFinish { mushrooms: 0 }).await;

            match result
            {
                Ok(_) => Ok(String::from("Gem has been collected")),
                Err(SFError::ServerError(msg)) if msg == "need a free slot" => Ok(String::from("")),
                Err(e) => Ok(String::from("Unexpected error during gem collection")),
            }
        }
        else
        {
            Ok(String::from(""))
        }
    }
    else
    {
        if (check_whether_to_pause_gem_search(fortress.clone()))
        {
            return Ok(String::from(""));
        }

        let search_silver_cost = gem_search_info.cost.silver;
        if (search_silver_cost > character_silver)
        {
            return Ok(String::from(""));
            return Ok(String::from("Not enough gold for a gem search"));
        }
        let search_wood_cost = gem_search_info.cost.wood;
        let fortress_wood = &fortress.resources[FortressResourceType::Wood].current.clone();
        if (search_wood_cost > *fortress_wood)
        {
            return Ok(String::from(""));
            return Ok(String::from("Not enough wood for a gem search"));
        }
        let seach_stone_cost = gem_search_info.cost.stone;
        let fortress_stone = &fortress.resources[Stone].current.clone();
        if (seach_stone_cost > *fortress_stone)
        {
            return Ok(String::from(""));
            return Ok(String::from("Not enough stone for a gem search"));
        }

        let _ = &session.send_command(Command::FortressGemStoneSearch).await;
        Ok(String::from("Gem search started"))
    }
}

pub async fn check_if_enough_resources(session: &mut SimpleSession, building: FortressBuildingType) -> bool
{
    let gamestate = match session.send_command(Command::Update).await
    {
        Ok(state) => state,
        Err(err) =>
        {
            eprintln!("Failed to update game state: {:?}", err);
            return false;
        }
    };

    let fortress = match gamestate.fortress.as_ref()
    {
        Some(f) => f,
        None =>
        {
            return false;
        }
    };

    let building_data = &fortress.buildings[building];

    let needed_wood = building_data.upgrade_cost.wood;
    let needed_stone = building_data.upgrade_cost.stone;
    let needed_silver = building_data.upgrade_cost.silver;

    let current_wood = fortress.resources[FortressResourceType::Wood].current;
    let current_stone = fortress.resources[FortressResourceType::Stone].current;
    let current_silver = gamestate.character.silver;

    if current_wood < needed_wood
    {
        return false;
    }
    if current_stone < needed_stone
    {
        return false;
    }
    if current_silver < needed_silver
    {
        return false;
    }

    true
}

async fn reroll_fortress_opponnent(session: &mut SimpleSession, fortress: &Fortress) -> Result<bool, Box<dyn std::error::Error>>
{
    if let Some(attack_free_reroll) = fortress.attack_free_reroll
    {
        if Local::now() >= attack_free_reroll
        {
            session.send_command(Command::FortressNewEnemy { use_mushroom: false }).await?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn attack_fortress(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut gs = match session.send_command(Command::Update).await
    {
        Ok(gs) => gs.clone(),
        Err(err) =>
        {
            eprintln!("Error updating game state: {:?}", err);
            return Ok(String::from(""));
        }
    };

    let force_minimum_attack_setting: String = fetch_character_setting(&gs, "fortressAttackMode").unwrap_or("".to_string());
    let attack_multiplier: f64 = fetch_character_setting(&gs, "fortressAdditionalSoldierPercent").unwrap_or(0.0);

    let force_minimum_attack = force_minimum_attack_setting == "fortressAttackOneSoliderAttackOnly";
    let fortress = match &gs.fortress
    {
        Some(fortress) => fortress,
        None =>
        {
            return Ok(String::from(""));
        }
    };
    let available_soldiers = fortress.units[FortressUnitType::Soldier].count;

    let target = match &fortress.attack_target
    {
        Some(target) => target,
        None =>
        {
            return Ok(String::from(""));
        }
    };

    if *target == 0
    {
        if reroll_fortress_opponnent(session, fortress).await?
        {
            return Ok("".to_string());
        }
    }

    gs.lookup.reset_lookups();

    let new_gs = match session.send_command(Command::ViewPlayer { ident: target.to_string() }).await
    {
        Ok(new_gs) => new_gs.clone(),
        Err(err) =>
        {
            eprintln!("Error fetching player data for target {}: {:?} {}", target, err, gs.character.name);
            return Ok("".to_string());
        }
    };

    let player = match new_gs.lookup.lookup_pid(*target)
    {
        Some(player) => player,
        None =>
        {
            return Ok("".to_string());
        }
    };

    let soldier_advice = match player.soldier_advice
    {
        Some(advice) => advice,
        None =>
        {
            if reroll_fortress_opponnent(session, fortress).await?
            {
                return Ok("".to_string());
            }
            return Ok("".to_string());
        }
    };

    let max_soldiers = fortress.units[FortressUnitType::Soldier].limit;
    let available_soldiers = fortress.units[FortressUnitType::Soldier].count;

    if force_minimum_attack && soldier_advice != 1
    {
        if reroll_fortress_opponnent(session, fortress).await?
        {
            return Ok("".to_string());
        }
        return Ok("".to_string());
    }

    if soldier_advice as u16 > max_soldiers
    {
        if reroll_fortress_opponnent(session, fortress).await?
        {
            return Ok("".to_string());
        }
        return Ok("".to_string());
    }

    if available_soldiers < (soldier_advice as u16) / 2
    {
        if reroll_fortress_opponnent(session, fortress).await?
        {
            return Ok("".to_string());
        }
        return Ok("".to_string());
    }

    let mut result = "".to_string();
    let multiplier = 1.0 + attack_multiplier as f64 / 100.0;
    let boosted_advice = (soldier_advice as f64 * multiplier).ceil() as u16;
    let final_attack_count = boosted_advice.min(available_soldiers).min(max_soldiers);

    if final_attack_count >= soldier_advice as u16
    {
        session.send_command(Command::FortressAttack { soldiers: final_attack_count as u32 }).await?;
        write_character_log(
            &gs.character.name,
            gs.character.player_id,
            &format!("FORTRESS: Attack sent with {} soldiers (target: {})", final_attack_count, target),
        );
        result = format!("Fortress attack sent with {} soldiers!", final_attack_count);
    }

    Ok(result)
}

pub async fn build_fortress_our_order(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    if let Some(fortress) = &gs.fortress
    {
        if let Some(currently_building) = &fortress.building_upgrade.target
        {
            if let Some(upgrade_finish) = fortress.building_upgrade.finish
            {
                if upgrade_finish <= Local::now()
                {
                    session.send_command(Command::FortressBuildFinish { f_type: currently_building.clone(), mushrooms: 0 }).await?;
                }
            }
            return Ok("".to_string());
        }

        if (check_whether_to_pause_building_fortress_barracks(fortress.clone()))
        {
            return Ok("".to_string());
        }

        if (check_whether_to_pause_building_fortress_gem_mine(fortress.clone()))
        {
            return Ok("".to_string());
        }

        if let Some(building_to_upgrade) = find_next_building(fortress, true)
        {
            let current_amount_wood = fortress.resources[Wood].current;
            let current_amount_stone = fortress.resources[Stone].current;
            let character_silver = gs.character.silver;

            if building_to_upgrade == FortressBuildingType::FortressGroupBonusUpgrade
            {
                let hall_cost = fortress.hall_of_knights_upgrade_price;
                let has_enough_resources = current_amount_wood >= hall_cost.wood
                    && current_amount_stone >= hall_cost.stone
                    && character_silver >= hall_cost.silver;

                if has_enough_resources
                {
                    match session.send_command(Command::FortressUpgradeHallOfKnights).await
                    {
                        Ok(_) =>
                        {
                            write_character_log(
                                &gs.character.name,
                                gs.character.player_id,
                                "FORTRESS: Upgrading Hall of Knights",
                            );
                            return Ok("Upgrading Hall of Knights".to_string());
                        }
                        Err(SFError::ServerError(msg)) if is_fortress_resource_error(&msg) => return Ok("".to_string()),
                        Err(err) => return Err(err.into()),
                    }
                }
            }
            else if fortress.can_build(building_to_upgrade, character_silver)
            {
                match session.send_command(Command::FortressBuild { f_type: building_to_upgrade }).await
                {
                    Ok(_) =>
                    {
                        let building_name = get_building_name(building_to_upgrade);
                        write_character_log(
                            &gs.character.name,
                            gs.character.player_id,
                            &format!("FORTRESS: Upgrading {}", building_name),
                        );
                        return Ok(format!("Upgrading: {}", building_name).to_string());
                    }
                    Err(SFError::ServerError(msg)) if is_fortress_resource_error(&msg) => return Ok("".to_string()),
                    Err(err) => return Err(err.into()),
                }
            }
        }
    }

    Ok("".to_string())
}

fn is_fortress_resource_error(msg: &str) -> bool
{
    let msg = msg.to_ascii_lowercase();
    msg.contains("need more wood")
        || msg.contains("need more stone")
        || msg.contains("need more silver")
        || msg.contains("not enough wood")
        || msg.contains("not enough stone")
        || msg.contains("not enough silver")
}

fn get_building_name(p0: FortressBuildingType) -> String
{
    match p0
    {
        FortressBuildingType::Fortress => return "Fortress".to_string(),
        FortressBuildingType::LaborersQuarters => return "LaborersQuarters".to_string(),
        FortressBuildingType::WoodcuttersHut => return "WoodcuttersHut".to_string(),
        FortressBuildingType::Quarry => return "Quarry".to_string(),
        FortressBuildingType::GemMine => return "GemMine".to_string(),
        FortressBuildingType::Academy => return "Academy".to_string(),
        FortressBuildingType::ArcheryGuild => return "ArcheryGuild".to_string(),
        FortressBuildingType::Barracks => return "Barracks".to_string(),
        FortressBuildingType::MagesTower => return "MagesTower".to_string(),
        FortressBuildingType::Treasury => return "Treasury".to_string(),
        FortressBuildingType::Smithy => return "Smithy".to_string(),
        FortressBuildingType::Wall => return "Wall".to_string(),
        FortressBuildingType::FortressGroupBonusUpgrade => return "FortressGroupBonusUpgrade".to_string(),
    }
}

fn find_next_building(fortress: &Fortress, should_print: bool) -> Option<FortressBuildingType>
{
    let mut building_counts: EnumMap<FortressBuildingType, usize> = EnumMap::default();

    let build_order = create_fortress_building_order_fixed_list();

    for building_type in build_order
    {
        building_counts[building_type] += 1;

        let mut level = 0;
        let current_building = &fortress.buildings[building_type];
        level = current_building.level;
        if (building_type == FortressBuildingType::FortressGroupBonusUpgrade)
        {
            level = fortress.hall_of_knights_level;
        }

        if level < building_counts[building_type] as u16
        {
            return Some(building_type);
        }
    }
    None
}

pub fn create_fortress_building_order_fixed_list() -> Vec<FortressBuildingType>
{
    let mut fortress_buildings: Vec<FortressBuildingType> = Vec::new();

    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::LaborersQuarters);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::Fortress);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::FortressGroupBonusUpgrade);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Academy);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::Treasury);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::GemMine);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Barracks);
    fortress_buildings.push(FortressBuildingType::MagesTower);
    fortress_buildings.push(FortressBuildingType::ArcheryGuild);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);
    fortress_buildings.push(FortressBuildingType::Wall);
    fortress_buildings.push(FortressBuildingType::Smithy);
    fortress_buildings.push(FortressBuildingType::WoodcuttersHut);
    fortress_buildings.push(FortressBuildingType::Quarry);

    fortress_buildings
}

pub fn check_whether_to_pause_building_fortress_barracks(fortress: Fortress) -> bool
{
    find_next_building(&fortress, false) == Some(FortressBuildingType::Barracks) && (fortress.units[FortressUnitType::Soldier].in_training > 0 || fortress.units[FortressUnitType::Archer].in_training > 0 || fortress.units[FortressUnitType::Magician].in_training > 0)
}

pub fn check_whether_to_pause_building_fortress_gem_mine(fortress: Fortress) -> bool
{
    //
    return find_next_building(&fortress.clone(), false) == Option::from(FortressBuildingType::GemMine) && fortress.gem_search.start.is_some() && fortress.buildings[FortressBuildingType::GemMine].level < 25;
}

pub fn check_whether_to_pause_gem_search(fortress: Fortress) -> bool
{
    if (fortress.buildings[FortressBuildingType::GemMine].level >= 25)
    {
        return false;
    }
    return fortress.building_upgrade.target == Option::from(FortressBuildingType::GemMine) || find_next_building(&fortress.clone(), false) == Option::from(FortressBuildingType::GemMine) && fortress.buildings[FortressBuildingType::GemMine].level < 25;
}

pub fn check_whether_to_pause_unit_training(fortress: Fortress) -> bool
{
    //
    return find_next_building(&fortress.clone(), false) == Option::from(FortressBuildingType::Barracks);
}
