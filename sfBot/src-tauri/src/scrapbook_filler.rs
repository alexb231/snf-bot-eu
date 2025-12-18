use std::error::Error;

use chrono::{Duration, Local};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sf_api::{
    command::{AttributeType, Command},
    SimpleSession,
};
use strum::IntoEnumIterator;
#[derive(Serialize)]
struct ScrapbookRequest
{
    raw_scrapbook: String,
    server: String,
    max_attrs: u64,
}

#[derive(Debug, Deserialize)]
struct PlayerResponse
{
    player_name: String,
    new_count: u32,
}

pub async fn fill_scrapbook(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let mut result = String::from("");

    let gs = session.send_command(Command::CheckArena).await?.clone();

    let current_time = Local::now();
    let current_time_minus_1 = current_time - Duration::minutes(1);

    let free_fight = if let Some(next_free_fight) = gs.arena.next_free_fight { current_time_minus_1 >= next_free_fight } else { false };

    if !free_fight
    {
        return Ok(String::from(""));
    }

    let raw_scrapbook = match &gs.character.scrapbook
    {
        Some(scrapbook) => scrapbook.raw_data.clone(),
        None =>
        {
            println!("No scrapbook found, skipping...");
            return Ok("".into());
        }
    };

    let total_stats: u64 = AttributeType::iter().map(|attr| gs.character.attribute_basis[attr] as u64 + gs.character.attribute_additions[attr] as u64 + gs.character.attribute_times_bought[attr] as u64).sum();

    let max_attrs = (total_stats as f64 * 0.8).round() as u64;

    let client = Client::new();
    let payload = ScrapbookRequest {
        raw_scrapbook,
        server: session.server_url().to_string(),
        max_attrs,
    };

    let response = client.post("https://mfbot-api.marenga.dev/scrapbook_advice").json(&payload).send().await?;

    if response.status().is_success()
    {
        let players: Vec<PlayerResponse> = response.json().await?;
        let mut rng = StdRng::from_entropy(); // <- Send-kompatibler RNG otherwise tauri complains
        if let Some(random_player) = players.choose(&mut rng)
        {
            let fight_player = Command::Fight {
                name: random_player.player_name.clone(),
                use_mushroom: false,
            };
            session.send_command(fight_player).await?;
            return Ok(format!("Fought opponent: {}", random_player.player_name));
        }
    }
    else
    {
        println!("Error: {}", response.status());
        return Ok("".into());
    }

    Ok(result)
}

fn ensure_https(url: &str) -> String
{
    if url.starts_with("http://") || url.starts_with("https://")
    {
        url.to_string()
    }
    else
    {
        format!("https://{}", url)
    }
}
