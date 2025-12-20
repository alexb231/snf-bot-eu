#![allow(warnings)]

use std::time::Duration;

use chrono::Local;
use sf_api::{
    command::{Command, DiceType, DiceType::ReRoll, RollDicePrice, RollDicePrice::Free},
    gamestate::{
        dungeons::{DungeonProgress, LightDungeon},
        GameState,
    },
    SimpleSession,
};
use tokio::time::sleep;

use crate::{bot_runner::write_character_log, fetch_character_setting};

pub async fn sleep_between_commands(ms: u64) { sleep(Duration::from_millis(ms)).await; }

pub async fn play_dice(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let mut gamestate = session.send_command(Command::Update).await?.clone();
    let character_name = gamestate.character.name.clone();
    let character_id = gamestate.character.player_id;
    let dice_game = &gamestate.tavern.dice_game.clone();
    let skip_wait_time_settings: bool = fetch_character_setting(&gamestate, "tavernDiceGameSkipUsingHG").unwrap_or(false);
    let hourglas_count = &gamestate.tavern.quicksand_glasses;
    let skip_wait_time = *hourglas_count > 0 && skip_wait_time_settings;

    if (dice_game.next_free.is_none())
    {
        if (!checkIfDiceGameOpen(&mut gamestate))
        {
            return Ok(String::from(""));
        }
    }

    if (dice_game.remaining <= 0)
    {
        return Ok(String::from(""));
    }

    use chrono::{Duration, Local};

    let now = Local::now();
    let x = dice_game.next_free.unwrap_or_else(|| now - Duration::seconds(10));

    if (!skip_wait_time)
    {
        if now < x
        {
            return Ok(String::from(""));
        }
    }

    let mut did_play = false;

    // if(dice_game.next_free.unwrap() >= Local::now()) { return Ok(()); }
    if (gamestate.tavern.dice_game.current_dice.is_empty())
    {
        let start_round_one = start_round_one(session, skip_wait_time).await;
        match start_round_one
        {
            Ok(flag) =>
            {
                did_play = true; // we started round 1
                if (flag)
                {
                    // won in round 1
                    write_character_log(&character_name, character_id, "DICE: Won dice game in round 1");
                    return Ok(String::from("Won Dice game in round 1"));
                }
                else if !flag
                {
                    // not won -> go round 2
                    drop(start_round_one);
                    start_round_two(session).await;
                }
            }
            Err(e) =>
            {
                return Err(e);
            }
        }
    }
    else
    {
        // game in progress: finish it if no reward yet
        if gamestate.tavern.dice_game.reward.is_none()
        {
            did_play = true;
            start_round_two(session).await;
        }
    }

    if did_play
    {
        write_character_log(&character_name, character_id, "DICE: Played dice game");
        Ok("played dice game".to_string())
    }
    else
    {
        Ok(String::from(""))
    }
}

fn checkIfDiceGameOpen(gs: &mut GameState) -> bool
{
    let pets = gs.pets.clone();
    if (pets.is_none())
    {
        // println!("pets was none");
        return false;
    }
    let tower = gs.dungeons.light[LightDungeon::Tower];
    match tower
    {
        DungeonProgress::Locked =>
        {
            // println!("locked dungeon");
            return false;
        }
        DungeonProgress::Open { .. } => return true,
        DungeonProgress::Finished => return true,
    }
}

async fn start_round_one(session: &mut SimpleSession, skip_wait_time: bool) -> Result<bool, Box<dyn std::error::Error>>
{
    let dices: [DiceType; 5] = [ReRoll, ReRoll, ReRoll, ReRoll, ReRoll];

    let gamestate = if skip_wait_time
    {
        session.send_command(Command::RollDice { payment: RollDicePrice::Hourglass, dices }).await
    }
    else
    {
        session.send_command(Command::RollDice { payment: Free, dices }).await
    };

    let gamestate = match gamestate
    {
        Ok(state) => state,
        Err(e) =>
        {
            // eprintln!("Error sending roll dice command: {}", e);
            return Err(Box::new(e));
        }
    };

    let current_dice = gamestate.tavern.dice_game.current_dice.clone();

    if let Some(reward) = gamestate.tavern.dice_game.reward
    {
        // kann sein das wir hier schon gewonnen haben
        // println!("trying to claim in round 1");

        let prios = load_prios(gamestate);
        if prios.contains(&reward.win_typ)
        {
            let succ = match claim_reward(session, 1).await
            {
                true => true,
                false => false,
            };

            Ok(succ)
        }
        else
        {
            return Ok(false);
        }
    }
    else
    {
        Ok(false)
    }
}

