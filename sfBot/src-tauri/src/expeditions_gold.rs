#![allow(warnings)]

use std::{borrow::Borrow, collections::HashMap, fmt::Debug, hash::Hash, time::Duration};

use chrono::{DateTime, Local};
use serde_json::json;
use sf_api::{
    command::{Command, TimeSkip},
    gamestate::{
        rewards::RewardType,
        tavern::{AvailableExpedition, AvailableTasks, CurrentAction, ExpeditionEncounter, ExpeditionStage, ExpeditionThing},
    },
    SimpleSession,
};
use tokio::time::sleep;

use crate::{
    bot_runner::write_character_log,
    expedition_utils::{clear_all_encounters_counts, get_all_encounters_counts, get_encounter_count, increment_encounter_count, is_expedition_still_completeable, log_expedition_info, print_all_encounter_counts, select_best_expedition_reward_based_on_priority, should_buy_beer},
    expeditions_exp::{pick_best_crossroads_toilet_paper_exp, try_picking_best_crossroad_based_on_expedition_type_exp},
    fetch_character_setting,
    inventory_management::manage_inventory,
    utils::{get_global_settings, get_u64_setting},
};

pub fn time_remaining<T: Borrow<DateTime<Local>>>(time: T) -> Duration { (*time.borrow() - Local::now()).to_std().unwrap_or_default() }

pub fn map_prios(prio_list: Vec<String>) -> HashMap<RewardType, usize>
{
    let mut reward_priority_map = HashMap::new();

    for (i, reward_name) in prio_list.iter().enumerate()
    {
        if let Some(reward_type) = convert_string_to_reward(reward_name)
        {
            reward_priority_map.insert(reward_type, i);
        }
    }

    reward_priority_map
}

pub fn convert_string_to_reward(string_reward_type: &str) -> Option<RewardType>
{
    match string_reward_type
    {
        "Hellevator Points" => Some(RewardType::HellevatorPoints),
        "Hellevator Cards" => Some(RewardType::HellevatorCards),
        "Mushrooms" => Some(RewardType::Mushrooms),
        "Silver" => Some(RewardType::Silver),
        "Lucky Coins" => Some(RewardType::LuckyCoins),
        "Wood" => Some(RewardType::Wood),
        "Stone" => Some(RewardType::Stone),
        "Arcane Splinter" => Some(RewardType::Arcane),
        "Metal" => Some(RewardType::Metal),
        "Souls" => Some(RewardType::Souls),
        "Fruit Basket" => Some(RewardType::FruitBasket),
        "XP" => Some(RewardType::XP),
        "Pet Egg" => Some(RewardType::Egg),
        "Quicksand Glasses" => Some(RewardType::QuicksandGlass),
        "Honor" => Some(RewardType::Honor),
        "Beer" => Some(RewardType::Beer),
        "Frame" => Some(RewardType::Frame),
        "Legendary Gem" => Some(RewardType::LegendaryGem),
        "Gold Fidget" => Some(RewardType::GoldFidget),
        "Silver Fidget" => Some(RewardType::SilverFidget),
        "Bronze Fidget" => Some(RewardType::BronzeFidget),
        "Gem" => Some(RewardType::Gem),
        _ => Some(RewardType::Unknown),
    }
}

