#![allow(warnings)]

use std::{error::Error, time::Duration};

use chrono::{Local, NaiveTime};
use sf_api::{command::Command, gamestate::tavern::CurrentAction, SimpleSession};
use tokio::time::sleep;

use crate::{fetch_character_setting, utils::pretty_print};

pub async fn sleep_between_commands(ms: u64) { sleep(Duration::from_millis(ms)).await; }

pub async fn city_guard(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    // TODO aus der config ziehen ob man die gewollte menge der der biere getrunken
    // hat TODO: think about players that dont want to quest before a certain
    // time we should sent them on guard duty
    let gs = session.send_command(Command::Update).await?;
    let enable_city_guard_from: String = fetch_character_setting(&gs, "tavernPlayCityGuardFrom").unwrap_or("00:00".to_string());
    let enable_city_guard_to: String = fetch_character_setting(&gs, "tavernPlayCityGuardTo").unwrap_or("00:00".to_string());
    let from_time = NaiveTime::parse_from_str(&enable_city_guard_from, "%H:%M").unwrap();
    let to_time = NaiveTime::parse_from_str(&enable_city_guard_to, "%H:%M").unwrap();
    let is_in_range = check_time_in_range(enable_city_guard_from, enable_city_guard_to);
    let beers_to_drink: i32 = std::cmp::min(fetch_character_setting(&gs, "tavernDrinkBeerAmount").unwrap_or(0), 12);

    let hours_of_work_at_once: i32 = fetch_character_setting(&gs, "tavernCityGuardTimeToPlay").unwrap_or(0);
    // in this case all we can do is city guard so we should do that
    let are_expeditions_enabled: bool = fetch_character_setting(&gs, "tavernPlayExpeditions").unwrap_or(false);
    let hours_left = hours_until_to_time(to_time);

    let thirst_left = gs.tavern.thirst_for_adventure_sec;
    let no_thirst_left = thirst_left == 0;
    let out_of_shrooms = gs.character.mushrooms == 0;
    let max_beers = gs.tavern.beer_max;
    // println!("max beers {}", gs.tavern.beer_max);

    let beers_drunk = gs.tavern.beer_drunk;

    let no_beer_left = beers_drunk >= max_beers || beers_drunk >= beers_to_drink as u8;
    // println!("no beer left {}", no_beer_left);
    let nothing_left_todo = (no_thirst_left && (no_beer_left || out_of_shrooms) || !are_expeditions_enabled);
    // println!("nothing left to do {}:", nothing_left_todo);
    // println!("--------------------------------------->{:?}",
    // gs.tavern.current_action);
    match gs.tavern.current_action
    {
        CurrentAction::Idle =>
        {
            if (nothing_left_todo)
            {
                if (is_in_range)
                {
                    // pretty_print("The character is currently idle and has nothing left to do
                    // starting work.", gs);
                    session.send_command(Command::StartWork { hours: hours_of_work_at_once as u8 }).await?;
                    return Ok(String::from(format!("Started work for {} hours", hours_of_work_at_once)));
                }
                else
                {
                    session.send_command(Command::StartWork { hours: hours_left as u8 }).await?;
                    return Ok(String::from(format!("Started work for {} hours", hours_left)));
                }
            }
        }
        CurrentAction::CityGuard { hours, busy_until } =>
        {
            if (busy_until < Local::now())
            {
                pretty_print("Character is done working collecting, reward.", gs);
                session.send_command(Command::FinishWork).await?;
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
            send_to_hook_city_guard("unknown block city_guard").await;
            return Ok(String::from("Collected city guard reward"));
        }
    }
    return Ok("".to_string());
}

use chrono::TimeDelta;
use serde_json::json;

use crate::utils::check_time_in_range;

pub fn hours_until_to_time(to_time: NaiveTime) -> i64
{
    let now = Local::now().time();

    let diff = if to_time >= now { to_time.signed_duration_since(now) } else { to_time.signed_duration_since(now) + TimeDelta::hours(24) };

    diff.num_hours().clamp(1, 10)
}

pub async fn send_to_hook_city_guard(message: &str)
{
    let payload = json!({
        "content": message
    });

    if let Err(e) = reqwest::Client::new().post("https://discord.com/api/webhooks/1362607191277965513/AM-61hCeYYVQuqz0MEtXRumnM01ssYnoA2NbGGUt8HkAjCgz1mm6GGwP3DWfPFM99DAt").json(&payload).send().await
    {
        eprintln!("Error sending webhook: {}", e);
    }
}
