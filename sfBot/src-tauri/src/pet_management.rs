#![allow(warnings)]

use std::{borrow::Borrow, collections::HashMap, error::Error, fmt::Debug, time::Duration};

use chrono::{DateTime, Local};
use serde_json::json;
use sf_api::{
    command::{AttributeType, Command},
    error::SFError,
    gamestate::{
        rewards::Event,
        unlockables::{HabitatExploration, HabitatType, Pet, Pets},
        GameState,
    },
    misc::EnumMapGet,
    SimpleSession,
};

use crate::{bot_runner::write_character_log, city_guard::sleep_between_commands};

pub async fn feed_all_pets(session: &mut SimpleSession, feed_pets: bool, expensive_route: bool, max_pets_to_a_day: usize) -> Result<String, Box<dyn Error>>
{
    if (!feed_pets)
    {
        return Ok("".to_string());
    }

    let pet_id_mapping_shadow = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
    let pet_id_mapping_light = vec![21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40];
    let pet_id_mapping_earth = vec![41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60];
    let pet_id_mapping_fire = vec![61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80];
    let pet_id_mapping_water = vec![81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100];

    let transform_to_pet_id_and_levels = |pet_id_mapping: Vec<u32>, desired_levels: Vec<u16>| -> Vec<Option<(u32, u16)>> {
        pet_id_mapping
            .iter()
            .zip(desired_levels.iter())
            .map(|(&pet_id, &desired_level)| {
                if desired_level > 0
                {
                    Some((pet_id, desired_level))
                }
                else
                {
                    None
                }
            })
            .collect()
    };

    // Habitat-specific desired levels
    let desired_levels_shadow = if (expensive_route)
    {
        vec![0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 200]
    }
    else
    {
        vec![0, 0, 40, 0, 0, 0, 0, 0, 87, 0, 0, 0, 0, 0, 0, 98, 0, 0, 0, 200]
    };
    let desired_levels_light = if (expensive_route)
    {
        vec![0, 0, 76, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 74, 0, 200]
    }
    else
    {
        vec![0, 0, 93, 0, 0, 0, 0, 0, 0, 0, 0, 0, 91, 0, 0, 0, 74, 0, 200]
    };
    let desired_levels_earth = if (expensive_route)
    {
        vec![0, 0, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 200]
    }
    else
    {
        vec![0, 0, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 84, 0, 0, 0, 78, 0, 200]
    };
    let desired_levels_fire = if (expensive_route)
    {
        vec![0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 200]
    }
    else
    {
        vec![0, 0, 24, 0, 0, 0, 97, 0, 0, 0, 0, 0, 0, 0, 0, 94, 0, 0, 0, 200]
    };
    let desired_levels_water = if (expensive_route)
    {
        vec![0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 200]
    }
    else
    {
        vec![0, 0, 39, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 200]
    };

    // Create transformed pet ID and level vectors for each habitat
    let transformed_shadow = transform_to_pet_id_and_levels(pet_id_mapping_shadow, desired_levels_shadow);
    let transformed_light = transform_to_pet_id_and_levels(pet_id_mapping_light, desired_levels_light);
    let transformed_earth = transform_to_pet_id_and_levels(pet_id_mapping_earth, desired_levels_earth);
    let transformed_fire = transform_to_pet_id_and_levels(pet_id_mapping_fire, desired_levels_fire);
    let transformed_water = transform_to_pet_id_and_levels(pet_id_mapping_water, desired_levels_water);

    // Feed pets based on habitat and desired levels
    feed_pets_hardcoded_best_route(session, transformed_shadow, HabitatType::Shadow, max_pets_to_a_day).await?;
    feed_pets_hardcoded_best_route(session, transformed_light, HabitatType::Light, max_pets_to_a_day).await?;
    feed_pets_hardcoded_best_route(session, transformed_earth, HabitatType::Earth, max_pets_to_a_day).await?;
    feed_pets_hardcoded_best_route(session, transformed_fire, HabitatType::Fire, max_pets_to_a_day).await?;
    feed_pets_hardcoded_best_route(session, transformed_water, HabitatType::Water, max_pets_to_a_day).await?;

    Ok("".to_string())
}