pub async fn play_expeditions_gold(session: &mut SimpleSession, char_name: &str, skip_wait_time_using_hourglas: bool, beers_to_drink: u8, prio_list: Vec<String>) -> Result<String, Box<dyn std::error::Error>>
{
    let server_host = session.server_url().host_str().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
    let mut chosen_expedition_type: Option<ExpeditionThing> = None;
    let user_setting_prio = map_prios(prio_list);
    let global_map = get_global_settings().await.unwrap_or_default();
    let min_wait = get_u64_setting(&global_map, "globalSleepTimesMin", 50);
    let max_wait = get_u64_setting(&global_map, "globalSleepTimesMax", 100);

    loop
    {
        let gs = session.send_command(Command::Update).await?.clone();
        manage_inventory(session).await?;

        let beer_drunk = gs.tavern.beer_drunk;
        let beer_max = gs.tavern.beer_max;
        let thirst_for_adventure_sec = gs.tavern.thirst_for_adventure_sec;
        let mushrooms = gs.character.mushrooms;
        let mushrooms_to_keep: i32 = fetch_character_setting(&gs, "itemsInventoryMinMushroomsSaved").unwrap_or(1);
        let character_equip = &gs.character.equipment;
        if should_buy_beer(character_equip, beers_to_drink, beer_drunk, beer_max, thirst_for_adventure_sec, mushrooms, mushrooms_to_keep as u32)
        {
            session.send_command(Command::BuyBeer).await?;
        }

        let gs = session.send_command(Command::Update).await?;
        let exp = &gs.tavern.expeditions;
        if let Some(active) = exp.active()
        {
            let current = active.current_stage();
            let cmd = match current
            {
                ExpeditionStage::Boss(_) =>
                {
                    if active.current_floor == 10
                    {
                        print_all_encounter_counts(&*gs.character.name);
                        log_expedition_info(&*gs.character.name, gs.character.player_id, &server_host, "gold", active.current_floor, chosen_expedition_type.as_ref(), active.heroism as u32, &get_all_encounters_counts(&*gs.character.name));
                    }

                    Command::ExpeditionContinue
                }
                ExpeditionStage::Rewards(rewards) =>
                {
                    if rewards.is_empty()
                    {
                        return Ok("".to_string());
                        panic!("No rewards to choose from");
                    }

                    if let Some(best_reward_pos) = select_best_expedition_reward_based_on_priority(&rewards, &user_setting_prio)
                    {
                        Command::ExpeditionPickReward { pos: best_reward_pos }
                    }
                    else
                    {
                        return Err("No rewards available to pick".into());
                    }
                }
                ExpeditionStage::Encounters(roads) =>
                {
                    if roads.is_empty()
                    {
                        return Ok("".to_string());
                        panic!("No crossroads to choose from");
                    }

                    if chosen_expedition_type.is_none()
                    {
                        if let AvailableTasks::Expeditions(expeditions) = gs.tavern.available_tasks()
                        {
                            if let Some((_, expedition_type)) = select_best_expedition_gold(expeditions)
                            {
                                chosen_expedition_type = Some(expedition_type.target.clone());
                            }
                        }
                    }

                    let can_complete_expedition = is_expedition_still_completeable(chosen_expedition_type.as_ref(), active.current_floor, &*char_name);

                    let chests_picked_amount = get_encounter_count(&*char_name, ExpeditionThing::Suitcase);

                    let best_index = if chests_picked_amount >= 2
                    {
                        if (can_complete_expedition)
                        {
                            try_picking_best_crossroad_based_on_expedition_type_exp(roads, chosen_expedition_type.as_ref(), active.current_floor, &*char_name)
                        }
                        else
                        {
                            pick_best_crossroads_toilet_paper_exp(&roads, 5, &*char_name)
                            // current floor 5 because we need to choose one
                            // between 1-9
                        }
                    }
                    else
                    {
                        if (can_complete_expedition)
                        {
                            try_picking_best_crossroad_based_on_expedition_type_gold(roads, chosen_expedition_type.as_ref(), active.current_floor, &*char_name)
                        }
                        else
                        {
                            pick_best_crossroads_toilet_paper_gold(&roads, 5, &*char_name)
                            // current floor 5 because we need to choose one
                            // between 1-9
                        }
                    };

                    let best_index = best_index.unwrap_or_else(|| {
                        eprintln!("Error: No valid crossroad index found.");
                        0 // required if bot gets logged out in the middle of
                          // choosing a crossroad
                    });
                    Command::ExpeditionPickEncounter { pos: best_index }
                }
                ExpeditionStage::Waiting(until) =>
                {
                    if skip_wait_time_using_hourglas && gs.tavern.quicksand_glasses > 0
                    {
                        session.send_command(Command::ExpeditionSkipWait { typ: TimeSkip::Glass }).await?;
                        continue;
                    }
                    else
                    {
                        return Ok(String::from("Character is waiting for the expedition to finish."));
                    }
                }
                ExpeditionStage::Finished =>
                {
                    return Ok(String::from("Finished expedition."));
                    // continue;
                }
                ExpeditionStage::Unknown => panic!("Unknown expedition stage encountered"),
            };
            sleep(Duration::from_millis(fastrand::u64(min_wait..max_wait))).await;
            session.send_command(cmd).await?;
        }
        else
        {
            sleep(Duration::from_millis(fastrand::u64(min_wait..max_wait))).await;
            let gs = session.send_command(Command::Update).await?.clone();
            if gs.tavern.thirst_for_adventure_sec > 0 || matches!(gs.tavern.current_action, CurrentAction::Expedition)
            {
                match gs.tavern.current_action
                {
                    CurrentAction::Idle =>
                    {}
                    CurrentAction::CityGuard { hours, busy_until } =>
                    {
                        return Ok(String::from(""));
                    }
                    CurrentAction::Quest { quest_idx, busy_until } =>
                    {
                        return Ok(String::from(""));
                    }
                    CurrentAction::Expedition =>
                    {
                        // character is still on an expedition, continue processing it
                        session.send_command(Command::ExpeditionContinue).await?;
                        // return Ok(String::from(""));
                    }
                    CurrentAction::Unknown(_) =>
                    {},
                }

                if let CurrentAction::Idle = gs.tavern.current_action
                {
                    match &gs.tavern.available_tasks()
                    {
                        AvailableTasks::Expeditions(expeditions) =>
                        {
                            if let Some((pos, best_expedition)) = select_best_expedition_gold(&expeditions)
                            {
                                chosen_expedition_type = Some(best_expedition.target.clone());
                                clear_all_encounters_counts(&*char_name);
                                session.send_command(Command::ExpeditionStart { pos }).await?;
                                write_character_log(
                                    &gs.character.name,
                                    gs.character.player_id,
                                    &format!(
                                        "EXPEDITION_GOLD: Started {:?} (thirst {}s)",
                                        best_expedition.target,
                                        best_expedition.thirst_for_adventure_sec
                                    ),
                                );
                            }
                            else
                            {
                                return Ok(String::from(""));
                            }
                        }
                        AvailableTasks::Quests(_) =>
                        {
                            return Ok(String::from(""));
                        }
                    }
                }
                else
                {
                    return Ok(String::from(""));
                }
            }
            else
            {
                return Ok(String::from(""));
            }
        }
    }
    return Ok(String::from(""));
}

