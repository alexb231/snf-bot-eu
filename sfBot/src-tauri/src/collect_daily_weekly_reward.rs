#![allow(warnings)]

use std::error::Error;

use sf_api::{command::Command, SimpleSession};

pub async fn collect_daily_and_weekly_rewards(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = &session.send_command(Command::Update).await?.clone();
    let enable_daily_reward_collection: bool = fetch_character_setting(&gs, "miscCollectDailyRewards").unwrap_or(false);
    let mut result = String::from("");
    if (enable_daily_reward_collection)
    {
        result += &*collect_daily_rewards(session).await?;
    }

    let enable_weekly_reward_collection: bool = fetch_character_setting(&gs, "miscCollectWeeklyRewards").unwrap_or(false);
    if (enable_weekly_reward_collection)
    {
        result += &*collect_weekly_rewards(session).await?;
    }

    let enable_daily_calendar_collection: bool = fetch_character_setting(&gs, "miscCollectCalendar").unwrap_or(false);
    let collect_only_exp_calendar: bool = fetch_character_setting(&gs, "miscCollectCalendarExpOnly").unwrap_or(false);
    let consider_mushroom_calendar: bool = fetch_character_setting(&gs, "miscCollectCalendarMushroomsCalendar").unwrap_or(false);
    let miscDontCollectCalendarBeforeStr: String = fetch_character_setting(&gs, "miscDontCollectCalendarBefore").unwrap_or("00:15".to_string());
    let miscDontCollectCalendarBeforeTime = NaiveTime::parse_from_str(&miscDontCollectCalendarBeforeStr, "%H:%M").unwrap();
    let charname = gs.character.name.clone();
    let calendar_rewards = &gs.specials.calendar.rewards;

    let is_in_range = Local::now().time() > miscDontCollectCalendarBeforeTime;
    if (is_in_range)
    {
        if (enable_daily_calendar_collection && !collect_only_exp_calendar)
        {
            result += &*collect_daily_calendar(session).await?;
        }
        if (collect_only_exp_calendar)
        {
            result += &*collect_daily_calendar_exp_only(session, consider_mushroom_calendar).await?;
        }
    }

    if gs.specials.advent_calendar.is_some()
    {
        collect_advents_calendar(session).await?;
        result += "Collected advent calendar.";
    }
    Ok(result)
}

use chrono::{Local, NaiveTime};
use sf_api::gamestate::rewards::CalendarRewardType;

use crate::fetch_character_setting;

async fn collect_daily_calendar_exp_only(session: &mut SimpleSession, consider_mushroom_calendar: bool) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let calendar_rewards = &gs.specials.calendar.rewards;
    let calendar_next = &gs.specials.calendar.next_possible;

    let mut exp_count = 0;
    let mut mushroom_count = 0;
    let mut is_exp_calendar = false;
    let mut is_mushroom_calendar = false;

    for reward in calendar_rewards
    {
        if (reward.typ == CalendarRewardType::Experience)
        {
            exp_count += 1;
        }

        if (reward.typ == CalendarRewardType::Mushrooms)
        {
            mushroom_count += 1;
        }
    }

    is_exp_calendar = exp_count >= 2;
    is_mushroom_calendar = mushroom_count >= 3;

    // println!("is_exp_calendar: {}, is_mushroom_calendar: {}, consider_mushroom_calendar: {}", is_exp_calendar, is_mushroom_calendar, consider_mushroom_calendar);

    if (gs.character.inventory.count_free_slots() <= 0)
    {
        return Ok(String::from(""));
    }

    if !(is_exp_calendar || (is_mushroom_calendar && consider_mushroom_calendar))
    {
        // println!(" Skip collection: calendar does not meet criteria.");
        return Ok(String::from(""));
    }

    if (is_exp_calendar || (is_mushroom_calendar && consider_mushroom_calendar))
    {
        if let Some(unlock_time) = calendar_next
        {
            if *unlock_time <= Local::now()
            {
                let collect_command = Command::CollectCalendar;
                if let Err(err) = session.send_command(collect_command).await
                {
                    eprintln!("Error: func collect_daily_calendar while executing CollectCalendar command: {}", err);
                    return Ok(String::from("Collected daily calendar."));
                }
            }
        }
    }
    return Ok(String::from(""));
}

async fn collect_daily_calendar(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let calendar = &gs.specials.calendar.next_possible;

    if (gs.character.inventory.count_free_slots() <= 0)
    {
        return Ok(String::from(""));
    }

    if let Some(unlock_time) = calendar
    {
        if *unlock_time <= Local::now()
        {
            let collect_command = Command::CollectCalendar;
            if let Err(err) = session.send_command(collect_command).await
            {
                eprintln!("Error: func collect_daily_calendar while executing CollectCalendar command: {}", err);
                return Ok(String::from("Collected daily calendar."));
            }
        }
    }
    return Ok(String::from(""));
}

async fn collect_daily_rewards(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let daily_tasks = &gs.specials.tasks.daily;
    let total_points = daily_tasks.earned_points();
    let position_to_claim = match total_points
    {
        13.. => Some(2),
        10..=12 => Some(1),
        5..=9 => Some(0),
        _ => None,
    };

    if let Some(pos) = position_to_claim
    {
        if (daily_tasks.can_open_chest(pos))
        {
            let command = Command::CollectDailyQuestReward { pos };
            session.send_command(command).await?;
            return Ok(format!("Collecting daily rewards at position {}", pos + 1));
        }
    }
    Ok(String::from(""))
}

async fn collect_weekly_rewards(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = &session.send_command(Command::Update).await?;
    let weekly_tasks = &gs.specials.tasks.event;

    let total_points = weekly_tasks.earned_points();

    let position_to_claim = match total_points
    {
        3 => Some(2),
        2 => Some(1),
        1 => Some(0),
        _ => None,
    };

    if let Some(pos) = position_to_claim
    {
        if (weekly_tasks.can_open_chest(pos))
        {
            let command = Command::CollectEventTaskReward { pos };
            session.send_command(command).await?;
            return Ok(format!("Collecting weekly rewards at position {}", pos + 1));
        }
    }
    Ok(String::from(""))
}

async fn collect_advents_calendar(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let advents_calender_command = Command::Custom {
        cmd_name: "AdventsCalendarClaimReward".to_string(),
        arguments: vec![],
    };

    if let Err(err) = session.send_command(advents_calender_command).await
    {
        eprintln!("Error: func collect_advents_calendar while executing advents_calender_command command: {}", err);
        return Ok(());
    }
    Ok(())
}
