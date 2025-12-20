#![allow(warnings)]

use std::{error::Error, fmt::Debug};

use chrono::{Duration, Local};
use sf_api::{command::Command, error::SFError, gamestate::social::OtherPlayer, PlayerId, SimpleSession};

use crate::fetch_character_setting;

pub async fn test123(name: &str, zahl: usize) -> Result<String, String>
{
    if zahl > 100
    {
        Err(format!("The number {} is too large!", zahl))
    }
    else
    {
        Ok(format!("Hello, {}! You've been greeted from Rust! {}", name, zahl))
    }
}

pub async fn arena_fight(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gamestate = &session.send_command(Command::CheckArena).await?.clone();
    let stop_after_exp_fights: bool = fetch_character_setting(&gamestate, "arenaStopWhenDone").unwrap_or(false);
    let arena = &gamestate.arena.clone();

    let max_exp_fights = 10;
    if stop_after_exp_fights && arena.fights_for_xp >= max_exp_fights
    {
        return Ok(String::from(""));
    }

    // check ob free fight rdy ist
    let current_time = Local::now();
    let current_time_minus_3 = current_time - Duration::minutes(3);

    if let Some(next_free_fight) = arena.next_free_fight
    {
        if next_free_fight >= current_time_minus_3
        {
            return Ok(String::from(""));
        }
    }

    // hol dir die 3 möglichen gegner, check wer das lowste lvl hat und fighte den
    let available_opponents_pid = arena.enemy_ids;
    if (available_opponents_pid.len() == 0)
    {
        return Ok(String::from(""));
    }
    if (available_opponents_pid[0] == 0)
    {
        return Ok(String::from(""));
    }

    let mut available_opponents: Vec<OtherPlayer> = Vec::new();

    for opponent_pid in available_opponents_pid
    {
        let result = convert_pid_to_opponent(session, opponent_pid).await;

        match result
        {
            Ok(opponent) =>
            {
                available_opponents.push(opponent);
            }
            Err(e) =>
            {
                eprintln!("Error occurred: {}", e);
                continue;
            }
        }
    }

    if (&available_opponents.len() == &0)
    {
        return Ok(String::from(""));
    }
    let mut lowest_target = &available_opponents[0];
    for opponent in available_opponents.iter()
    {
        if opponent.level < lowest_target.level
        {
            lowest_target = opponent;
        }
    }

    let opponent_name = lowest_target.name.clone();
    let fight_command = Command::Fight { name: opponent_name, use_mushroom: false };
    if let Err(err) = session.send_command(fight_command).await
    {
        return Ok(String::from("Error: func arena_fight while executing fight_command command"));
    }
    // können glaub nicht sagen ob gewonnen oder verloren

    Ok(String::from(""))
}

async fn convert_pid_to_opponent(session: &mut SimpleSession, opponent_pid: PlayerId) -> Result<OtherPlayer, Box<dyn Error>>
{
    // change return type to `OtherPlayer`
    let gs = match session.send_command(Command::ViewPlayer { ident: opponent_pid.to_string() }).await
    {
        Ok(gs) => gs,
        Err(e) =>
        {
            eprintln!("failed to get opponnent");
            return Err(Box::new(e));
        }
    };
    let lookup = &gs.lookup;

    if let Some(opponent) = lookup.lookup_pid(opponent_pid)
    {
        Ok(opponent.clone())
    }
    else
    {
        Err(Box::new(SFError::ParsingError("Could not get opponent", "".to_string())))
    }
}