async fn send_to_hook_exp_gold(message: &str)
{
    let payload = json!({
        "content": message
    });

    if let Err(e) = reqwest::Client::new().post("https://discord.com/api/webhooks/1365473727407718503/eqwcotk7x6U7SsI6IzHS3dDHVTFkolj46mCGOlUiO4ALGNfZ56-O7Kn5P7mt2RmhvaGr").json(&payload).send().await
    {
        eprintln!("Error sending webhook: {}", e);
    }
}

pub fn try_picking_best_crossroad_based_on_expedition_type_gold(encounters: Vec<ExpeditionEncounter>, chosen_expedition_type: Option<&ExpeditionThing>, current_floor: u8, char_name: &str) -> Option<usize>
{
    let index_to_pick = match chosen_expedition_type
    {
        Some(ExpeditionThing::ToiletPaper) => pick_best_crossroads_toilet_paper_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::RevealingCouple) => pick_best_crossroads_revealing_lady_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Balloons) => pick_best_crossroads_bewitched_stew_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Dragon) => pick_best_crossroads_dragon_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Unicorn) => pick_best_crossroads_unicorn_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::WinnersPodium) => pick_best_crossroads_winners_podium_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::BurntCampfire) => pick_best_crossroads_burnt_campfire_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::BrokenSword) => pick_best_crossroads_broken_sword_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::RoyalFrog) => pick_best_crossroads_toxic_fountain_cure_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Klaus) => pick_best_crossroads_klaus_gold(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Cake) => pick_best_crossroads_suckling_pig_gold(&encounters, current_floor, char_name),
        _ => Some(0),
    };

    index_to_pick
}