async fn refresh_pet_state(
    session: &mut SimpleSession,
    habitat_type: HabitatType,
    pet_id: u32,
) -> Result<Option<(u16, u16, usize, usize)>, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let pets = match &gs.pets
    {
        Some(pets) => pets,
        None => return Ok(None),
    };

    let habitat = &pets.habitats[habitat_type];
    let fruits_available = habitat.fruits as usize;
    let total_fruits = (pets.habitats[HabitatType::Shadow].fruits
        + pets.habitats[HabitatType::Light].fruits
        + pets.habitats[HabitatType::Earth].fruits
        + pets.habitats[HabitatType::Fire].fruits
        + pets.habitats[HabitatType::Water].fruits) as usize;

    let pet = match habitat.pets.iter().find(|pet| pet.id == pet_id)
    {
        Some(pet) => pet,
        None => return Ok(None),
    };

    Ok(Some((pet.level, pet.fruits_today, fruits_available, total_fruits)))
}

async fn try_feed_pet(session: &mut SimpleSession, pet_id: u32, fruit_idx: u32) -> Result<bool, Box<dyn Error>>
{
    match session.send_command(Command::PetFeed { pet_id, fruit_idx }).await
    {
        Ok(_) => Ok(true),
        Err(SFError::ServerError(msg)) =>
        {
            if msg.contains("maxed out")
            {
                Ok(false)
            } else {
                Err(Box::new(SFError::ServerError(msg)))
            }
        }
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn feed_pets_hardcoded_best_route(session: &mut SimpleSession, desired_levels_vec: Vec<Option<(u32, u16)>>, habitat_type: HabitatType, max_pets_to_a_day: usize) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    if gs.pets.is_none()
    {
        return Ok(("".to_string()));
    }
    let character_name = gs.character.name.clone();
    let character_id = gs.character.player_id;
    struct FeedSummary
    {
        habitat: HabitatType,
        feeds: u16,
        level: u16,
        feeds_today: u16,
    }
    let mut feed_summaries: HashMap<u32, FeedSummary> = HashMap::new();
    let mut pet_state_cache: HashMap<u32, (u16, u16)> = HashMap::new();

    let mut available_pets = get_pets_with_minimum_level_and_id(&gs, 1, habitat_type);
    available_pets.sort_by(|a, b| b.0.cmp(&a.0)); // ort pets by descending level

    let mut events = &gs.specials.events.active;
    let pet_max_feed_amount = if events.contains(&Event::AssemblyOfAwesomeAnimals) { 9 } else { 3 };
    let max_feeds_per_day = pet_max_feed_amount * max_pets_to_a_day;
    let mut feed_counter = 0;
    let pet_max_level = gs.pets.as_ref().map(|pets| pets.max_pet_level).unwrap_or(200);

    let mut fruits_available = match gs.pets
    {
        None => return Ok(("".to_string())),
        Some(ref pets) => pets.habitats[habitat_type].fruits as usize,
    };
    let mut total_fruits = get_total_available_fruit_count(gs.pets);

    if fruits_available == 0
    {
        return Ok(("".to_string()));
    }

    // handle partially fed pets
    let mut partially_fed_pet: Option<(usize, &Pet)> = None;
    for (index, pet) in available_pets.iter()
    {
        if pet.fruits_today > 0
        {
            feed_counter += pet.fruits_today as usize;

            if pet.fruits_today < pet_max_feed_amount as u16
            {
                partially_fed_pet = Some((*index, pet));
            }
        }
    }

    // feed partially fed pet
    if let Some((index, pet)) = partially_fed_pet
    {
        let (mut pet_level, mut pet_fruits_today) = pet_state_cache
            .get(&pet.id)
            .copied()
            .unwrap_or((pet.level, pet.fruits_today));
        while pet_fruits_today < pet_max_feed_amount as u16 && feed_counter < max_feeds_per_day && fruits_available > 0
        {
            if pet_level >= pet_max_level
            {
                break;
            }

            if !try_feed_pet(session, pet.id, total_fruits as u32).await?
            {
                pet_level = pet_max_level;
                break;
            }
            feed_counter += 1;
            if let Some((level, fruits_today, new_fruits, new_total)) =
                refresh_pet_state(session, habitat_type, pet.id).await?
            {
                pet_level = level;
                pet_fruits_today = fruits_today;
                fruits_available = new_fruits;
                total_fruits = new_total;
                let entry = feed_summaries.entry(pet.id).or_insert(FeedSummary {
                    habitat: habitat_type,
                    feeds: 0,
                    level: pet_level,
                    feeds_today: pet_fruits_today,
                });
                entry.feeds += 1;
                entry.level = pet_level;
                entry.feeds_today = pet_fruits_today;
                pet_state_cache.insert(pet.id, (pet_level, pet_fruits_today));
            }
            else
            {
                break;
            }

        }
    }
    // feed pets based on their desired level and id from the vec desired_levels_vec
    for (index, pet) in available_pets.iter()
    {
        if let Some(Some((pet_id, desired_level))) = desired_levels_vec.get(*index)
        {
            let desired_level = (*desired_level).min(pet_max_level);
            let (mut pet_level, mut pet_fruits_today) = pet_state_cache
                .get(&pet.id)
                .copied()
                .unwrap_or((pet.level, pet.fruits_today));
            if pet_level >= desired_level || pet_fruits_today >= pet_max_feed_amount as u16
            {
                continue;
            }

            while pet_level < desired_level && pet_fruits_today < pet_max_feed_amount as u16 && feed_counter < max_feeds_per_day && fruits_available > 0
            {
                if !try_feed_pet(session, pet.id, total_fruits as u32).await?
                {
                    pet_level = pet_max_level;
                    break;
                }
                feed_counter += 1;
                if let Some((level, fruits_today, new_fruits, new_total)) =
                    refresh_pet_state(session, habitat_type, pet.id).await?
                {
                    pet_level = level;
                    pet_fruits_today = fruits_today;
                    fruits_available = new_fruits;
                    total_fruits = new_total;
                    let entry = feed_summaries.entry(pet.id).or_insert(FeedSummary {
                        habitat: habitat_type,
                        feeds: 0,
                        level: pet_level,
                        feeds_today: pet_fruits_today,
                    });
                    entry.feeds += 1;
                    entry.level = pet_level;
                    entry.feeds_today = pet_fruits_today;
                    pet_state_cache.insert(pet.id, (pet_level, pet_fruits_today));
                }
                else
                {
                    break;
                }

                if feed_counter >= max_feeds_per_day
                {
                    return Ok(("".to_string()));
                }
            }
        }
    }

    // Explanation: we need to check whether the pet has reached their
    // desired level and was unlocked before start feeding anything else, in the
    // future people might want to level  further than the desired level so we might
    // have to account for that or simply replace the lists with desired levels
    // of 100 and or add a boolean that will continue feeding up to 100 instead
    // of the ones that are currently provided
    let fed_all_desired = desired_levels_vec.iter().all(|desired_pet| {
        match *desired_pet
        {
            Some((desired_pet_id, desired_level)) =>
            {
                // check  pet exists in available_pets and matches  desired pet_id
                if let Some((_, pet)) = available_pets.iter().find(|(_, pet)| pet.id == desired_pet_id)
                {
                    pet.level >= desired_level
                }
                else
                {
                    // some desired pet isnt available yet so we dont feed further pets
                    false
                }
            }
            None =>
            {
                //  no desired level is specified for this pet  treat as fed
                true
            }
        }
    });

    let highest_desired_at_max = match desired_levels_vec
        .iter()
        .filter_map(|desired_pet| desired_pet.as_ref())
        .max_by_key(|(_, desired_level)| *desired_level)
    {
        Some((desired_pet_id, desired_level)) if *desired_level >= pet_max_level =>
        {
            available_pets
                .iter()
                .find(|(_, pet)| pet.id == *desired_pet_id)
                .map(|(_, pet)| pet.level >= pet_max_level)
                .unwrap_or(false)
        }
        _ => false,
    };

    // no other pet should be fed until all desired pet levels are reached,
    // unless the highest desired pet is already maxed out
    if (fed_all_desired || highest_desired_at_max) && feed_counter < max_feeds_per_day
    {
        for (index, pet) in available_pets.iter()
        {
            let (mut pet_level, mut pet_fruits_today) = pet_state_cache
                .get(&pet.id)
                .copied()
                .unwrap_or((pet.level, pet.fruits_today));
            if pet_fruits_today >= pet_max_feed_amount as u16 || pet_level >= pet_max_level
            {
                continue;
            }

            while pet_fruits_today < pet_max_feed_amount as u16
                && pet_level < pet_max_level
                && feed_counter < max_feeds_per_day
                && fruits_available > 0
            {
                if !try_feed_pet(session, pet.id, total_fruits as u32).await?
                {
                    pet_level = pet_max_level;
                    break;
                }
                feed_counter += 1;
                if let Some((level, fruits_today, new_fruits, new_total)) =
                    refresh_pet_state(session, habitat_type, pet.id).await?
                {
                    pet_level = level;
                    pet_fruits_today = fruits_today;
                    fruits_available = new_fruits;
                    total_fruits = new_total;
                    let entry = feed_summaries.entry(pet.id).or_insert(FeedSummary {
                        habitat: habitat_type,
                        feeds: 0,
                        level: pet_level,
                        feeds_today: pet_fruits_today,
                    });
                    entry.feeds += 1;
                    entry.level = pet_level;
                    entry.feeds_today = pet_fruits_today;
                    pet_state_cache.insert(pet.id, (pet_level, pet_fruits_today));
                }
                else
                {
                    break;
                }

                if feed_counter >= max_feeds_per_day
                {
                    return Ok(("".to_string()));
                }
            }
        }
    }

    if !feed_summaries.is_empty()
    {
        for (pet_id, summary) in feed_summaries
        {
            write_character_log(
                &character_name,
                character_id,
                &format!(
                    "PETS: Fed pet {} ({:?}) level {} (+{} feeds, feeds today: {})",
                    pet_id, summary.habitat, summary.level, summary.feeds, summary.feeds_today
                ),
            );
        }
    }

    Ok(("".to_string()))
}

pub fn get_total_available_fruit_count(pets: Option<Pets>) -> usize
{
    match pets
    {
        None => 0,
        Some(ref pets) =>
        {
            let mut count = 0;
            count += pets.habitats[HabitatType::Shadow].fruits;
            count += pets.habitats[HabitatType::Light].fruits;
            count += pets.habitats[HabitatType::Earth].fruits;
            count += pets.habitats[HabitatType::Fire].fruits;
            count += pets.habitats[HabitatType::Water].fruits;
            count as usize
        }
    }
}

pub fn get_pets_with_minimum_level_and_id(gs: &GameState, min_level: u16, habitat_type: HabitatType) -> Vec<(usize, Pet)>
{
    let pets = match &gs.pets
    {
        Some(pets) => pets,
        None => return vec![],
    };

    let owned_pets = &pets.habitats[habitat_type].pets;

    owned_pets.iter().enumerate().filter(|(_, pet)| pet.level >= min_level).map(|(index, pet)| (index, pet.clone())).collect()
}

pub fn get_pets_with_minimum_level(gs: GameState, min_level: u16, habitat_type: HabitatType) -> Vec<Pet>
{
    let pets = match &gs.pets
    {
        Some(pets) => pets,
        None => return vec![],
    };

    let owned_pets = &pets.habitats[habitat_type].pets;
    let mut filtered_pets = owned_pets.iter().filter(|pet| pet.level >= min_level).cloned().collect::<Vec<Pet>>();

    filtered_pets.sort_by(|a, b| b.level.cmp(&a.level));

    filtered_pets
}

pub async fn fight_pet_dungeon(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    let pets = match gs.pets
    {
        Some(pets) => pets,
        None => return Ok(("".to_string())),
    };

    let is_fight_free = match pets.next_free_exploration
    {
        Some(habitat_free_fight_time) => habitat_free_fight_time < Local::now(),
        None => true,
    };

    if (!is_fight_free)
    {
        return Ok("".to_string());
    }

    let result = fight_pet_dungeon_impl(session).await?;

    return Ok(result);
}

pub async fn fight_pet_dungeon_impl(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    let highest_luck_shadow = get_pet_with_highest_luck_for_habitat(session, &gs, HabitatType::Shadow).await;
    let highest_luck_light = get_pet_with_highest_luck_for_habitat(session, &gs, HabitatType::Light).await;
    let highest_luck_earth = get_pet_with_highest_luck_for_habitat(session, &gs, HabitatType::Earth).await;
    let highest_luck_fire = get_pet_with_highest_luck_for_habitat(session, &gs, HabitatType::Fire).await;
    let highest_luck_water = get_pet_with_highest_luck_for_habitat(session, &gs, HabitatType::Water).await;

    let pets = match gs.pets
    {
        Some(pets) => pets,
        None => return Ok(("".to_string())),
    };

    let habitats_map = pets.habitats;
    let shadow_hab = &habitats_map[HabitatType::Shadow].exploration;
    let light_hab = &habitats_map[HabitatType::Light].exploration;
    let earth_hab = &habitats_map[HabitatType::Earth].exploration;
    let fire_hab = &habitats_map[HabitatType::Fire].exploration;
    let water_hab = &habitats_map[HabitatType::Water].exploration;

    let shadow_hab_stage = match shadow_hab
    {
        HabitatExploration::Finished => 100,
        HabitatExploration::Exploring { fights_won, next_fight_lvl } => *fights_won,
    };
    let light_hab_stage = match light_hab
    {
        HabitatExploration::Finished => 100,
        HabitatExploration::Exploring { fights_won, next_fight_lvl } => *fights_won,
    };
    let earth_hab_stage = match earth_hab
    {
        HabitatExploration::Finished => 100,
        HabitatExploration::Exploring { fights_won, next_fight_lvl } => *fights_won,
    };
    let fire_hab_stage = match fire_hab
    {
        HabitatExploration::Finished => 100,
        HabitatExploration::Exploring { fights_won, next_fight_lvl } => *fights_won,
    };
    let water_hab_stage = match water_hab
    {
        HabitatExploration::Finished => 100,
        HabitatExploration::Exploring { fights_won, next_fight_lvl } => *fights_won,
    };

    if (water_hab_stage == 100 && fire_hab_stage == 100 && light_hab_stage == 100 && earth_hab_stage == 100 && shadow_hab_stage == 100)
    {
        // return Ok(("Finished all pet dungeons".to_string()));
        return Ok(String::from(""));
    }

    let mut best_stat_relation = 0.0;
    let mut habitat_to_attack: HabitatType = HabitatType::Shadow; // provide default
    let mut pet_id_to_use = 0;
    let mut enemy_hab_pos = 0;

    if (shadow_hab_stage < 20)
    {
        let shadow_enemy_stats = get_current_habitat_enemy(shadow_hab_stage);
        match highest_luck_shadow
        {
            Some((habitat_type, pet_id, luck_stat)) =>
            {
                let result = (luck_stat as f64 / shadow_enemy_stats as f64);
                if (result > best_stat_relation)
                {
                    best_stat_relation = result;
                    habitat_to_attack = habitat_type;
                    pet_id_to_use = pet_id;
                    enemy_hab_pos = shadow_hab_stage;
                }
            }
            _ =>
            {}
        }
    }

    if (light_hab_stage < 20)
    {
        let light_enemy_stats = get_current_habitat_enemy(light_hab_stage);
        match highest_luck_light
        {
            Some((habitat_type, pet_id, luck_stat)) =>
            {
                let result = (luck_stat as f64 / light_enemy_stats as f64);
                if (result > best_stat_relation)
                {
                    best_stat_relation = result;
                    habitat_to_attack = habitat_type;
                    pet_id_to_use = pet_id;
                    enemy_hab_pos = light_hab_stage;
                }
            }
            _ =>
            {}
        }
    }

    if (earth_hab_stage < 20)
    {
        let earth_enemy_stats = get_current_habitat_enemy(earth_hab_stage);
        match highest_luck_earth
        {
            Some((habitat_type, pet_id, luck_stat)) =>
            {
                let result = (luck_stat as f64 / earth_enemy_stats as f64);
                if (result > best_stat_relation)
                {
                    best_stat_relation = result;
                    habitat_to_attack = habitat_type;
                    pet_id_to_use = pet_id;
                    enemy_hab_pos = earth_hab_stage;
                }
            }
            _ =>
            {}
        }
    }

    if (fire_hab_stage < 20)
    {
        let fire_enemy_stats = get_current_habitat_enemy(fire_hab_stage);
        match highest_luck_fire
        {
            Some((habitat_type, pet_id, luck_stat)) =>
            {
                let result = (luck_stat as f64 / fire_enemy_stats as f64);
                if (result > best_stat_relation)
                {
                    best_stat_relation = result;
                    habitat_to_attack = habitat_type;
                    pet_id_to_use = pet_id;
                    enemy_hab_pos = fire_hab_stage;
                }
            }
            _ =>
            {}
        }
    }

    if (water_hab_stage < 20)
    {
        let water_enemy_stats = get_current_habitat_enemy(water_hab_stage);
        match highest_luck_water
        {
            Some((habitat_type, pet_id, luck_stat)) =>
            {
                let result = (luck_stat as f64 / water_enemy_stats as f64);
                if (result > best_stat_relation)
                {
                    best_stat_relation = result;
                    habitat_to_attack = habitat_type;
                    pet_id_to_use = pet_id;
                    enemy_hab_pos = water_hab_stage;
                }
            }
            _ =>
            {}
        }
    }

    let msg = format!("Fighting habitat {:?} enemy at position {}: with stat relation: {} using pet id: {}", habitat_to_attack, enemy_hab_pos + 1, best_stat_relation, pet_id_to_use);
    if (pet_id_to_use == 0)
    {
        return Ok(String::from(""));
    }
    session
        .send_command(Command::FightPetDungeon {
            use_mush: false,
            habitat: habitat_to_attack,
            enemy_pos: enemy_hab_pos + 1,
            player_pet_id: pet_id_to_use - 1,
        })
        .await?;

    return Ok(msg);
}

pub async fn get_pet_with_highest_luck_for_habitat(session: &mut SimpleSession, gs: &GameState, habitat_type: HabitatType) -> Option<(HabitatType, u32, u32)>
{
    sleep_between_commands(20).await;
    let available_pets = get_pets_with_minimum_level(gs.clone(), 1, habitat_type);

    let mut highest_luck_stat = 0;
    let mut highest_luck_pet_id = 0;

    for pet in &available_pets
    {
        let pet_stats_result = session.send_command(Command::ViewPet { pet_id: pet.id as u16 }).await;

        match pet_stats_result
        {
            Ok(_) =>
            {
                let new_gs_result = session.send_command(Command::Update).await;

                match new_gs_result
                {
                    Ok(new_gs) =>
                    {
                        let new_pets = &new_gs.pets;

                        let unwrapped_pets = match new_pets
                        {
                            Some(pets) => pets,
                            None => continue,
                        };

                        let new_habitat = &unwrapped_pets.habitats[habitat_type];
                        let new_pet_array = &new_habitat.pets;

                        for x in new_pet_array
                        {
                            if let Some(stats) = &x.stats
                            {
                                if stats.attributes[AttributeType::Luck] > highest_luck_stat
                                {
                                    highest_luck_stat = stats.attributes[AttributeType::Luck];
                                    highest_luck_pet_id = x.id;
                                }
                            }
                        }
                    }
                    Err(_) =>
                    {
                        continue;
                    }
                }
            }
            Err(_) =>
            {
                continue;
            }
        }
    }

    if highest_luck_stat > 0
    {
        Some((habitat_type, highest_luck_pet_id, highest_luck_stat))
    }
    else
    {
        None
    }
}

pub async fn fight_pet_arena(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    if (gs.pets.is_none())
    {
        return Ok(String::from(""));
    }

    let pet_arena_opponent = match gs.pets
    {
        None => return Ok(String::from("")),
        Some(ref pets) => &pets.opponent,
    };

    // when pets are unlocked next free fight is always none until the next day or a
    // fight happened
    let next_free_battle = pet_arena_opponent.next_free_battle.unwrap_or_else(Local::now);

    let is_the_next_fight_free = next_free_battle <= *&Local::now() || pet_arena_opponent.next_free_battle.is_none();

    if !is_the_next_fight_free
    {
        return Ok(String::from(""));
    }

    let enemy_playerid = pet_arena_opponent.id;
    let enemy_pets_level = pet_arena_opponent.level_total;
    let enemys_daily_habitat = match pet_arena_opponent.habitat
    {
        None => return Ok(String::from("")),
        Some(habitat) => habitat,
    };

    if let Some((best_habitat, best_level)) = decide_next_pet_to_fight(&gs, enemys_daily_habitat, enemy_pets_level as u16)
    {
        let command = Command::FightPetOpponent { opponent_id: enemy_playerid, habitat: best_habitat };
        session.send_command(command).await?;
        return Ok(String::from(format!("The best habitat to fight is {:?}.", best_habitat)));
    }
    return Ok(String::from(""));
}
pub fn get_total_level_owned_pets_by_habitat(gs: &GameState, habitat_type: HabitatType) -> u16
{
    let mut total_pet_level = match &gs.pets
    {
        None => return 0,
        Some(pets) =>
        {
            let ownned_pets = &pets.habitats[habitat_type].pets;
            let mut sum_level = 0;
            for x in ownned_pets.iter()
            {
                sum_level += x.level;
            }
            sum_level
        }
    };
    total_pet_level
}
pub fn get_cleansed_pets_levels(gs: &GameState, our_habitat: HabitatType, enemy_habitat_type: HabitatType) -> f32
{
    let total_level = get_total_level_owned_pets_by_habitat(gs, our_habitat);

    let modifier = match (our_habitat, enemy_habitat_type)
    {
        // Water strong vs Fire: +25%
        (HabitatType::Water, HabitatType::Fire) => 1.25,
        // Water weak vs Shadow: -20%
        (HabitatType::Water, HabitatType::Shadow) => 0.80,

        // Fire strong vs Earth: +25%
        (HabitatType::Fire, HabitatType::Earth) => 1.25,
        // Fire weak vs Water: -20%
        (HabitatType::Fire, HabitatType::Water) => 0.80,

        // Earth strong vs Light: +25%
        (HabitatType::Earth, HabitatType::Light) => 1.25,
        // Earth weak vs Fire: -20%
        (HabitatType::Earth, HabitatType::Fire) => 0.80,

        // Light strong vs Shadow: +25%
        (HabitatType::Light, HabitatType::Shadow) => 1.25,
        // Light weak vs Earth: -20%
        (HabitatType::Light, HabitatType::Earth) => 0.80,

        // Shadow strong vs Water: +25%
        (HabitatType::Shadow, HabitatType::Water) => 1.25,
        // Shadow weak vs Light: -20%
        (HabitatType::Shadow, HabitatType::Light) => 0.80,

        _ => 1.0,
    };

    (total_level as f32 * modifier)
}

pub fn get_pets_left_for_pet_arena(gs: &GameState) -> Vec<HabitatType>
{
    let mut pets_left: Vec<HabitatType> = Vec::new();

    let pet_habitats = match &gs.pets
    {
        None => return pets_left,
        Some(pets) => &pets.habitats,
    };

    if !pet_habitats[HabitatType::Light].battled_opponent
    {
        pets_left.push(HabitatType::Light);
    }
    if !pet_habitats[HabitatType::Water].battled_opponent
    {
        pets_left.push(HabitatType::Water);
    }
    if !pet_habitats[HabitatType::Shadow].battled_opponent
    {
        pets_left.push(HabitatType::Shadow);
    }
    if !pet_habitats[HabitatType::Earth].battled_opponent
    {
        pets_left.push(HabitatType::Earth);
    }
    if !pet_habitats[HabitatType::Fire].battled_opponent
    {
        pets_left.push(HabitatType::Fire);
    }

    pets_left
}

pub fn decide_next_pet_to_fight(gs: &GameState, enemys_daily_habitat: HabitatType, enemy_level: u16) -> Option<(HabitatType, u16)>
{
    let pets_left = get_pets_left_for_pet_arena(gs);

    if pets_left.is_empty()
    {
        return None;
    }

    let mut habitats_with_levels: Vec<(HabitatType, u16)> = pets_left
        .iter()
        .map(|&habitat| {
            let cleansed_level = get_cleansed_pets_levels(gs, habitat, enemys_daily_habitat) as u16;
            (habitat, cleansed_level)
        })
        .collect();

    let mut stronger_or_equal = vec![];
    let mut slightly_weaker = vec![];
    let mut much_weaker = vec![];

    for &(habitat, level) in &habitats_with_levels
    {
        if level >= enemy_level
        {
            stronger_or_equal.push((habitat, level));
        }
        else if level > (enemy_level as f32 * 0.9).ceil() as u16
        {
            slightly_weaker.push((habitat, level));
        }
        else
        {
            much_weaker.push((habitat, level));
        }
    }

    // if there are multiple much weaker, choose the second strongest
    if much_weaker.len() > 1
    {
        much_weaker.sort_by(|a, b| b.1.cmp(&a.1));
        return Some(much_weaker[1].clone());
    }

    // ff there's only one much weaker, choose it
    if much_weaker.len() == 1
    {
        return Some(much_weaker[0].clone());
    }

    // if no much weaker, fallback to slightly weaker or stronger or equal
    if let Some(best_option) = slightly_weaker.iter().max_by(|a, b| a.1.cmp(&b.1))
    {
        return Some(*best_option);
    }

    if let Some(best_option) = stronger_or_equal.iter().max_by(|a, b| a.1.cmp(&b.1))
    {
        return Some(*best_option);
    }

    None
}

// stärkerer gegner ist besser als unser schnitt oder exakt gleich gut
// leicht schwächerer gegner ist zwischen 0,1% und 9,99% schwächer als wir
// sehr viel schwächerer gegner ist über 10% schwächer als wir das 2. stärkste
// element, das noch besser ist als der gegner. wenn wir nur noch ein element
// haben, das besser ist als der gegner, dann nehmen wir das.
pub fn time_remaining<T: Borrow<DateTime<Local>>>(time: T) -> Duration { (*time.borrow() - Local::now()).to_std().unwrap_or_default() }

pub fn get_current_habitat_enemy(index: u32) -> u32
{
    let mut data: HashMap<u32, u32> = HashMap::new();
    data.insert(0, 15);
    data.insert(1, 37);
    data.insert(2, 72);
    data.insert(3, 125);
    data.insert(4, 196);
    data.insert(5, 296);
    data.insert(6, 419);
    data.insert(7, 567);
    data.insert(8, 854);
    data.insert(9, 1181);
    data.insert(10, 1571);
    data.insert(11, 2064);
    data.insert(12, 2908);
    data.insert(13, 3901);
    data.insert(14, 5053);
    data.insert(15, 6372);
    data.insert(16, 8741);
    data.insert(17, 12411);
    data.insert(18, 16614);
    data.insert(19, 18240);

    *data.get(&index).unwrap_or(&0)
}
