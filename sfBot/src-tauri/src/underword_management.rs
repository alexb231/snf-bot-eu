use std::fmt::Debug;

use chrono::{DateTime, Local, NaiveTime, TimeDelta};
use enum_map::EnumMap;
use sf_api::{
    command::Command,
    gamestate::underworld::{Underworld, UnderworldBuildingType, UnderworldResourceType, UnderworldUnitType},
    SimpleSession,
};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use crate::{
    fetch_character_setting,
    utils::{check_time_in_range, shitty_print},
};

pub async fn perform_underworld_atk_suggested_enemy(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    if gs.underworld.is_none()
    {
        return Ok(String::from(""));
    }

    let underworld = match gs.underworld
    {
        None => return Ok(String::from("")),
        Some(uw) => uw,
    };

    let underworld_buildings = underworld.buildings;
    let lures_today = underworld.lured_today;

    let gate_level = underworld_buildings[UnderworldBuildingType::Gate].level;
    let keeper_level = underworld_buildings[UnderworldBuildingType::Keeper].level;
    let max_lures = if gate_level >= 5 { 5 } else { gate_level };

    if (lures_today >= max_lures as u16 || keeper_level < 1)
    {
        // shitty_print("no keeper build, or max lures for underworld reached
        // skipping");
        return Ok(String::from(""));
    }

    let player_rank = session
        .send_command(Command::Custom {
            cmd_name: "PlayerGetHallOfFame".to_string(),
            arguments: ["-4", "", "0", "0"].iter().map(|&s| s.to_string()).collect(),
        })
        .await?;

    if let Some(fortress) = player_rank.fortress.as_ref()
    {
        if let Some(enemy_rank) = fortress.suggested_underworld_enemy_rank
        {
            let suggested_enemy_rank = enemy_rank;

            let calculate_page = suggested_enemy_rank / 51;

            let hall_of_fame_page = session.send_command(Command::HallOfFamePage { page: calculate_page as usize }).await?.clone();
            let players = &hall_of_fame_page.hall_of_fames.players;
            for x in players
            {
                if x.rank == suggested_enemy_rank
                {
                    let gamestate_after_lookup = session.send_command(Command::ViewPlayer { ident: x.name.to_string() }).await?.clone();
                    let lookup_player = &gamestate_after_lookup.lookup.lookup_name(&*x.name.to_string());
                    if let Some(looked_up_player) = lookup_player
                    {
                        session.send_command(Command::UnderworldAttack { player_id: looked_up_player.player_id }).await?;
                    }
                    else
                    {
                        shitty_print("No suggested player was found for the underworld");
                    }
                }
            }
        }
    }

    Ok(String::from(""))
}

pub async fn perform_underworld_atk_favourite_enemy(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;

    if gs.underworld.is_none()
    {
        return Ok(("".to_string()));
    }

    let underworld = match &gs.underworld
    {
        None => return Ok(("".to_string())),
        Some(uw) => uw,
    };

    let underworld_buildings = underworld.buildings;
    let lures_today = underworld.lured_today;

    let gate_level = underworld_buildings[UnderworldBuildingType::Gate].level;
    let keeper_level = underworld_buildings[UnderworldBuildingType::Keeper].level;
    let max_lures = if gate_level >= 5 { 5 } else { gate_level };

    if (lures_today >= max_lures as u16 || keeper_level < 1)
    {
        // shitty_print("no keeper build, or max lures for underworld reached
        // skipping");
        return Ok(("".to_string()));
    }

    let favourite_player: String = fetch_character_setting(&gs, "underworldFavouriteOpponents").unwrap_or_else(|| "".to_string());
    if (favourite_player == "")
    {
        return Ok(String::from("No favourite player specified in the underworld section."));
    }

    let gamestate_after_lookup = session.send_command(Command::ViewPlayer { ident: favourite_player.clone() }).await?.clone();
    let lookup_player = &gamestate_after_lookup.lookup.lookup_name(&favourite_player);
    if let Some(looked_up_player) = lookup_player
    {
        session.send_command(Command::UnderworldAttack { player_id: looked_up_player.player_id }).await?;
        return Ok(format!("Lured {} into the Underworld", looked_up_player.name));
    }
    return Ok(("".to_string()));
}