pub fn pick_best_crossroads_toilet_paper_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Bait, 3.0);
        priority_map.insert(ExpeditionThing::Dragon, 4.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::ToiletPaper, 3.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::ToiletPaper, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 5.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 6.0);
        priority_map.insert(ExpeditionThing::Dragon, 8.0);
        priority_map.insert(ExpeditionThing::Prince, 9.0);
        priority_map.insert(ExpeditionThing::Unicorn, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 12.0);
        priority_map.insert(ExpeditionThing::Phoenix, 13.0);
        priority_map.insert(ExpeditionThing::Cake, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::Donkey, 16.0);
        priority_map.insert(ExpeditionThing::CupCake, 17.0);
        priority_map.insert(ExpeditionThing::CampFire, 18.0);
        priority_map.insert(ExpeditionThing::Dummy2, 19.0);
        priority_map.insert(ExpeditionThing::BentSword, 20.0);
        priority_map.insert(ExpeditionThing::Well, 21.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 22.0);
        priority_map.insert(ExpeditionThing::Dummy1, 23.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 24.0);
        priority_map.insert(ExpeditionThing::Socks, 25.0);
        priority_map.insert(ExpeditionThing::ClothPile, 26.0);
        priority_map.insert(ExpeditionThing::Key, 27.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 28.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 29.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 30.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 31.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 32.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 33.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 34.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 36.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 37.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 38.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 39.0);
        priority_map.insert(ExpeditionThing::Bait, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Hand, 42.0);
        priority_map.insert(ExpeditionThing::Feet, 43.0);
        priority_map.insert(ExpeditionThing::Body, 44.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 45.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 46.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
        }
    }
    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }

    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;

        increment_encounter_count(&*char_name, picked_encounter);

    }

    picked_index
}

