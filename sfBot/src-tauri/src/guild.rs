#![allow(warnings)]

use std::{collections::HashMap, error::Error, fmt::Debug};

use chrono::{Duration, Local, NaiveTime};
use sf_api::{
    command::Command,
    gamestate::{
        dungeons::Portal,
        guild::{BattlesJoined, Guild, GuildRank},
        social::OtherGuild,
        GameState,
    },
    SimpleSession,
};

use crate::fetch_character_setting;

pub async fn sign_up_for_guild_attack_and_defense(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    // raid count as attack
    let gs = session.send_command(Command::Update).await?.clone();
    let sign_up_atk: bool = fetch_character_setting(&gs, "quartersSignUpGuildAtks").unwrap_or(false);
    let signup_def: bool = fetch_character_setting(&gs, "quartersSignUpGuildDef").unwrap_or(false);
    if !(sign_up_atk && signup_def)
    {
        return Ok("".to_string());
    }
    let char_name = &gs.character.name;
    let mut result = "".to_string();
    let guild_option = &gs.guild.clone();
    if (char_name == "Berg de Highsen" && &gs.character.level == &421u16)
    {
        // println!("{:?}", guild_option);
        println!("{:?}", gs.guild.unwrap().defending);
    }

    if let Some(guild) = guild_option
    {
        if let Some(_) = guild.defending
        {
            if (signup_def)
            {
                let already_participating = check_participation(&guild, char_name, BattlesJoined::Defense);
                if !already_participating
                {
                    let _ = session.send_command(Command::GuildJoinDefense).await;
                    result += "\nSigned up for guild def";
                }
            }
        }
        else
        {
            // sometimes the server doesnt show that theres a defence available so we need
            // to double check
            if (check_member_signups(guild))
            {
                let already_participating = check_participation(&guild, char_name, BattlesJoined::Defense);
                if (!already_participating)
                {
                    let _ = session.send_command(Command::GuildJoinDefense).await;
                    result += "\nSigned up for guild def";
                }
            }
        }

        if let Some(_) = guild.attacking
        {
            if (sign_up_atk)
            {
                let already_participating = check_participation(&guild, char_name, BattlesJoined::Attack);
                if !already_participating
                {
                    let _ = session.send_command(Command::GuildJoinAttack).await;
                    result += "\nSigned up for guild attack";
                }
            }
        }
    }

    Ok(result)
}

pub async fn declare_guild_attack(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let mut msg = String::from("");
    let favourite_enemy_guilds: String = fetch_character_setting(&gs, "quartersOrderAtkFavouriteEnemies").unwrap_or("".to_string());
    let declare_first_attack_time: String = fetch_character_setting(&gs, "quartersOrderAtkFavouriteEnemiesTimeFirst").unwrap_or("".to_string());
    let declare_second_attack_time: String = fetch_character_setting(&gs, "quartersOrderAtkFavouriteEnemiesTimeSecond").unwrap_or("".to_string());
    let enable_declare_guild_attack: bool = fetch_character_setting(&gs, "quartersOrderAtk").unwrap_or(false);

    if (!enable_declare_guild_attack)
    {
        return Ok(msg);
    }

    if (favourite_enemy_guilds == "".to_string() || declare_first_attack_time == "".to_string() || declare_second_attack_time == "".to_string())
    {
        return Ok(msg);
    }
    let favourite_guild_names: Vec<&str> = favourite_enemy_guilds.split('/').map(|s| s.trim()).collect();

    let guild = match &gs.guild
    {
        Some(g) => g,
        None => return Ok(msg),
    };
    let next_guild_atk_bool = guild.next_attack_possible.is_some();

    if next_guild_atk_bool
    {
        return Ok(msg); // this is the case when there is currently an attack
                        // ongoing ?!
    }

    // TODO: read the the text below if there is a bug
    // if there is ever a bug report that guild attack declaration doesnt work or
    // causes lots of invalid sessions then its most likely due to
    // guild.next_attack_possible being none or some, couldnt figure out when this
    // is some and when its none but its some if currently an attack is already
    // scheduled

    let prepared_battle = &guild.attacking;
    if (prepared_battle.is_some())
    {
        return Ok(msg);
    }

    let mut other_guilds: HashMap<String, OtherGuild> = HashMap::new();
    if is_past_attack_time(&declare_first_attack_time) || is_past_attack_time(&declare_second_attack_time)
    {
        for guild_name in favourite_guild_names
        {
            let viewing_guild_state = session.send_command(Command::ViewGuild { guild_ident: guild_name.to_string() }).await?;
            let other_guilds_from_response = &viewing_guild_state.lookup.guilds;

            for (key, value) in other_guilds_from_response
            {
                other_guilds.insert(key.clone(), value.clone());
            }
        }
        for (key, value) in other_guilds
        {
            if (value.defends_against.is_none() && !next_guild_atk_bool)
            {
                session.send_command(Command::GuildAttack { guild: key.clone() }).await?;
                msg += &format!("Attacking guild: {}", key);
                return Ok(msg);
            }
        }
    }

    return Ok("".to_string());
}

