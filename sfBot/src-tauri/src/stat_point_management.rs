use std::{collections::HashMap, error::Error};

use enum_map::EnumMap;
use sf_api::{
    command::{AttributeType, Command},
    gamestate::character::Class,
    SimpleSession,
};

use crate::{fetch_character_setting, lottery::sleep_between_commands, skill_point_list::get_attribute_price_map, utils::pretty_print};

pub async fn upgrade_skill_points(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    pretty_print(format!("Upgrading skill points for: {:?}", gs.character.name), gs);
    let char_distribution_Str: i32 = fetch_character_setting(&gs, "characterStatDistributionStr").unwrap_or(0);
    let char_distribution_Dex: i32 = fetch_character_setting(&gs, "characterStatDistributionDex").unwrap_or(0);
    let char_distribution_Int: i32 = fetch_character_setting(&gs, "characterStatDistributionInt").unwrap_or(0);
    let char_distribution_Const: i32 = fetch_character_setting(&gs, "characterStatDistributionConst").unwrap_or(0);
    let char_distribution_Luck: i32 = fetch_character_setting(&gs, "characterStatDistributionLuck").unwrap_or(0);
    let gold_to_keep: i64 = fetch_character_setting(&gs, "itemsInventoryMinGoldSaved").unwrap_or(0) * 100;

    let bought_attributes = gs.character.attribute_times_bought;
    let current_distribution = calc_current_percent_dist(bought_attributes);
    let target_distribution = get_target_distribution(char_distribution_Str, char_distribution_Dex, char_distribution_Int, char_distribution_Const, char_distribution_Luck);
    let attribute_to_increase = decide_attribute_to_increase(&gs.character.class, current_distribution, target_distribution);

    if let Some(attribute) = attribute_to_increase
    {
        let mut current_count = bought_attributes[attribute];
        let mut available_silver = gs.character.silver;
        if (gold_to_keep > available_silver as i64)
        {
            available_silver = 0;
        }
        else
        {
            available_silver = (available_silver as i64 - gold_to_keep) as u64;
        }
        let mut points_to_buy = 0;
        while let Some(price) = get_skill_point_price(current_count + 1)
        {
            if price == 0 || available_silver < price
            {
                break;
            }
            available_silver -= price;
            current_count += 1;
            points_to_buy += 1;

            session.send_command(Command::IncreaseAttribute { increase_to: current_count, attribute }).await?;
            sleep_between_commands(15).await;

            if available_silver < price
            {
                break;
            }

            // nur so als grenze
            if points_to_buy > 500
            {
                break;
            }
        }

        if points_to_buy > 0
        {
            return Ok(format!("Increased {} by {} points", attribute_to_string(attribute), points_to_buy));
        }
    }

    return Ok("".to_string());
}

// pub fn has_char_enough_silver_to_buy_a_skill_point(gs: &GameState,
// attribute_to_increase: Option<AttributeType>) -> bool {
//     if let Some(attribute) = attribute_to_increase
//     {
//         let times_bought = gs.character.attribute_times_bought[attribute] +
// 1;
//
//         // at some point, all skill points cost 10m gold
//         if times_bought >= 3152
//         {
//             return gs.character.silver >= 1_000_000_000;
//         }
//
//         let price_map = get_attribute_price_map();
//
//
//     }
//     false
// }

pub fn has_char_enough_silver_to_buy_a_skill_point(silver: u64, times_bought: u32) -> bool
{
    if times_bought >= 3152
    {
        return silver >= 1_000_000_000;
    }
    let price_map = get_attribute_price_map();

    let price = price_map.get((times_bought as usize)).copied().unwrap_or(0) as u64;
    return silver >= price;
}

pub fn get_skill_point_price(times_bought: u32) -> Option<u64>
{
    let price_map = get_attribute_price_map();
    let price = price_map.get((times_bought as usize)).copied();

    if times_bought >= 3152
    {
        return Some(1_000_000_000);
    }

    return price.map(|p| p as u64);
}

