#![allow(warnings)]

use std::{fmt::Debug, time::Duration};

use sf_api::{
    command::{Command, FortunePayment},
    error::SFError,
    gamestate::rewards::Event,
    SimpleSession,
};
use tokio::time::sleep;

use crate::fetch_character_setting;

pub async fn sleep_between_spins(ms: u64) { sleep(Duration::from_millis(ms)).await; }

pub async fn spin_lucky_wheel(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let mut wheel_spins: i32 = fetch_character_setting(&gs, "quartersSpinLuckyWithResourcesAmount").unwrap_or(0);
    let resource_for_spinning: String = fetch_character_setting(&gs, "quartersSpinLuckyWithResources").unwrap_or("".to_string());
    let free_slots = &gs.character.inventory.count_free_slots();
    if (free_slots <= &0)
    {
        return Ok(String::from(""));
    }

    let spin_wheel = true;
    let _spend_mushrooms = true;
    let events = &gs.specials.events.active;
    let max_spins = if events.contains(&Event::LuckyDay) { 40 } else { 20 };
    if (wheel_spins > max_spins)
    {
        wheel_spins = max_spins;
    }
    if session.send_command(Command::Update).await?.specials.wheel.clone().spins_today >= wheel_spins as u8
    {
        return Ok(String::from(""));
    }
    if spin_wheel
    {
        while session.send_command(Command::Update).await?.specials.wheel.clone().spins_today <= wheel_spins as u8
        {
            let new_gs = session.send_command(Command::Update).await?.clone();
            if (new_gs.character.inventory.free_slot().is_none())
            {
                break;
            }

            if new_gs.specials.wheel.clone().spins_today < 1
            {
                let _result = free_spin(session).await;
                return Ok(String::from("Performed ree lucky wheel spin"));
            }
            else
            {
                // TODO correct check needs to be implemented here not only lucky coins
                if session.send_command(Command::Update).await?.specials.wheel.clone().lucky_coins >= 10 || (get_resource_from_setting(resource_for_spinning.as_str()) == FortunePayment::Mushrooms && new_gs.character.mushrooms > 0)
                {
                    match session
                        .send_command(Command::SpinWheelOfFortune {
                            payment: get_resource_from_setting(resource_for_spinning.as_str()),
                        })
                        .await
                    {
                        Ok(_) =>
                        {}
                        Err(SFError::ServerError(msg)) if msg == "need a free slot" =>
                        {
                            return Ok(String::from(""));
                        }
                        Err(SFError::ServerError(msg)) if msg == "pet wheel reward type" => continue,
                        Err(SFError::ServerError(msg)) if msg == "no more turns today" =>
                        {
                            return Ok(String::from(""));
                        }
                        Err(e) =>
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
        }
    }
    else
    {
        let new_gs = session.send_command(Command::Update).await?;
        if (new_gs.character.inventory.free_slot().is_none())
        {
            return Ok(String::from(""));
        }

        if new_gs.specials.wheel.clone().spins_today < 1
        {
            let _ = free_spin(session).await;
            return Ok(String::from(""));
        }
    }
    Ok(String::from(""))
}

pub async fn free_spin(session: &mut SimpleSession) -> Result<(), Box<dyn std::error::Error>>
{
    session.send_command(Command::SpinWheelOfFortune { payment: FortunePayment::FreeTurn }).await?;
    return Ok(());
}

pub fn get_resource_from_setting(mount_to_buy: &str) -> FortunePayment
{
    match mount_to_buy
    {
        "spinLuckyWheelMushrooms" => FortunePayment::Mushrooms,
        "spinLuckyWheelCoins" => FortunePayment::LuckyCoins,
        _ => FortunePayment::LuckyCoins,
    }
}