pub fn pick_best_crossroads_revealing_lady_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Socks, 3.0);
        priority_map.insert(ExpeditionThing::ClothPile, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Socks, 3.0);
        priority_map.insert(ExpeditionThing::ClothPile, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 5.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 23.0);
        priority_map.insert(ExpeditionThing::Socks, 24.0);
        priority_map.insert(ExpeditionThing::ClothPile, 25.0);
        priority_map.insert(ExpeditionThing::Key, 26.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 27.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 28.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 29.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 30.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 31.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 33.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 34.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::Bait, 39.0);
        priority_map.insert(ExpeditionThing::Girl, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 41.0);
        priority_map.insert(ExpeditionThing::Feet, 42.0);
        priority_map.insert(ExpeditionThing::Body, 43.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);
        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }

    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_bewitched_stew_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Well, 3.0);
        priority_map.insert(ExpeditionThing::Girl, 4.0);
        priority_map.insert(ExpeditionThing::Balloons, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Well, 3.0);
        priority_map.insert(ExpeditionThing::Girl, 4.0);
        priority_map.insert(ExpeditionThing::Balloons, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 4.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 5.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 23.0);
        priority_map.insert(ExpeditionThing::Socks, 24.0);
        priority_map.insert(ExpeditionThing::ClothPile, 25.0);
        priority_map.insert(ExpeditionThing::Key, 26.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 27.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 28.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 29.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 30.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 31.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 33.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 34.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::Bait, 39.0);
        priority_map.insert(ExpeditionThing::Girl, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 41.0);
        priority_map.insert(ExpeditionThing::Feet, 42.0);
        priority_map.insert(ExpeditionThing::Body, 43.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);
        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }

    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_dragon_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Bait, 3.0);
        priority_map.insert(ExpeditionThing::Dragon, 4.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
    }
    else if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Bait, 3.0);
        priority_map.insert(ExpeditionThing::Dragon, 4.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
    }
    else if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::Dragon, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 6.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 23.0);
        priority_map.insert(ExpeditionThing::Socks, 24.0);
        priority_map.insert(ExpeditionThing::ClothPile, 25.0);
        priority_map.insert(ExpeditionThing::Key, 26.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 27.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 28.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 29.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 30.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 31.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 33.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 34.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::Bait, 39.0);
        priority_map.insert(ExpeditionThing::Girl, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 41.0);
        priority_map.insert(ExpeditionThing::Feet, 42.0);
        priority_map.insert(ExpeditionThing::Body, 43.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }

    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_unicorn_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 3.0);
        priority_map.insert(ExpeditionThing::Donkey, 4.0);
        priority_map.insert(ExpeditionThing::Rainbow, 5.0);
        priority_map.insert(ExpeditionThing::Unicorn, 6.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 3.0);
        priority_map.insert(ExpeditionThing::Donkey, 4.0);
        priority_map.insert(ExpeditionThing::Rainbow, 5.0);
        priority_map.insert(ExpeditionThing::Unicorn, 6.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::Unicorn, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 6.0);
        priority_map.insert(ExpeditionThing::Dragon, 8.0);
        priority_map.insert(ExpeditionThing::Prince, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 23.0);
        priority_map.insert(ExpeditionThing::Socks, 24.0);
        priority_map.insert(ExpeditionThing::ClothPile, 25.0);
        priority_map.insert(ExpeditionThing::Key, 26.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 27.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 28.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 29.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 30.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 31.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 33.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 34.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::Bait, 39.0);
        priority_map.insert(ExpeditionThing::Girl, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 41.0);
        priority_map.insert(ExpeditionThing::Feet, 42.0);
        priority_map.insert(ExpeditionThing::Body, 43.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);
        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }

    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_winners_podium_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 3.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 4.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 3.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 4.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 23.0);
        priority_map.insert(ExpeditionThing::Socks, 24.0);
        priority_map.insert(ExpeditionThing::ClothPile, 25.0);
        priority_map.insert(ExpeditionThing::Key, 26.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 27.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 28.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 29.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 30.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 31.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 33.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 34.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::Bait, 39.0);
        priority_map.insert(ExpeditionThing::Girl, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 41.0);
        priority_map.insert(ExpeditionThing::Feet, 42.0);
        priority_map.insert(ExpeditionThing::Body, 43.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }

    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_burnt_campfire_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::CampFire, 3.0);
        priority_map.insert(ExpeditionThing::Phoenix, 4.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::CampFire, 11.5);
        priority_map.insert(ExpeditionThing::Phoenix, 4.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    else if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::Donkey, 16.0);
        priority_map.insert(ExpeditionThing::CupCake, 17.0);
        priority_map.insert(ExpeditionThing::CampFire, 18.0);
        priority_map.insert(ExpeditionThing::Dummy2, 19.0);
        priority_map.insert(ExpeditionThing::BentSword, 20.0);
        priority_map.insert(ExpeditionThing::Well, 21.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 22.0);
        priority_map.insert(ExpeditionThing::Dummy1, 23.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 24.0);
        priority_map.insert(ExpeditionThing::Socks, 25.0);
        priority_map.insert(ExpeditionThing::ClothPile, 26.0);
        priority_map.insert(ExpeditionThing::Key, 27.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 28.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 29.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 30.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 31.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 32.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 33.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 34.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::Bait, 39.0);
        priority_map.insert(ExpeditionThing::Girl, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 41.0);
        priority_map.insert(ExpeditionThing::Feet, 42.0);
        priority_map.insert(ExpeditionThing::Body, 43.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }

    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

