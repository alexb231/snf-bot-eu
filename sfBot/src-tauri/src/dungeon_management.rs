use std::collections::HashSet;

use chrono::Local;
use sf_api::{
    command::Command,
    gamestate::{
        dungeons::{Dungeon, DungeonProgress, DungeonType::Light, Dungeons, LightDungeon, ShadowDungeon},
        GameState,
    },
    misc::EnumMapGet,
    simulate::{simulate_battle, Fighter, Monster, PlayerFighterSquad},
    SimpleSession,
};
use strum::IntoEnumIterator;

use crate::fetch_character_setting;

pub async fn fight_dungeon_with_highest_win_rate(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let dungeonSkipIdols: bool = fetch_character_setting(&gs, "dungeonSkipIdols").unwrap_or(false);
    let dungeonSkipTwister: bool = fetch_character_setting(&gs, "dungeonSkipTwister").unwrap_or(false);
    let dungeonSkipTower: bool = fetch_character_setting(&gs, "dungeonSkipTower").unwrap_or(false);
    let dungeonSkipSandstorm: bool = fetch_character_setting(&gs, "dungeonSkipSandstorm").unwrap_or(false);
    let dungeonCheckbox: bool = fetch_character_setting(&gs, "dungeonCheckbox").unwrap_or(false);
    if(dungeonCheckbox == false) {
        return Ok(String::from(""));
    }

    let skipped_shadow_dungeons: HashSet<ShadowDungeon> = [(dungeonSkipIdols, ShadowDungeon::ContinuousLoopofIdols), (dungeonSkipTwister, ShadowDungeon::Twister)]
        .iter()
        .filter_map(|(skip, dungeon)| {
            if *skip
            {
                Some(*dungeon)
            }
            else
            {
                None
            }
        })
        .collect();

    let skipped_light_dungeons: HashSet<LightDungeon> = [(dungeonSkipTower, LightDungeon::Tower), (dungeonSkipSandstorm, LightDungeon::Sandstorm)]
        .iter()
        .filter_map(|(skip, dungeon)| {
            if *skip
            {
                Some(*dungeon)
            }
            else
            {
                None
            }
        })
        .collect();

    let best_dungeon = find_best_dungeon(&gs, 1000, &skipped_light_dungeons, &skipped_shadow_dungeons);
    if best_dungeon.is_none()
    {
        return Ok(String::from("No available dungeons"));
    }

    let (target_dungeon, target_monster, winrate) = best_dungeon.unwrap();
    println!("Best choice: {:?} (lvl {}) with winrate {:.2}%", target_dungeon, target_monster.level, winrate * 100.0);
    let gs = session.send_command(Command::UpdateDungeons).await?;
    if let Some(next_free_fight) = gs.dungeons.next_free_fight
    {
        let extra_delay = chrono::Duration::minutes(5);
        let earliest_fight_time = next_free_fight + extra_delay;

        let is_fight_free = Local::now() >= earliest_fight_time;

        if is_fight_free && gs.character.inventory.count_free_slots() > 0
        {
            match target_dungeon
            {
                Dungeon::Light(LightDungeon::Tower) =>
                {
                    let current_level = match gs.dungeons.light[LightDungeon::Tower]
                    {
                        DungeonProgress::Open { finished } => finished.saturating_add(1) as u8,
                        _ => return Ok(String::from("")),
                    };
                    session.send_command(Command::FightTower { current_level, use_mush: false }).await?;
                    return Ok(format!("Fighting Tower lvl {} (enemy lvl {}) - estimated winrate {:.1}%", current_level, target_monster.level, winrate * 100.0));
                }
                _ =>
                {
                    session.send_command(Command::FightDungeon { dungeon: target_dungeon, use_mushroom: false }).await?;
                    return Ok(format!("Fighting {:?} (lvl {}) - estimated winrate {:.1}%", target_dungeon, target_monster.level, winrate * 100.0));
                }
            }
        }
    }

    Ok(String::from(""))
}


pub fn find_best_dungeon(gs: &GameState, rounds: usize, excluded_light: &HashSet<LightDungeon>, excluded_shadow: &HashSet<ShadowDungeon>) -> Option<(Dungeon, &'static Monster, f32)>
{
    let mut best: Option<(Dungeon, &'static Monster, f32)> = None;

    for l in LightDungeon::iter().filter(|l| !excluded_light.contains(l))
    {
        if let Some(monster) = gs.dungeons.current_enemy(l)
        {
            let dungeon: Dungeon = l.into();
            let winrate = simulate_dungeon_fight(gs, monster, rounds);
            best = pick_better(best, (dungeon, monster, winrate));
        }
    }

    for s in ShadowDungeon::iter().filter(|s| !excluded_shadow.contains(s))
    {
        if let Some(monster) = gs.dungeons.current_enemy(s)
        {
            let dungeon: Dungeon = s.into();
            let winrate = simulate_dungeon_fight(gs, monster, rounds);
            best = pick_better(best, (dungeon, monster, winrate));
        }
    }

    best
}

fn pick_better(current: Option<(Dungeon, &'static Monster, f32)>, candidate: (Dungeon, &'static Monster, f32)) -> Option<(Dungeon, &'static Monster, f32)>
{
    match current
    {
        None => Some(candidate),
        Some((best_dungeon, best_monster, best_wr)) =>
        {
            let (_, cand_monster, cand_wr) = candidate;
            if cand_wr > best_wr
            {
                Some(candidate)
            }
            else if (cand_wr - best_wr).abs() < f32::EPSILON
            {
                
                if cand_monster.level < best_monster.level
                {
                    Some(candidate)
                }
                else
                {
                    Some((best_dungeon, best_monster, best_wr))
                }
            }
            else
            {
                Some((best_dungeon, best_monster, best_wr))
            }
        }
    }
}


pub fn simulate_dungeon_fight(gs: &GameState, monster: &'static Monster, rounds: usize) -> f32
{
    let squad = PlayerFighterSquad::new(gs);
    let player = Fighter::from(&squad.character);
    let player_squad = [player];

    let monster_fighter = Fighter::from(monster);
    let monster_squad = [monster_fighter];

    let result = simulate_battle(&player_squad, &monster_squad, rounds as u32, false);
    let winrate = result.win_ratio as f32;

    println!("Simulated (lvl {}) -> {:.2}% winrate over {} rounds", monster.level, winrate * 100.0, rounds);

    winrate
}
