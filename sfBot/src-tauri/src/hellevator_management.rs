use std::error::Error;

use sf_api::{
    command::Command,
    gamestate::unlockables::{HellevatorDailyReward, HellevatorRaidFloor, HellevatorStatus},
    SimpleSession,
};

use crate::fetch_character_setting;

pub async fn play_hellevator(session: &mut SimpleSession) -> Result<std::string::String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let mut ret = String::new();

    if gs.character.level < 10
    {
        return Ok(String::new());
    }

    let claim_final_reward: bool = fetch_character_setting(&gs, "quartersHellevatorClaimRewardFinal").unwrap_or(false);
    let claim_daily_reward: bool = fetch_character_setting(&gs, "quartersHellevatorClaimReward").unwrap_or(false);
    let helle_cards_to_keep: i32 = fetch_character_setting(&gs, "quartersHellevatorKeyCardsKeep").unwrap_or(0);
    let join_raid: bool = fetch_character_setting(&gs, "quartersHellevatorJoinRaid").unwrap_or(false);
    let raid_floor_setting: i64 = fetch_character_setting(&gs, "quartersHellevatorJoinRaidFloor").unwrap_or(0);

    let hellevator = match gs.hellevator.status()
    {
        HellevatorStatus::RewardClaimable =>
        {
            if claim_final_reward
            {
                session.send_command(Command::HellevatorClaimFinal).await?;
                return Ok("Final reward claimed".into());
            }
            return Ok(String::new());
        }
        HellevatorStatus::NotEntered =>
        {
            session.send_command(Command::HellevatorEnter).await?;
            return Ok("Entered Hellevator".into());
        }
        HellevatorStatus::NotAvailable =>
        {
            println!("Hellevator is not available currently");
            return Ok(ret);
        }
        HellevatorStatus::Active(h) => h,
    };

    let keycards_available = hellevator.key_cards;
    let mut can_reach_selected_raid_floor = false;

    if claim_daily_reward
    {
        if let Some(rewards) = &hellevator.rewards_yesterday
        {
            if rewards.claimable()
            {
                session.send_command(Command::HellevatorClaimDailyYesterday).await?;
            }
        }
        if let Some(rewards) = &hellevator.rewards_today
        {
            if rewards.claimable()
            {
                session.send_command(Command::HellevatorClaimDaily).await?;
            }
        }
    }

    let is_signed_up = hellevator.guild_raid_floors.iter().flat_map(|floor| &floor.today_assigned).any(|name| name == &gs.character.name);
    let mut can_join_raid_now = false;

    if join_raid && !is_signed_up && keycards_available >= 5
    {
        if raid_floor_setting < 1
        {
            ret.push_str("Hellevator: Hell Attack floor is invalid. ");
        }
        else
        {
            let floor_chosen = (raid_floor_setting - 1) as usize;
            if floor_chosen >= hellevator.guild_raid_floors.len()
            {
                ret.push_str("Hellevator: Hell Attack floor is out of range. ");
            }
            else
            {
                let required_floor = ((floor_chosen + 1) as u32) * 50;
                if hellevator.current_floor < required_floor
                {
                    ret.push_str(
                        &format!(
                            "Hellevator: Cannot join Hell Attack {} (requires floor {}, current floor {}). ",
                            floor_chosen + 1,
                            required_floor,
                            hellevator.current_floor
                        )
                    );
                }
                else
                {
                    can_reach_selected_raid_floor = true;
                    session.send_command(Command::HellevatorJoinHellAttack { plain: floor_chosen, use_mushroom: false }).await?;
                    can_join_raid_now = true;
                    ret.push_str("Hellevator: Joined Hell Attack.");
                }
            }
        }
    }

    let should_reserve_cards_for_raid = join_raid && !is_signed_up && !can_join_raid_now && can_reach_selected_raid_floor;
    let min_cards_to_keep = if should_reserve_cards_for_raid { helle_cards_to_keep.max(5) } else { helle_cards_to_keep };

    if keycards_available > min_cards_to_keep as u32
    {
        session.send_command(Command::HellevatorFight { use_mushroom: false }).await?;
        ret.push_str("Fought in Hellevator.");
    }

    Ok(ret)
}