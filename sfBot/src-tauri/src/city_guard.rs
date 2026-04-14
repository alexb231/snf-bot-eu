#![allow(warnings)]

use std::{error::Error, time::Duration};

use chrono::{Local, NaiveTime};
use sf_api::{command::Command, gamestate::tavern::CurrentAction, SimpleSession};
use tokio::time::sleep;

use crate::{bot_runner::write_character_log, fetch_character_setting};

pub async fn sleep_between_commands(ms: u64) { sleep(Duration::from_millis(ms)).await; }

pub async fn city_guard(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let character_name = gs.character.name.clone();
    let character_id = gs.character.player_id;
    let enable_city_guard_from: String = fetch_character_setting(&gs, "tavernPlayCityGuardFrom").unwrap_or("00:00".to_string());
    let enable_city_guard_to: String = fetch_character_setting(&gs, "tavernPlayCityGuardTo").unwrap_or("00:00".to_string());
    let from_time = NaiveTime::parse_from_str(&enable_city_guard_from, "%H:%M").unwrap();
    let to_time = NaiveTime::parse_from_str(&enable_city_guard_to, "%H:%M").unwrap();
    let is_in_range = check_time_in_range(enable_city_guard_from, enable_city_guard_to);
    let beers_to_drink: i32 = std::cmp::min(fetch_character_setting(&gs, "tavernDrinkBeerAmount").unwrap_or(0), 12).max(0);
    let play_expeditions : bool = fetch_character_setting(&gs, "tavernPlayExpeditions").unwrap_or(true);
    let hours_of_work_at_once: i32 = fetch_character_setting(&gs, "tavernCityGuardTimeToPlay").unwrap_or(1);
    
    let hours_left = hours_until_to_time(to_time);

    let thirst_left = gs.tavern.thirst_for_adventure_sec;
    let no_thirst_left = thirst_left == 0;
    let max_beers = gs.tavern.beer_max;

    let beers_drunk = gs.tavern.beer_drunk;

    let target_beers = std::cmp::min(beers_to_drink as u8, max_beers);
    let beers_needed = target_beers.saturating_sub(beers_drunk);
    let not_enough_mushrooms_for_beers = gs.character.mushrooms < beers_needed as u32;
    let no_beer_left = beers_drunk >= target_beers;
    
    let nothing_left_todo = !play_expeditions || (no_thirst_left && (no_beer_left || not_enough_mushrooms_for_beers));
    
    
    
    match gs.tavern.current_action
    {
        CurrentAction::Idle =>
        {
            if (nothing_left_todo)
            {
                if (is_in_range)
                {
                    session.send_command(Command::StartWork { hours: hours_of_work_at_once as u8 }).await?;
                    write_character_log(
                        &character_name,
                        character_id,
                        &format!("CITY_GUARD: Started work for {} hours", hours_of_work_at_once),
                    );
                    return Ok(String::from(format!("Started work for {} hours", hours_of_work_at_once)));
                }
                else
                {
                    session.send_command(Command::StartWork { hours: hours_left as u8 }).await?;
                    write_character_log(
                        &character_name,
                        character_id,
                        &format!("CITY_GUARD: Started work for {} hours", hours_left),
                    );
                    return Ok(String::from(format!("Started work for {} hours", hours_left)));
                }
            }
        }
        CurrentAction::CityGuard { hours, busy_until } =>
        {
            if (busy_until < Local::now())
            {
                session.send_command(Command::FinishWork).await?;
                write_character_log(
                    &character_name,
                    character_id,
                    &format!(
                        "CITY_GUARD: Finished work, collected reward (scheduled until {})",
                        busy_until.format("%H:%M:%S")
                    ),
                );
                return Ok(String::from("Collected city guard reward"));
            }
            return Ok("".to_string());
        }
        CurrentAction::Quest { quest_idx, busy_until } =>
        {
            return Ok(String::from(""));
        }
        CurrentAction::Expedition =>
        {
            return Ok(String::from(""));
        }
        CurrentAction::Unknown(optional_time) =>
        {
            println!("city guard unknown action block finishing work");
            session.send_command(Command::FinishWork).await?;
            let msg = match optional_time
            {
                Some(time) => format!(
                    "CITY_GUARD: Finished work, collected reward (scheduled until {})",
                    time.format("%H:%M:%S")
                ),
                None => "CITY_GUARD: Finished work, collected reward".to_string(),
            };
            write_character_log(
                &character_name,
                character_id,
                &msg,
            );
            return Ok(String::from("Collected city guard reward"));
        }
    }
    return Ok("".to_string());
}

use chrono::TimeDelta;
use crate::utils::check_time_in_range;

pub fn hours_until_to_time(to_time: NaiveTime) -> i64
{
    let now = Local::now().time();

    let diff = if to_time >= now { to_time.signed_duration_since(now) } else { to_time.signed_duration_since(now) + TimeDelta::hours(24) };

    diff.num_hours().clamp(1, 10)
}