async fn claim_reward(session: &mut SimpleSession, round: usize) -> bool
{
    let gs = match session.send_command(Command::Update).await
    {
        Ok(state) => state,
        Err(e) =>
        {
            // eprintln!("Error updating game state: {}", e);
            return false; // exit the function on error
        }
    };
    if (round == 2)
    {
        return true;
    }
    let current_dice = gs.tavern.dice_game.current_dice.clone();

    let current_reward = match gs.tavern.dice_game.reward
    {
        Some(reward) => reward.win_typ.clone(),
        None =>
        {
            // eprintln!("No reward found");
            return false; // Exit the function if no reward
        }
    };

    let mut claim_array: [String; 6] = ["0".to_string(), "0".to_string(), "0".to_string(), "0".to_string(), "0".to_string(), "0".to_string()];
    let mut index = 1;

    for reward in current_dice
    {
        if reward == current_reward
        {
            if reward == DiceType::Silver
            {
                claim_array[index] = "1".to_string();
            }
            if reward == DiceType::Stone
            {
                claim_array[index] = "2".to_string();
            }
            if reward == DiceType::Wood
            {
                claim_array[index] = "3".to_string();
            }
            if reward == DiceType::Souls
            {
                claim_array[index] = "4".to_string();
            }
            if reward == DiceType::Arcane
            {
                claim_array[index] = "5".to_string();
            }
            if reward == DiceType::Hourglass
            {
                claim_array[index] = "6".to_string();
            }
        }
        index += 1;
    }
    // ReRoll = 0,
    // Silver = 1 ,
    // Stone = 2,
    // Wood = 3,
    // Souls= 4,
    // Arcane = 5,
    // Hourglass= 6,
    // println!("---------- i claimed :   {:?}", claim_array);
    let claim_reward_cmd = String::from("RollDice");
    match session
        .send_command(Command::Custom {
            cmd_name: claim_reward_cmd,
            arguments: claim_array.to_vec(),
        })
        .await
    {
        Ok(_) =>
        {
            // println!("i claimed stuff");
            return true;
        }
        Err(_) =>
        {
            // println!("was not ok");
            return false;
        }
    };
}

async fn start_round_two(session: &mut SimpleSession) -> bool
{
    // println!("Starting round two");
    let gamestate = match session.send_command(Command::Update).await
    {
        Ok(state) => state,
        Err(e) =>
        {
            // eprintln!("Error updating game state: {}", e);
            return false;
        }
    };

    let mut preference_priority: Vec<DiceType> = load_prios(gamestate);
    // println!("unsere prios sind: {:?}", preference_priority);
    let dices = gamestate.tavern.dice_game.current_dice.clone();

    let mut silver = DiceCounter { typ: DiceType::Silver, count: 0 };
    let mut stone = DiceCounter { typ: DiceType::Stone, count: 0 };
    let mut wood = DiceCounter { typ: DiceType::Wood, count: 0 };
    let mut souls = DiceCounter { typ: DiceType::Souls, count: 0 };
    let mut arcane = DiceCounter { typ: DiceType::Arcane, count: 0 };
    let mut hourglass = DiceCounter { typ: DiceType::Hourglass, count: 0 };

    let mut onTable = Vec::new();
    onTable.push(silver);
    onTable.push(stone);
    onTable.push(wood);
    onTable.push(souls);
    onTable.push(arcane);
    onTable.push(hourglass);

    // Zähl die occurences
    for dice in dices.clone()
    {
        if dice == silver.typ
        {
            silver.count += 1;
        }
        if dice == stone.typ
        {
            stone.count += 1;
        }
        if dice == wood.typ
        {
            wood.count += 1;
        }
        if dice == souls.typ
        {
            souls.count += 1;
        }
        if dice == arcane.typ
        {
            arcane.count += 1;
        }
        if dice == hourglass.typ
        {
            hourglass.count += 1;
        }
    }

    let mut dice_to_keep = ReRoll;
    let mut highestCount = 0;
    for (i, diceCounter) in onTable.iter().enumerate()
    {
        if (preference_priority.contains(&diceCounter.typ))
        {
            if (diceCounter.count >= highestCount)
            {
                highestCount = diceCounter.count;
                dice_to_keep = diceCounter.typ;
            }
        }
    }

    if (dice_to_keep == ReRoll)
    {
        // war wohl nix drin was wir wollen, also reroll komplett neu
        let mut new_dices: [DiceType; 5] = [ReRoll, ReRoll, ReRoll, ReRoll, ReRoll];
        let gamestate_after = match session.send_command(Command::RollDice { payment: Free, dices: new_dices }).await
        {
            Ok(state) => state,
            Err(e) =>
            {
                //     eprintln!("error rolling dice: {}", e);
                return false;
            }
        };

        if let Some(reward) = gamestate_after.tavern.dice_game.reward
        {
            // println!("trying to calim in round2");
            let succ = match claim_reward(session, 2).await
            {
                true => true,
                false => false,
            };
            return succ;
        }
        else
        {
            return false;
        }
    }
    else
    {
        // jetzt geh über die dices die der server zurück gegeben hat und ersetz alles
        // mit rerol auser den type den wir aufheben
        let mut new_dices: [DiceType; 5] = [ReRoll, ReRoll, ReRoll, ReRoll, ReRoll];
        for i in 0..dices.clone().len()
        {
            if (dices[i] == dice_to_keep)
            {
                new_dices[i] = dice_to_keep; // ansonsten lass es Reroll bleiben
            }
        }

        let gamestate_after = match session.send_command(Command::RollDice { payment: Free, dices: new_dices }).await
        {
            Ok(state) => state,
            Err(e) =>
            {
                // eprintln!("Error rolling dice: {}", e);
                return false;
            }
        };

        // schauen ob wir gewonnen haben
        if gamestate_after.tavern.dice_game.reward.is_some()
        {
            // println!("trying to calim in round2.2");
            let succ = match claim_reward(session, 2).await
            {
                true => true,
                false => false,
            };
            return succ;
        }
        else
        {
            return false;
        }
    }
}

