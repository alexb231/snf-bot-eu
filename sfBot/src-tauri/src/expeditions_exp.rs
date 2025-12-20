#![allow(warnings)]

use std::{borrow::Borrow, collections::HashMap, fmt::Debug, time::Duration};

use chrono::{DateTime, Local};
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
    expeditions_gold::{pick_best_crossroads_toilet_paper_gold, try_picking_best_crossroad_based_on_expedition_type_gold},
    fetch_character_setting,
    inventory_management::manage_inventory,
    utils::{get_global_settings, get_u64_setting},
};

pub fn time_remaining<T: Borrow<DateTime<Local>>>(time: T) -> Duration { (*time.borrow() - Local::now()).to_std().unwrap_or_default() }

pub fn map_prios(prio_list: Vec<String>) -> HashMap<RewardType, usize>
{
    let mut reward_priority_map = HashMap::new();

    for (prioNumber, reward_name) in prio_list.iter().enumerate()
    {
        if let Some(reward_type) = crate::expeditions_gold::convert_string_to_reward(reward_name)
        {
            reward_priority_map.insert(reward_type, prioNumber);
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

pub async fn play_expeditions_exp(session: &mut SimpleSession, char_name: &str, skip_wait_time_using_hourglas: bool, beers_to_drink: u8, prio_list: Vec<String>) -> Result<String, Box<dyn std::error::Error>>
{
    let server_host = session.server_url().host_str().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
    let mut chosen_expedition_type: Option<ExpeditionThing> = None;
    let user_setting_prio = map_prios(prio_list);
    let global_map = get_global_settings().await.unwrap_or_default();
    let min_wait = get_u64_setting(&global_map, "globalSleepTimesMin", 50);
    let max_wait = get_u64_setting(&global_map, "globalSleepTimesMax", 100);

    // https://f9.sfgame.net/cmd.php?req=AdvertisementsCompleted&params=MQ==&sid=0-spyAW5gkGbaaN8
    let gs = session.send_command(Command::Update).await?.clone();

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
            let updated_gs = session.send_command(Command::Update).await?;
            let new_beer_count = updated_gs.tavern.beer_drunk;
            let new_beer_max = updated_gs.tavern.beer_max;
            let new_mushrooms = updated_gs.character.mushrooms;
            let new_thirst = updated_gs.tavern.thirst_for_adventure_sec;
            write_character_log(
                &gs.character.name,
                gs.character.player_id,
                &format!(
                    "TAVERN: Bought beer ({}/{}), mushrooms: {}, thirst: {}",
                    new_beer_count, new_beer_max, new_mushrooms, new_thirst
                ),
            );
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
                        print_all_encounter_counts(char_name);
                        log_expedition_info(char_name, gs.character.player_id, &server_host, "exp", active.current_floor, chosen_expedition_type.as_ref(), active.heroism as u32, &get_all_encounters_counts(char_name));
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
                            if let Some((_, expedition_type)) = select_best_expedition_exp(expeditions)
                            {
                                chosen_expedition_type = Some(expedition_type.target.clone());
                            }
                        }
                    }

                    let can_complete_expedition = is_expedition_still_completeable(chosen_expedition_type.as_ref(), active.current_floor, &*char_name);

                    // checken ob wir genug punkte haben und wechseln entsprechend auf gold
                    let best_index = if active.current_floor == 6 && active.heroism >= 35
                    {
                        try_picking_best_crossroad_based_on_expedition_type_gold(roads, chosen_expedition_type.as_ref(), active.current_floor, &*char_name)
                    }
                    else if active.current_floor == 7 && active.heroism >= 38
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
                    }
                    else if active.current_floor >= 8 && active.heroism >= 40
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
                    }
                    else
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
                    };

                    // Handle the Option to ensure no crashes occur
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
                        return Ok(String::from(""));
                        // return Ok(());
                    }
                }
                ExpeditionStage::Finished =>
                {
                    return Ok(String::from("Finished expedition."));
                    continue;
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
                        return Ok(String::from("Can't go on an expedition because I'm on guard duty"));
                    }
                    CurrentAction::Quest { quest_idx, busy_until } =>
                    {
                        return Ok(String::from("I'm on a quest... can't do expeditions"));
                    }
                    CurrentAction::Expedition =>
                    {
                        // character is still on an expedition, continue processing it
                        session.send_command(Command::ExpeditionContinue).await?;
                        // return Ok(String::from(""));
                    }
                    CurrentAction::Unknown(_) => {},
                }

                if let CurrentAction::Idle = gs.tavern.current_action
                {
                    match gs.tavern.available_tasks()
                    {
                        AvailableTasks::Expeditions(expeditions) =>
                        {
                            if let Some((pos, best_expedition)) = select_best_expedition_exp(&expeditions)
                            {
                                chosen_expedition_type = Some(best_expedition.target.clone());
                                clear_all_encounters_counts(&*gs.character.name);
                                session.send_command(Command::ExpeditionStart { pos }).await?;
                                write_character_log(
                                    &gs.character.name,
                                    gs.character.player_id,
                                    &format!(
                                        "EXPEDITION_EXP: Started Expedition: {:?} (thirst left {}s)",
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
            }
            else
            {
                return Ok(String::from(""));
            }
        }
    }
    Ok(String::from(""))
}

pub fn try_picking_best_crossroad_based_on_expedition_type_exp(encounters: Vec<ExpeditionEncounter>, chosen_expedition_type: Option<&ExpeditionThing>, current_floor: u8, char_name: &str) -> Option<usize>
{
    let index_to_pick = match chosen_expedition_type
    {
        Some(ExpeditionThing::ToiletPaper) => pick_best_crossroads_toilet_paper_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::RevealingCouple) => pick_best_crossroads_revealing_lady_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Balloons) => pick_best_crossroads_bewitched_stew_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Dragon) => pick_best_crossroads_dragon_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Unicorn) => pick_best_crossroads_unicorn_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::WinnersPodium) => pick_best_crossroads_winners_podium_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::BurntCampfire) => pick_best_crossroads_burnt_campfire_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::BrokenSword) => pick_best_crossroads_broken_sword_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::RoyalFrog) => pick_best_crossroads_toxic_fountain_cure_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Klaus) => pick_best_crossroads_klaus_exp(&encounters, current_floor, char_name),
        Some(ExpeditionThing::Cake) => pick_best_crossroads_suckling_pig_exp(&encounters, current_floor, char_name),
        _ => Some(0),
    };

    index_to_pick
}