pub async fn collect_underworld_resources(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let enable_soul_collection: bool = fetch_character_setting(&gs, "underworldCollectSouls").unwrap_or(false);
    let enable_gold_collection: bool = fetch_character_setting(&gs, "underworldCollectGold").unwrap_or(false);
    let enable_thirst_collection: bool = fetch_character_setting(&gs, "underworldCollectThirst").unwrap_or(false);
    let mut result = String::from("");
    if let Some(underworld) = &gs.underworld
    {
        let last_updated = match underworld.last_collectable_update
        {
            Some(time) => time,
            None =>
            {
                return Ok(String::from(""));
            }
        };

        fn duration_since(last_updated: DateTime<Local>) -> TimeDelta
        {
            let now: DateTime<Local> = Local::now();
            now.signed_duration_since(last_updated)
        }

        let duration_since = duration_since(last_updated);
        let wait_between_gathers = 900;
        if (duration_since.num_seconds() >= wait_between_gathers)
        {
            let buildings = &underworld.buildings;
            let gold_pit = &buildings[UnderworldBuildingType::GoldPit];
            let time_machine = &buildings[UnderworldBuildingType::Adventuromatic];
            let soul_extractor = &buildings[UnderworldBuildingType::SoulExtractor];
            let thirst_in_time_machine = &underworld.production[UnderworldResourceType::ThirstForAdventure].last_collectable;

            if enable_soul_collection && (underworld.upgrade_building != Some(UnderworldBuildingType::SoulExtractor) && soul_extractor.level > 0)
            {
                session.send_command(Command::UnderworldCollect { resource: UnderworldResourceType::Souls }).await?;
                // result += "souls ";
            }

            if enable_thirst_collection && (underworld.upgrade_building != Some(UnderworldBuildingType::Adventuromatic) && time_machine.level > 0 && *thirst_in_time_machine > 0)
            {
                session.send_command(Command::UnderworldCollect { resource: UnderworldResourceType::ThirstForAdventure }).await?;
                // result += " thirst ";
            }

            if (enable_gold_collection && (underworld.upgrade_building != Some(UnderworldBuildingType::GoldPit) || gold_pit.level >= 25) && gold_pit.level > 0)
            {
                let dont_collect_from: String = fetch_character_setting(&gs, "underworldDontCollectGoldFrom").unwrap_or("00:00".to_string());
                let dont_collect_to: String = fetch_character_setting(&gs, "underworldDontCollectGoldTo").unwrap_or("00:01".to_string());
                let is_in_range = check_time_in_range(dont_collect_from, dont_collect_to);
                if (!is_in_range)
                {
                    session.send_command(Command::UnderworldCollect { resource: UnderworldResourceType::Silver }).await?;
                    // result += " gold ";
                }
            }
        }
    }
    let mut finalMessage = String::from("");
    if (result != "")
    {
        finalMessage += "Collected underworld resources: ";
        finalMessage += &result;
    }
    Ok(finalMessage)
}

pub async fn build_underworld_perfect_order(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let mut result = String::new();
    if let Some(underworld) = &gs.underworld
    {
        if let Some(currently_building) = &underworld.upgrade_building
        {
            if let Some(upgrade_finish) = underworld.upgrade_finish
            {
                if upgrade_finish <= chrono::Local::now()
                {
                    session.send_command(Command::UnderworldUpgradeFinish { building: currently_building.clone(), mushrooms: 0 }).await?;
                }
            }
            return Ok(String::from(""));
        }

        if let Some(building_to_upgrade) = find_next_building(underworld)
        {
            let current_souls_amount = underworld.souls_current;
            let character_silver = gs.character.silver;
            let building_prices = underworld.buildings[building_to_upgrade].upgrade_cost;
            let souls_required = building_prices.souls;
            let silver_required = building_prices.silver;

            if current_souls_amount >= souls_required && character_silver >= silver_required
            {
                session.send_command(Command::UnderworldUpgradeStart { building: building_to_upgrade, mushrooms: 0 }).await?;
                result += String::from(format!("Started to upgrade {}", get_building_name(building_to_upgrade))).as_str();
            }
            else
            {
                result += "Not enough souls or silver to upgrade building.";
            }
        }
    }

    Ok(result)
}

fn get_building_name(building: UnderworldBuildingType) -> String
{
    let mut result = String::new();
    match building
    {
        UnderworldBuildingType::HeartOfDarkness => return String::from("HeartOfDarkness"),
        UnderworldBuildingType::Gate => return String::from("Gate"),
        UnderworldBuildingType::GoldPit => return String::from("GoldPit"),
        UnderworldBuildingType::SoulExtractor => return String::from("SoulExtractor"),
        UnderworldBuildingType::GoblinPit => return String::from("GoblinPit"),
        UnderworldBuildingType::TortureChamber => return String::from("TortureChamber"),
        UnderworldBuildingType::GladiatorTrainer => return String::from("GladiatorTrainer"),
        UnderworldBuildingType::TrollBlock => return String::from("TrollBlock"),
        UnderworldBuildingType::Adventuromatic => return String::from("Adventuromatic"),
        UnderworldBuildingType::Keeper => return String::from("Keeper"),
    }
}

/// returns the building we need to upgrade next based on dreams list, not smart
/// but works
fn find_next_building(underworld: &Underworld) -> Option<UnderworldBuildingType>
{
    let mut building_counts: EnumMap<UnderworldBuildingType, usize> = EnumMap::default();

    let build_order = create_underworld_building_order_fixed_list();

    for building_type in build_order
    {
        building_counts[building_type] += 1;

        let current_building = &underworld.buildings[building_type];

        if current_building.level < building_counts[building_type] as u8
        {
            let msg = format!("level of {:?} building ({}) doesnt match the count ({}). will be upgraded next .", building_type, current_building.level, building_counts[building_type]);
            shitty_print(msg);
            return Some(building_type);
        }
    }
    None
}

pub async fn level_up_uw_keeper(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    session.send_command(Command::UnderworldUnitUpgrade { unit: UnderworldUnitType::Keeper }).await?;
    return Ok("Upgraded Keeper in the underworld".to_string());
}

pub fn create_underworld_building_order_fixed_list() -> Vec<UnderworldBuildingType>
{
    let mut underworld_buildings: Vec<UnderworldBuildingType> = Vec::new();
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::HeartOfDarkness);
    underworld_buildings.push(UnderworldBuildingType::SoulExtractor);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::Adventuromatic);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::GladiatorTrainer);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::Gate);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::TortureChamber);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::Keeper);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::TrollBlock);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoblinPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);
    underworld_buildings.push(UnderworldBuildingType::GoldPit);

    underworld_buildings
}