pub fn decide_attribute_to_increase(class: &Class, current_distribution: HashMap<AttributeType, f32>, target_distribution: HashMap<AttributeType, f32>) -> Option<AttributeType>
{
    let main_attribute = get_main_attribute(class);

    let mut largest_gap = 0.0;
    let mut attribute_to_increase = None;

    for (&attribute, &target_percent) in &target_distribution
    {
        let current_percent = *current_distribution.get(&attribute).unwrap_or(&0.0);
        let gap = target_percent - current_percent;

        if gap > largest_gap
        {
            largest_gap = gap;
            attribute_to_increase = Some(attribute);
        }
    }

    if largest_gap == 0.0
    {
        return main_attribute;
    }

    // Otherwise, return the attribute with the largest gap
    attribute_to_increase
}

pub fn calc_current_percent_dist(bought_attributes: EnumMap<AttributeType, u32>) -> HashMap<AttributeType, f32>
{
    let total: u32 = bought_attributes.values().sum();
    let mut distribution = HashMap::new();

    if total == 0
    {
        return distribution;
    }

    for (attribute, &value) in &bought_attributes
    {
        let percentage = (value as f32 / total as f32) * 100.0;
        distribution.insert(attribute, percentage);
    }

    distribution
}

// pub fn get_target_distribution(class: &Class) -> HashMap<AttributeType, f32>
// {
//     let mut target_distribution = HashMap::new();
//
//     match class
//     {
//         // int based classes
//         Class::Bard | Class::Mage | Class::Druid | Class::Necromancer =>
//         {
//             target_distribution.insert(AttributeType::Intelligence, 55.0);
//             target_distribution.insert(AttributeType::Dexterity, 2.0);
//             target_distribution.insert(AttributeType::Strength, 2.0);
//         }
//         // dex based classes
//         Class::Scout | Class::Assassin | Class::DemonHunter =>
//         {
//             target_distribution.insert(AttributeType::Dexterity, 55.0);
//             target_distribution.insert(AttributeType::Strength, 2.0);
//             target_distribution.insert(AttributeType::Intelligence, 2.0);
//         }
//         // str based classes
//         Class::Warrior | Class::BattleMage | Class::Berserker |
// Class::Paladin =>         {
//             target_distribution.insert(AttributeType::Strength, 55.0);
//             target_distribution.insert(AttributeType::Dexterity, 2.0);
//             target_distribution.insert(AttributeType::Intelligence, 2.0);
//         }
//     }
//     target_distribution.insert(AttributeType::Constitution, 35.0);
//     target_distribution.insert(AttributeType::Luck, 6.0);
//
//     target_distribution
// }

pub fn get_target_distribution(char_distribution_Str: i32, char_distribution_Dex: i32, char_distribution_Int: i32, char_distribution_Const: i32, char_distribution_Luck: i32) -> HashMap<AttributeType, f32>
{
    let mut target_distribution = HashMap::new();
    target_distribution.insert(AttributeType::Strength, char_distribution_Str as f32);
    target_distribution.insert(AttributeType::Dexterity, char_distribution_Dex as f32);
    target_distribution.insert(AttributeType::Intelligence, char_distribution_Int as f32);
    target_distribution.insert(AttributeType::Constitution, char_distribution_Const as f32);
    target_distribution.insert(AttributeType::Luck, char_distribution_Luck as f32);

    target_distribution
}

pub fn get_main_attribute(class: &Class) -> Option<AttributeType>
{
    match class
    {
        Class::Bard | Class::Mage | Class::Druid | Class::Necromancer => Some(AttributeType::Intelligence),
        Class::Scout | Class::Assassin | Class::DemonHunter | Class::PlagueDoctor => Some(AttributeType::Dexterity),
        Class::Warrior | Class::BattleMage | Class::Berserker | Class::Paladin => Some(AttributeType::Strength),
    }
}

pub fn attribute_to_string(attribute: AttributeType) -> String
{
    match attribute
    {
        AttributeType::Strength =>
        {
            return String::from("Strength");
        }
        AttributeType::Dexterity =>
        {
            return String::from("Dexterity");
        }
        AttributeType::Intelligence =>
        {
            return String::from("Intelligence");
        }
        AttributeType::Constitution =>
        {
            return String::from("Constitution");
        }
        AttributeType::Luck =>
        {
            return String::from("Luck");
        }
    }
}