pub fn pick_best_crossroads_toilet_paper_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();
    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::ToiletPaper, 2.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::ToiletPaper, 2.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::ToiletPaper, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 4.0);
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
        priority_map.insert(ExpeditionThing::Suitcase, 23.0);
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
                priority_map.insert(x.typ, 6.0);
            }
        }
    }
    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }

    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

fn pick_best_crossroads_revealing_lady_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();
    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Socks, 2.0);
        priority_map.insert(ExpeditionThing::ClothPile, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Socks, 2.0);
        priority_map.insert(ExpeditionThing::ClothPile, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 4.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 4.0);
        priority_map.insert(ExpeditionThing::Dragon, 6.0);
        priority_map.insert(ExpeditionThing::Prince, 7.0);
        priority_map.insert(ExpeditionThing::Unicorn, 8.0);
        priority_map.insert(ExpeditionThing::Rainbow, 9.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 10.0);
        priority_map.insert(ExpeditionThing::Phoenix, 11.0);
        priority_map.insert(ExpeditionThing::Cake, 12.0);
        priority_map.insert(ExpeditionThing::Dumy3, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::CupCake, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Dummy2, 17.0);
        priority_map.insert(ExpeditionThing::BentSword, 18.0);
        priority_map.insert(ExpeditionThing::Well, 19.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 20.0);
        priority_map.insert(ExpeditionThing::Dummy1, 21.0);
        priority_map.insert(ExpeditionThing::Suitcase, 22.0);
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
                priority_map.insert(x.typ, 5.0);
            }
        }
    }
    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }

    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_bewitched_stew_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();
    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Well, 2.0);
        priority_map.insert(ExpeditionThing::Girl, 3.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 4.0);
        priority_map.insert(ExpeditionThing::Balloons, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);
        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Well, 2.0);
        priority_map.insert(ExpeditionThing::Girl, 3.0);
        priority_map.insert(ExpeditionThing::BaloonBounty, 4.0);
        priority_map.insert(ExpeditionThing::Balloons, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 4.0);
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
        priority_map.insert(ExpeditionThing::Suitcase, 23.0);
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
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_dragon_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Bait, 2.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 3.0);
        priority_map.insert(ExpeditionThing::Dragon, 4.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Bait, 2.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 3.0);
        priority_map.insert(ExpeditionThing::Dragon, 4.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::Dragon, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 4.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 5.0);
        priority_map.insert(ExpeditionThing::Prince, 7.0);
        priority_map.insert(ExpeditionThing::Unicorn, 8.0);
        priority_map.insert(ExpeditionThing::Rainbow, 9.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 10.0);
        priority_map.insert(ExpeditionThing::Phoenix, 11.0);
        priority_map.insert(ExpeditionThing::Cake, 12.0);
        priority_map.insert(ExpeditionThing::Dumy3, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::CupCake, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Dummy2, 17.0);
        priority_map.insert(ExpeditionThing::BentSword, 18.0);
        priority_map.insert(ExpeditionThing::Well, 19.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 20.0);
        priority_map.insert(ExpeditionThing::Dummy1, 21.0);
        priority_map.insert(ExpeditionThing::Suitcase, 22.0);
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
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_unicorn_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 2.0);
        priority_map.insert(ExpeditionThing::Donkey, 3.0);
        priority_map.insert(ExpeditionThing::Rainbow, 4.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 5.0);
        priority_map.insert(ExpeditionThing::Unicorn, 6.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 2.0);
        priority_map.insert(ExpeditionThing::Donkey, 3.0);
        priority_map.insert(ExpeditionThing::Rainbow, 4.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 5.0);
        priority_map.insert(ExpeditionThing::Unicorn, 6.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 4.0);
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
        priority_map.insert(ExpeditionThing::Suitcase, 23.0);
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
                priority_map.insert(x.typ, 6.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_winners_podium_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();
    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 5.1);
        priority_map.insert(ExpeditionThing::SmallHurdle, 2.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 3.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 5.2);
        priority_map.insert(ExpeditionThing::WinnersPodium, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 6.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 5.1);
        priority_map.insert(ExpeditionThing::SmallHurdle, 2.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 3.0);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 5.2);
        priority_map.insert(ExpeditionThing::WinnersPodium, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 6.0);
        priority_map.insert(ExpeditionThing::Dragon, 7.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 4.0);
        priority_map.insert(ExpeditionThing::Dragon, 6.0);
        priority_map.insert(ExpeditionThing::Prince, 7.0);
        priority_map.insert(ExpeditionThing::Unicorn, 8.0);
        priority_map.insert(ExpeditionThing::Rainbow, 9.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 10.0);
        priority_map.insert(ExpeditionThing::Phoenix, 11.0);
        priority_map.insert(ExpeditionThing::Cake, 12.0);
        priority_map.insert(ExpeditionThing::Dumy3, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::CupCake, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Dummy2, 17.0);
        priority_map.insert(ExpeditionThing::BentSword, 18.0);
        priority_map.insert(ExpeditionThing::Well, 19.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 20.0);
        priority_map.insert(ExpeditionThing::Dummy1, 21.0);
        priority_map.insert(ExpeditionThing::Suitcase, 22.0);
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
                priority_map.insert(x.typ, 5.0);
            }
        }
    }
    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_burnt_campfire_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::CampFire, 2.0);
        priority_map.insert(ExpeditionThing::Phoenix, 3.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 4.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::CampFire, 2.0);
        priority_map.insert(ExpeditionThing::Phoenix, 3.0);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 14.5);
        priority_map.insert(ExpeditionThing::BurntCampfire, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 4.0);
        priority_map.insert(ExpeditionThing::Dragon, 6.0);
        priority_map.insert(ExpeditionThing::Prince, 7.0);
        priority_map.insert(ExpeditionThing::Unicorn, 8.0);
        priority_map.insert(ExpeditionThing::Rainbow, 9.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 10.0);
        priority_map.insert(ExpeditionThing::Phoenix, 11.0);
        priority_map.insert(ExpeditionThing::Cake, 12.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::Suitcase, 23.0);
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
                priority_map.insert(x.typ, 5.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_broken_sword_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();
    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 2.0);
        priority_map.insert(ExpeditionThing::BentSword, 3.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 4.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 2.0);
        priority_map.insert(ExpeditionThing::BentSword, 3.0);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 14.5);
        priority_map.insert(ExpeditionThing::BrokenSword, 5.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 4.0);
        priority_map.insert(ExpeditionThing::Dragon, 6.0);
        priority_map.insert(ExpeditionThing::Prince, 7.0);
        priority_map.insert(ExpeditionThing::Unicorn, 8.0);
        priority_map.insert(ExpeditionThing::Rainbow, 9.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 10.0);
        priority_map.insert(ExpeditionThing::Phoenix, 11.0);
        priority_map.insert(ExpeditionThing::Cake, 12.0);
        priority_map.insert(ExpeditionThing::Dumy3, 13.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::Suitcase, 23.0);
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
                priority_map.insert(x.typ, 5.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_toxic_fountain_cure_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Prince, 2.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 3.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 4.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Prince, 2.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 14.5);
        priority_map.insert(ExpeditionThing::RoyalFrog, 4.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 4.0);
        priority_map.insert(ExpeditionThing::Dragon, 6.0);
        priority_map.insert(ExpeditionThing::Prince, 7.0);
        priority_map.insert(ExpeditionThing::Unicorn, 8.0);
        priority_map.insert(ExpeditionThing::Rainbow, 9.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 10.0);
        priority_map.insert(ExpeditionThing::Phoenix, 11.0);
        priority_map.insert(ExpeditionThing::Cake, 12.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 13.0);
        priority_map.insert(ExpeditionThing::Dumy3, 14.0);
        priority_map.insert(ExpeditionThing::Donkey, 15.0);
        priority_map.insert(ExpeditionThing::CupCake, 16.0);
        priority_map.insert(ExpeditionThing::CampFire, 17.0);
        priority_map.insert(ExpeditionThing::Dummy2, 18.0);
        priority_map.insert(ExpeditionThing::BentSword, 19.0);
        priority_map.insert(ExpeditionThing::Well, 20.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 21.0);
        priority_map.insert(ExpeditionThing::Dummy1, 22.0);
        priority_map.insert(ExpeditionThing::Suitcase, 23.0);
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
                priority_map.insert(x.typ, 5.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_klaus_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 4.5);
        priority_map.insert(ExpeditionThing::Hand, 2.0);
        priority_map.insert(ExpeditionThing::Feet, 3.0);
        priority_map.insert(ExpeditionThing::Body, 4.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 5.0);
        priority_map.insert(ExpeditionThing::Klaus, 6.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 4.5);
        priority_map.insert(ExpeditionThing::Hand, 2.0);
        priority_map.insert(ExpeditionThing::Feet, 3.0);
        priority_map.insert(ExpeditionThing::Body, 4.0);
        priority_map.insert(ExpeditionThing::KlausBounty, 5.0);
        priority_map.insert(ExpeditionThing::Klaus, 6.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Cake, 42.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 4.0);
        priority_map.insert(ExpeditionThing::Dragon, 6.0);
        priority_map.insert(ExpeditionThing::Prince, 7.0);
        priority_map.insert(ExpeditionThing::Unicorn, 8.0);
        priority_map.insert(ExpeditionThing::Rainbow, 9.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 10.0);
        priority_map.insert(ExpeditionThing::Phoenix, 11.0);
        priority_map.insert(ExpeditionThing::Cake, 12.0);
        priority_map.insert(ExpeditionThing::Dumy3, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::CupCake, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Dummy2, 17.0);
        priority_map.insert(ExpeditionThing::BentSword, 18.0);
        priority_map.insert(ExpeditionThing::Well, 19.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 20.0);
        priority_map.insert(ExpeditionThing::Dummy1, 21.0);
        priority_map.insert(ExpeditionThing::Suitcase, 22.0);
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
                priority_map.insert(x.typ, 5.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }

    picked_index
}