fn pick_best_crossroads_broken_sword_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 3.0);
        priority_map.insert(ExpeditionThing::BentSword, 4.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 5.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 3.0);
        priority_map.insert(ExpeditionThing::BentSword, 4.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 10.0);
        priority_map.insert(ExpeditionThing::Dummy2, 11.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 12.0);
        priority_map.insert(ExpeditionThing::Donkey, 13.0);
        priority_map.insert(ExpeditionThing::Rainbow, 14.0);
        priority_map.insert(ExpeditionThing::Unicorn, 15.0);
        priority_map.insert(ExpeditionThing::Dummy1, 16.0);
        priority_map.insert(ExpeditionThing::Bait, 17.0);
        priority_map.insert(ExpeditionThing::Dragon, 18.0);
        priority_map.insert(ExpeditionThing::Girl, 19.0);
        priority_map.insert(ExpeditionThing::Balloons, 20.0);
        priority_map.insert(ExpeditionThing::ClothPile, 21.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 22.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 23.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 24.0);
        priority_map.insert(ExpeditionThing::CupCake, 25.0);
        priority_map.insert(ExpeditionThing::Prince, 26.0);
        priority_map.insert(ExpeditionThing::CampFire, 28.0);
        priority_map.insert(ExpeditionThing::Phoenix, 29.0);
        priority_map.insert(ExpeditionThing::Socks, 30.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 34.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 34.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 34.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 34.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 34.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 34.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 34.06);
        priority_map.insert(ExpeditionThing::KlausBounty, 34.08);
        priority_map.insert(ExpeditionThing::Cake, 35.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 36.0);
        priority_map.insert(ExpeditionThing::Well, 39.0);
        priority_map.insert(ExpeditionThing::Hand, 42.0);
        priority_map.insert(ExpeditionThing::Feet, 43.0);
        priority_map.insert(ExpeditionThing::Body, 44.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 15.0);
        priority_map.insert(ExpeditionThing::Donkey, 16.0);
        priority_map.insert(ExpeditionThing::CupCake, 17.0);
        priority_map.insert(ExpeditionThing::CampFire, 18.0);
        priority_map.insert(ExpeditionThing::Dummy2, 19.0);
        priority_map.insert(ExpeditionThing::BentSword, 20.0);
        priority_map.insert(ExpeditionThing::Well, 21.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 22.0);
        priority_map.insert(ExpeditionThing::Dummy1, 23.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 24.0);
        priority_map.insert(ExpeditionThing::Socks, 25.0);
        priority_map.insert(ExpeditionThing::ClothPile, 26.0);
        priority_map.insert(ExpeditionThing::Key, 27.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 28.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 29.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 30.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 31.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 32.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 33.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 34.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 36.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 37.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 38.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 39.0);
        priority_map.insert(ExpeditionThing::Bait, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Hand, 42.0);
        priority_map.insert(ExpeditionThing::Feet, 43.0);
        priority_map.insert(ExpeditionThing::Body, 44.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 45.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 6.0);
            }
        }
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }
    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

fn pick_best_crossroads_toxic_fountain_cure_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();
    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Prince, 3.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 4.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Prince, 3.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 10.0);
        priority_map.insert(ExpeditionThing::Dummy2, 11.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 12.0);
        priority_map.insert(ExpeditionThing::Donkey, 13.0);
        priority_map.insert(ExpeditionThing::Rainbow, 14.0);
        priority_map.insert(ExpeditionThing::Unicorn, 15.0);
        priority_map.insert(ExpeditionThing::Dummy1, 16.0);
        priority_map.insert(ExpeditionThing::Bait, 17.0);
        priority_map.insert(ExpeditionThing::Dragon, 18.0);
        priority_map.insert(ExpeditionThing::Girl, 19.0);
        priority_map.insert(ExpeditionThing::Balloons, 20.0);
        priority_map.insert(ExpeditionThing::ClothPile, 21.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 22.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 23.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 24.0);
        priority_map.insert(ExpeditionThing::CupCake, 25.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 27.0);
        priority_map.insert(ExpeditionThing::CampFire, 28.0);
        priority_map.insert(ExpeditionThing::Phoenix, 29.0);
        priority_map.insert(ExpeditionThing::Socks, 30.0);
        priority_map.insert(ExpeditionThing::BentSword, 33.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 34.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 34.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 34.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 34.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 34.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 34.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 34.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 34.08);
        priority_map.insert(ExpeditionThing::Cake, 35.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 36.0);
        priority_map.insert(ExpeditionThing::Well, 39.0);
        priority_map.insert(ExpeditionThing::Hand, 42.0);
        priority_map.insert(ExpeditionThing::Feet, 43.0);
        priority_map.insert(ExpeditionThing::Body, 44.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 48.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::Donkey, 16.0);
        priority_map.insert(ExpeditionThing::CupCake, 17.0);
        priority_map.insert(ExpeditionThing::CampFire, 18.0);
        priority_map.insert(ExpeditionThing::Dummy2, 19.0);
        priority_map.insert(ExpeditionThing::BentSword, 20.0);
        priority_map.insert(ExpeditionThing::Well, 21.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 22.0);
        priority_map.insert(ExpeditionThing::Dummy1, 23.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 24.0);
        priority_map.insert(ExpeditionThing::Socks, 25.0);
        priority_map.insert(ExpeditionThing::ClothPile, 26.0);
        priority_map.insert(ExpeditionThing::Key, 27.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 28.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 29.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 30.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 31.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 32.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 33.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 34.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 36.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 37.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 38.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 39.0);
        priority_map.insert(ExpeditionThing::Bait, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Hand, 42.0);
        priority_map.insert(ExpeditionThing::Feet, 43.0);
        priority_map.insert(ExpeditionThing::Body, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }
    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