#[derive(Clone, Copy)]
struct DiceCounter
{
    typ: DiceType,
    count: usize,
}

fn load_prios(gs: &GameState) -> Vec<DiceType>
{
    let mut default_prios: Vec<DiceType> = Vec::new();
    default_prios.push(DiceType::Silver);
    default_prios.push(DiceType::Hourglass);
    default_prios.push(ReRoll);
    default_prios.push(DiceType::Souls);
    default_prios.push(DiceType::Arcane);
    default_prios.push(DiceType::Wood);
    default_prios.push(DiceType::Stone);

    let priosStr = fetch_character_setting(&gs, "tavernDiceGameRewardOrder").unwrap_or(Vec::new());
    // println!("prioStr: {:?}", priosStr);
    let mut prios = Vec::new();
    for str in priosStr
    {
        prios.push(match_preferred_dice(str.as_str()));
    }
    if (prios.len() <= 0)
    {
        // println!("nehme default");
        prios = default_prios;
    }

    let mut prios_parsed = Vec::new();

    // als nächstes werden die strings aus dem ui auf DiceType enums gemacht damit
    // die benutzt werden können
    let mut rerollFound = false;
    for prio in prios
    {
        if (rerollFound)
        {
            prios_parsed.push(ReRoll);
        }
        else
        {
            if (prio == ReRoll)
            {
                rerollFound = true;
            }
            prios_parsed.push(prio);
        }
    }
    // println!("priosParsed: {:?}", prios_parsed);
    let mut preference_priority: Vec<DiceType> = Vec::new();

    // Check and push non-ReRoll dice into the priority vector
    if prios_parsed[0] != DiceType::ReRoll
    {
        preference_priority.push(prios_parsed[0]);
    }
    if prios_parsed[1] != DiceType::ReRoll
    {
        preference_priority.push(prios_parsed[1]);
    }
    if prios_parsed[2] != DiceType::ReRoll
    {
        preference_priority.push(prios_parsed[2]);
    }
    if prios_parsed[3] != DiceType::ReRoll
    {
        preference_priority.push(prios_parsed[3]);
    }
    if prios_parsed[4] != DiceType::ReRoll
    {
        preference_priority.push(prios_parsed[4]);
    }
    if prios_parsed[5] != DiceType::ReRoll
    {
        preference_priority.push(prios_parsed[5]);
    }
    // println!("preference_priority {:?}", preference_priority);
    return preference_priority;
}

fn match_preferred_dice(preferred_dice: &str) -> DiceType
{
    match preferred_dice
    {
        "Reroll" => DiceType::ReRoll,
        "Gold" => DiceType::Silver,
        "Stone" => DiceType::Stone,
        "Wood" => DiceType::Wood,
        "Souls" => DiceType::Souls,
        "Arcane Splinter" => DiceType::Arcane,
        "HourGlass" => DiceType::Hourglass,
        _ => ReRoll,
    }
}