pub fn pick_best_crossroads_suckling_pig_exp(encounters: &[ExpeditionEncounter], current_floor: u8, char_name: &str) -> Option<usize>
{
    let mut priority_map = HashMap::new();

    if current_floor == 1
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Cake, 2.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::ClothPile, 30.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 31.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 33.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 34.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::Girl, 36.0);
        priority_map.insert(ExpeditionThing::Balloons, 37.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }

    if current_floor > 1 && current_floor < 10
    {
        priority_map.insert(ExpeditionThing::DummyBounty, 1.0);
        priority_map.insert(ExpeditionThing::Cake, 19.5);
        priority_map.insert(ExpeditionThing::UnicornHorn, 9.0);
        priority_map.insert(ExpeditionThing::Donkey, 10.0);
        priority_map.insert(ExpeditionThing::Rainbow, 11.0);
        priority_map.insert(ExpeditionThing::Unicorn, 12.0);
        priority_map.insert(ExpeditionThing::Bait, 13.0);
        priority_map.insert(ExpeditionThing::Dragon, 14.0);
        priority_map.insert(ExpeditionThing::Dumy3, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Phoenix, 17.0);
        priority_map.insert(ExpeditionThing::Prince, 18.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 19.0);
        priority_map.insert(ExpeditionThing::ClothPile, 20.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 21.0);
        priority_map.insert(ExpeditionThing::BigHurdle, 22.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 23.0);
        priority_map.insert(ExpeditionThing::Girl, 24.0);
        priority_map.insert(ExpeditionThing::Balloons, 25.0);
        priority_map.insert(ExpeditionThing::CupCake, 26.0);
        priority_map.insert(ExpeditionThing::Dummy2, 27.0);
        priority_map.insert(ExpeditionThing::Dummy1, 28.0);
        priority_map.insert(ExpeditionThing::Socks, 29.0);
        priority_map.insert(ExpeditionThing::SmallHurdle, 32.0);
        priority_map.insert(ExpeditionThing::Well, 35.0);
        priority_map.insert(ExpeditionThing::BentSword, 38.0);
        priority_map.insert(ExpeditionThing::DragonBounty, 39.0);
        priority_map.insert(ExpeditionThing::FrogBounty, 39.01);
        priority_map.insert(ExpeditionThing::UnicornBounty, 39.02);
        priority_map.insert(ExpeditionThing::BurntCampfireBounty, 39.03);
        priority_map.insert(ExpeditionThing::BaloonBounty, 39.04);
        priority_map.insert(ExpeditionThing::WinnerPodiumBounty, 39.05);
        priority_map.insert(ExpeditionThing::RevealingCoupleBounty, 39.06);
        priority_map.insert(ExpeditionThing::BrokenSwordBounty, 39.07);
        priority_map.insert(ExpeditionThing::KlausBounty, 39.08);
        priority_map.insert(ExpeditionThing::Key, 40.0);
        priority_map.insert(ExpeditionThing::Suitcase, 41.0);
        priority_map.insert(ExpeditionThing::Hand, 43.0);
        priority_map.insert(ExpeditionThing::Feet, 44.0);
        priority_map.insert(ExpeditionThing::Body, 45.0);
        priority_map.insert(ExpeditionThing::Klaus, 46.0);
        priority_map.insert(ExpeditionThing::BurntCampfire, 47.0);
        priority_map.insert(ExpeditionThing::RoyalFrog, 48.0);
        priority_map.insert(ExpeditionThing::BrokenSword, 49.0);

        for x in encounters
        {
            if x.heroism >= 10
            {
                priority_map.insert(x.typ, 7.0);
            }
            if x.heroism == 5 && x.typ != ExpeditionThing::Cake && x.typ != ExpeditionThing::SwordInStone
            {
                priority_map.insert(x.typ, 8.0);
            }
        }
        if get_encounter_count(char_name, ExpeditionThing::Bait) == 1
        {
            priority_map.insert(ExpeditionThing::DragonBounty, 1.5);
        }
    }
    if current_floor == 10
    {
        priority_map.insert(ExpeditionThing::Klaus, 1.0);
        priority_map.insert(ExpeditionThing::WinnersPodium, 2.0);
        priority_map.insert(ExpeditionThing::Balloons, 3.0);
        priority_map.insert(ExpeditionThing::RevealingCouple, 4.0);
        priority_map.insert(ExpeditionThing::Dragon, 6.0);
        priority_map.insert(ExpeditionThing::Cake, 7.0);
        priority_map.insert(ExpeditionThing::Prince, 8.0);
        priority_map.insert(ExpeditionThing::Unicorn, 9.0);
        priority_map.insert(ExpeditionThing::Rainbow, 10.0);
        priority_map.insert(ExpeditionThing::SwordInStone, 11.0);
        priority_map.insert(ExpeditionThing::Phoenix, 12.0);
        priority_map.insert(ExpeditionThing::Dumy3, 13.0);
        priority_map.insert(ExpeditionThing::Donkey, 14.0);
        priority_map.insert(ExpeditionThing::CupCake, 15.0);
        priority_map.insert(ExpeditionThing::CampFire, 16.0);
        priority_map.insert(ExpeditionThing::Dummy2, 17.0);
        priority_map.insert(ExpeditionThing::BentSword, 18.0);
        priority_map.insert(ExpeditionThing::Well, 19.0);
        priority_map.insert(ExpeditionThing::UnicornHorn, 20.0);
        priority_map.insert(ExpeditionThing::Dummy1, 21.0);
        priority_map.insert(ExpeditionThing::Suitcase, 22.0);
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
                priority_map.insert(x.typ, 5.0);
            }
        }
    }

    if get_encounter_count(char_name, ExpeditionThing::Cake) >= 1
    {
        priority_map.insert(ExpeditionThing::Cake, 19.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::RoyalFrog) >= 1
    {
        priority_map.insert(ExpeditionThing::RoyalFrog, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BurntCampfire) >= 1
    {
        priority_map.insert(ExpeditionThing::BurntCampfire, 14.5);
    }
    if get_encounter_count(char_name, ExpeditionThing::BrokenSword) >= 1
    {
        priority_map.insert(ExpeditionThing::BrokenSword, 14.5);
    }
    let picked_index = pick_best_encounter_exp(encounters, &priority_map);

    if let Some(index) = picked_index
    {
        let picked_encounter = encounters[index].typ;
        increment_encounter_count(char_name, picked_encounter);
    }
    picked_index
}
pub fn select_best_expedition_exp(expeditions: &[AvailableExpedition]) -> Option<(usize, &AvailableExpedition)>
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
            let target_name = format!("{:?}", best_expedition.target);

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
            ExpeditionThing::WinnersPodium => Some(3.2),
            ExpeditionThing::Dragon => Some(1.1),
            ExpeditionThing::Unicorn => Some(3.0),
            ExpeditionThing::Klaus => Some(1.0),
            ExpeditionThing::ToiletPaper => Some(3.1),
            ExpeditionThing::Cake => Some(5.0),
            ExpeditionThing::RevealingCouple => Some(7.0),
            ExpeditionThing::BurntCampfire => Some(7.2),
            ExpeditionThing::Balloons => Some(7.1),
            ExpeditionThing::RoyalFrog => Some(9.0),
            ExpeditionThing::BrokenSword => Some(9.1),
            _ => None,
        }
    }
}

pub fn pick_best_encounter_exp(encounters: &[ExpeditionEncounter], priority_map: &HashMap<ExpeditionThing, f64>) -> Option<usize>
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
        }
    }
    best_index
}