pub fn check_member_signups(guild: &Guild) -> bool
{
    let guild_members = &guild.members;
    for member in guild_members.iter()
    {
        if member.battles_joined.is_some()
        {
            if (member.battles_joined.unwrap() == BattlesJoined::Defense || member.battles_joined == Some(BattlesJoined::Both))
            {
                return true;
            }
        }
    }
    false
}

pub fn check_participation(guild: &Guild, char_name: &String, checking_battle: BattlesJoined) -> bool
{
    let guild_member = &guild.members;
    for member in guild_member.iter()
    {
        if member.name == char_name.to_string()
        {
            let battles = match member.battles_joined
            {
                None => return false,
                Some(battles) => battles,
            };
            return battles == checking_battle || battles == BattlesJoined::Both;
        }
    }
    return false;
}

pub async fn fight_hydra(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let guild = match &gs.guild
    {
        None => return Ok("".to_string()),
        Some(g) => g,
    };

    let members = &guild.members;
    let member_count = members.len(); // needs to be atleast 10 otherwise no hydra is available
    let hydra = &guild.hydra;
    let mut leader_level_ok: bool = false;

    for member in members.iter()
    {
        if (member.guild_rank == GuildRank::Leader && member.level >= 150)
        {
            leader_level_ok = true;
        }
    }

    if (!leader_level_ok)
    {
        return Ok("".to_string());
    }

    if hydra.remaining_fights <= 0 || guild.pet_id == 0 || member_count < 10
    {
        return Ok("".to_string());
    }

    let hydra_command = Command::GuildPetBattle { use_mushroom: false };
    if let Err(err) = session.send_command(hydra_command).await
    {
        return Ok("".to_string());
    }
    else
    {
        return Ok("fought hydra".to_string());
    }
}

pub async fn fight_guild_portal(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let name = gs.character.name.clone();
    let guild = match &gs.guild
    {
        None => return Ok("".to_string()),
        Some(g) => g,
    };

    if (gs.character.level >= 99)
    {
        let should_do_portal = fight_guild_portal_logic_stuff(name.clone(), guild);
        if should_do_portal
        {
            session.send_command(Command::GuildPortalBattle).await?;
            return Ok("Fought Demon portal.".to_string());
        }
    }
    return Ok("".to_string());
}

pub async fn fight_demon_portal(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    let mut msg = String::from("");
    if (gs.character.level < 99)
    {
        return Ok(msg);
    }

    let option_demon_portal = &gs.dungeons.portal;
    let demon_portal = match option_demon_portal
    {
        Some(portal) => portal,
        None =>
        {
            return Ok(msg);
        }
    };
    if (demon_portal.finished >= 50)
    {
        return Ok(msg);
    }

    if (demon_portal.can_fight)
    {
        session.send_command(Command::FightPortal).await?;
    }

    return Ok("".to_string());
}

pub fn fight_guild_portal_logic_stuff(own_char_name: String, guild: &Guild) -> bool
{
    let defeat_count = *&guild.portal.defeated_count;
    // 50 means all enemys have been cleared
    if (defeat_count == 50)
    {
        return false;
    }

    let guild_members = &guild.members;
    for x in guild_members.iter()
    {
        if (x.name == own_char_name)
        {
            if let Some(last_fought_datee) = x.portal_fought
            {
                return last_fought_datee.date_naive() < Local::now().date_naive();
            }
        }
    }
    return false;
}

fn is_past_attack_time(attack_time: &str) -> bool
{
    if let Ok(attack_time) = NaiveTime::parse_from_str(attack_time, "%H:%M")
    {
        let now = Local::now().time();

        let end_time = attack_time + Duration::hours(2);

        return now >= attack_time && now <= end_time;
    }
    false
}