fn pick_best_crossroads_klaus_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Hand, 3.0);
        priority_map.insert(ExpeditionThing::Feet, 4.0);
        priority_map.insert(ExpeditionThing::Body, 5.0);
        priority_map.insert(ExpeditionThing::Klaus, 6.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 8.5);
        priority_map.insert(ExpeditionThing::KlausBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Hand, 3.0);
        priority_map.insert(ExpeditionThing::Feet, 4.0);
        priority_map.insert(ExpeditionThing::Body, 5.0);
        priority_map.insert(ExpeditionThing::Klaus, 6.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 8.5);
        priority_map.insert(ExpeditionThing::KlausBounty, 8.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
                                                           
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        
        priority_map.insert(ExpeditionThing::Cake, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
                                                                 
        priority_map.insert(ExpeditionThing::Well, 40.0);
                                                          
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
                                                               
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Cake, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 23.0);
        priority_map.insert(ExpeditionThing::Socks, 24.0);
        priority_map.insert(ExpeditionThing::ClothPile, 25.0);
        priority_map.insert(ExpeditionThing::Key, 26.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 27.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 28.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 29.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 30.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 31.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 33.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 34.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::Bait, 39.0);
        priority_map.insert(ExpeditionThing::Girl, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 41.0);
        priority_map.insert(ExpeditionThing::Feet, 42.0);
        priority_map.insert(ExpeditionThing::Body, 43.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }
    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

fn pick_best_crossroads_suckling_pig_gold(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();
    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Cake, 3.0);
                                                         
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        
        
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::ClothPile, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 33.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 39.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Girl, 41.0);
        priority_map.insert(ExpeditionThing::Balloons, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::Key, 1.0);
        priority_map.insert(ExpeditionThing::Suitcase, 2.0);
        priority_map.insert(ExpeditionThing::Cake, 3.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 7.0);
        priority_map.insert(ExpeditionThing::Dumy3, 11.0);
        priority_map.insert(ExpeditionThing::Dummy2, 12.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::Rainbow, 15.0);
        priority_map.insert(ExpeditionThing::Unicorn, 16.0);
        priority_map.insert(ExpeditionThing::Dummy1, 17.0);
        priority_map.insert(ExpeditionThing::Bait, 18.0);
        priority_map.insert(ExpeditionThing::Dragon, 19.0);
        priority_map.insert(ExpeditionThing::Girl, 20.0);
        priority_map.insert(ExpeditionThing::Balloons, 21.0);
        priority_map.insert(ExpeditionThing::ClothPile, 22.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 23.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 24.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Prince, 27.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 28.0);
        priority_map.insert(ExpeditionThing::CampFire, 29.0);
        priority_map.insert(ExpeditionThing::Phoenix, 30.0);
        priority_map.insert(ExpeditionThing::Socks, 31.0);
        priority_map.insert(ExpeditionThing::BentSword, 34.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 35.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 35.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 35.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 35.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 35.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 35.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 35.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 35.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.08);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::Well, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 46.0);
        priority_map.insert(ExpeditionThing::Klaus, 47.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 9.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 10.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 7.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Suitcase, 1.0);
        priority_map.insert(ExpeditionThing::Klaus, 2.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 3.0);
        priority_map.insert(ExpeditionThing::Balloons, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Cake, 8.0);
        priority_map.insert(ExpeditionThing::Prince, 9.0);
        priority_map.insert(ExpeditionThing::Unicorn, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 12.0);
        priority_map.insert(ExpeditionThing::Phoenix, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 23.0);
        priority_map.insert(ExpeditionThing::Socks, 24.0);
        priority_map.insert(ExpeditionThing::ClothPile, 25.0);
        priority_map.insert(ExpeditionThing::Key, 26.0);
        priority_map.insert(ExpeditionThing::DummyBounty, 27.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 28.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 29.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 30.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 31.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 32.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 33.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 34.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 35.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 36.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 37.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 38.0);
        priority_map.insert(ExpeditionThing::Bait, 39.0);
        priority_map.insert(ExpeditionThing::Girl, 40.0);
        priority_map.insert(ExpeditionThing::Hand, 41.0);
        priority_map.insert(ExpeditionThing::Feet, 42.0);
        priority_map.insert(ExpeditionThing::Body, 43.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 44.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 45.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 17.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 11.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 11.5);
    }
    let picked_index = pick_best_encounter_gold(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}
