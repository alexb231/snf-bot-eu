use sf_api::{command::Command, gamestate::character::Mount, SimpleSession};

use crate::{bot_runner::write_character_log, fetch_character_setting};

pub async fn buy_mount(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let character_name = gs.character.name.clone();
    let character_id = gs.character.player_id;
    let character_mount = gs.character.mount;
    let current_shrooms = gs.character.mushrooms;
    let current_silver = gs.character.silver;
    let mount_to_buy: String = fetch_character_setting(&gs, "characterMount").unwrap_or("".to_string());
    if (mount_to_buy == "buyBestMountPossible")
    {
        if get_mount_value(character_mount) == 0
        {
            if current_shrooms > 25
            {
                println!("bought dragon");
                if let Err(e) = session.send_command(Command::BuyMount { mount: Mount::Dragon }).await
                {
                    eprintln!("{}", format!("Failed to buy Dragon mount: {:?}", e));
                }
                else
                {
                    write_character_log(
                        &character_name,
                        character_id,
                        &format!("MOUNT: Bought Dragon ({})", mount_cost_string(Mount::Dragon)),
                    );
                }
                return Ok(String::from("Bought mount: Dragon"));
            }

            if current_shrooms > 0 && current_silver >= 1000
            {
                println!("bought tiger");
                if let Err(e) = session.send_command(Command::BuyMount { mount: Mount::Tiger }).await
                {
                    eprintln!("{}", format!("Failed to buy Tiger mount: {:?}", e));
                }
                else
                {
                    write_character_log(
                        &character_name,
                        character_id,
                        &format!("MOUNT: Bought Tiger ({})", mount_cost_string(Mount::Tiger)),
                    );
                }
                return Ok(String::from("Bought mount: Tiger"));
            }

            if current_shrooms == 0 && current_silver >= 500
            {
                println!("bought horse");
                if let Err(e) = session.send_command(Command::BuyMount { mount: Mount::Horse }).await
                {
                    eprintln!("{}", format!("Failed to buy Horse mount: {:?}", e));
                }
                else
                {
                    write_character_log(
                        &character_name,
                        character_id,
                        &format!("MOUNT: Bought Horse ({})", mount_cost_string(Mount::Horse)),
                    );
                }
                return Ok(String::from("Bought mount: Horse"));
            }
            if current_silver >= 100
            {
                println!("bought cow");
                if let Err(e) = session.send_command(Command::BuyMount { mount: Mount::Cow }).await
                {
                    eprintln!("{}", format!("Failed to buy Cow mount: {:?}", e));
                }
                else
                {
                    write_character_log(
                        &character_name,
                        character_id,
                        &format!("MOUNT: Bought Cow ({})", mount_cost_string(Mount::Cow)),
                    );
                }
                return Ok(String::from("Bought mount: Cow"));
            }
        }
        return Ok(String::from(""));
    }
    else
    {
        if get_mount_value(character_mount) == 0
        {
            if let mount = get_mount_value_from_settings(&*mount_to_buy)
            {
                let can_afford = match mount
                {
                    Mount::Dragon => current_shrooms >= 25,
                    Mount::Tiger => current_shrooms > 0 && current_silver >= 1000,
                    Mount::Horse => current_silver >= 500,
                    Mount::Cow => current_silver >= 100,
                };

                if can_afford
                {
                    session.send_command(Command::BuyMount { mount }).await?;
                    let mount_name = get_mount_name(mount);
                    write_character_log(
                        &character_name,
                        character_id,
                        &format!("MOUNT: Bought {} ({})", mount_name, mount_cost_string(mount)),
                    );
                    return Ok(format!("Bought mount {:?}", mount));
                }
            }
        }
    }
    return Ok("".to_string());
}

fn get_mount_value(character_mount: Option<Mount>) -> i32
{
    match character_mount
    {
        Some(Mount::Cow) => 1,
        Some(Mount::Horse) => 2,
        Some(Mount::Tiger) => 3,
        Some(Mount::Dragon) => 4,
        None => 0,
    }
}
fn get_mount_name(character_mount: Mount) -> String
{
    match character_mount
    {
        Mount::Cow => return String::from("Cow"),
        Mount::Horse => return String::from("Horse"),
        Mount::Tiger => return String::from("Tiger"),
        Mount::Dragon => return String::from("Dragon"),
    }
}

fn mount_cost_string(mount: Mount) -> &'static str
{
    match mount
    {
        Mount::Dragon => "cost: 25 mushrooms",
        Mount::Tiger => "cost: 1000 silver + 1 mushroom",
        Mount::Horse => "cost: 500 silver",
        Mount::Cow => "cost: 100 silver",
    }
}

fn get_mount_value_from_settings(mount_to_buy: &str) -> Mount
{
    match mount_to_buy
    {
        "buyGriffonMount" => Mount::Dragon,
        "buyTigerMount" => Mount::Tiger,
        "buyHorseMount" => Mount::Horse,
        "buyCowMount" => Mount::Cow,
        _ => Mount::Cow,
    }
}