fn select_best_expedition_gold(expeditions: &[AvailableExpedition]) -> Option<(usize, &AvailableExpedition)>
{
    expeditions
        .iter()
        .enumerate()
        .min_by(|(_, exp1), (_, exp2)| {
            let mut priority_score1 = exp1.target.priority().unwrap_or(f32::MAX);
            let mut priority_score2 = exp2.target.priority().unwrap_or(f32::MAX);

            // Longer expeditions receive a penalty
            if exp1.thirst_for_adventure_sec > exp2.thirst_for_adventure_sec
            {
                priority_score1 += 1.3;
            }
            else if exp2.thirst_for_adventure_sec > exp1.thirst_for_adventure_sec
            {
                priority_score2 += 1.3;
            }

            priority_score1.partial_cmp(&priority_score2).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(pos, best_expedition)| {
            let msg = format!("Expedition chosen at position {} with target {:?} (Priority: {:?})", pos, best_expedition.target, best_expedition.target.priority());
            (pos, best_expedition)
        })
}

trait ExpeditionPriority
{
    fn priority(&self) -> Option<f32>;
}

impl ExpeditionPriority for ExpeditionThing
{
    fn priority(&self) -> Option<f32>
    {
        match self
        {
            ExpeditionThing::ToiletPaper => Some(1.1),
            ExpeditionThing::RevealingCouple => Some(1.2),
            ExpeditionThing::Balloons => Some(1.3),
            ExpeditionThing::Dragon => Some(2.0),
            ExpeditionThing::Unicorn => Some(3.0),
            ExpeditionThing::WinnersPodium => Some(3.1),
            ExpeditionThing::BurntCampfire => Some(5.0),
            ExpeditionThing::BrokenSword => Some(5.1),
            ExpeditionThing::RoyalFrog => Some(5.2),
            ExpeditionThing::Klaus => Some(6.0),
            ExpeditionThing::Cake => Some(6.1),
            _ => None,
        }
    }
}

fn pick_best_encounter_gold(encounters: &[ExpeditionEncounter], priority_map: &HashMap<ExpeditionThing, f64>) -> Option<usize>
{
    let mut best_index = None;
    let mut lowest_priority = f64::MAX;
    let mut best_encounter_type = None;

    for (index, encounter) in encounters.iter().enumerate()
    {
        if let Some(&priority) = priority_map.get(&encounter.typ)
        {
            if priority < lowest_priority
            {
                lowest_priority = priority;
                best_index = Some(index);
                best_encounter_type = Some(&encounter.typ);
            }
        }
    }

    if let Some(index) = best_index
    {
        if let Some(encounter_type) = best_encounter_type
        {
            let encounter_name = format!("{:?}", encounter_type);
            //     "Crossroad picked at index {} with type {:?} and priority
            // {}",     index, encounter_type, lowest_priority
            // );
        }
    }
    else
    {
    }
    best_index
}
