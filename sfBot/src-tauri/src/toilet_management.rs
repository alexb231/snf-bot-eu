use std::error::Error;

use sf_api::{
    command::{BlacksmithAction, Command},
    gamestate::{
        items::{GemSlot, Item, ItemType, PlayerItemPlace, PlayerItemPosition},
        tavern::Toilet,
    },
    SimpleSession,
};

use crate::{fetch_character_setting, inventory_management::sorted_items_with_indices};

const MAX_AURA_THRESHOLD: u32 = 400;

pub async fn use_toilet(session: &mut SimpleSession, throw_epics: bool, throw_normal_items: bool, throw_gems_only: bool, flush_when_full: bool) -> Result<String, Box<dyn Error>>
{
    let result = String::new();
    let gs = session.send_command(Command::Update).await?.clone();
    gs.tavern.toilet.unwrap().sacrifices_left;

    if (throw_epics)
    {
        if let Some(toilet) = gs.tavern.toilet.as_ref()
        {
            
            for _ in 0..toilet.sacrifices_left
            {
                
                throw_cheapest_epic_into_toilet(session, flush_when_full).await?;
            }
        }
    }

    
    if (throw_gems_only)
    {
        if let Some(toilet) = gs.tavern.toilet.as_ref()
        {
            for _ in 0..toilet.sacrifices_left
            {
                throw_gems_only_into_toilet(session, flush_when_full).await?;
            }
        }
    }

    
    if (throw_normal_items)
    {
        if let Some(toilet) = gs.tavern.toilet.as_ref()
        {
            for _ in 0..toilet.sacrifices_left
            {
                throw_cheapest_item_into_toilet(session, flush_when_full).await?;
            }
        }
    }

    return Ok(result);
}

async fn throw_gems_only_into_toilet(session: &mut SimpleSession, flush_when_full: bool) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let free_slot_count = gs.character.inventory.count_free_slots();

    let Some(toilet) = gs.tavern.toilet.as_ref()
    else
    {
        return Ok(());
    };

    let need_to_flush = should_flush(*toilet, free_slot_count, flush_when_full);
    if need_to_flush
    {
        if let Err(_) = session.send_command(Command::ToiletFlush).await
        {
            return Ok(());
        }
    }

    if toilet.sacrifices_left <= 0
    {
        return Ok(());
    }

    let mut gems_in_inventory: Vec<(usize, &Item)> = sorted_items_with_indices(&gs.character.inventory).into_iter().filter(|(_, item)| matches!(item.typ, ItemType::Gem(_))).collect();

    gems_in_inventory.sort_by_key(|&(_, item)| item.price);

    let any_item_price_100 = gems_in_inventory.iter().any(|&(_, item)| item.price == 100);
    let can_use_toilet = toilet.mana_currently != toilet.mana_total && (toilet.aura < MAX_AURA_THRESHOLD || any_item_price_100);

    if can_use_toilet
    {
        if let Some(&(index, item)) = gems_in_inventory.first()
        {
            let toilet_command = Command::ToiletDrop {
                item_pos: PlayerItemPosition {
                    place: PlayerItemPlace::MainInventory,
                    position: index,
                },
            };
            if let Err(err) = session.send_command(toilet_command).await
            {
                eprintln!("Error: throw_gems_only_into_toilet -> ToiletDrop command failed: {}", err);
                return Ok(());
            }
        }
    }

    Ok(())
}

async fn throw_cheapest_epic_into_toilet(session: &mut SimpleSession, flush_when_full: bool) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let exclude_weapons: bool = fetch_character_setting(&gs, "toiletExcludeEpicWeapons").unwrap_or(false);
    let free_slot_count = gs.character.inventory.count_free_slots();

    let Some(toilet) = gs.tavern.toilet.as_ref()
    else
    {
        return Ok(String::from(""));
    };

    let need_to_flush = should_flush(*toilet, free_slot_count, flush_when_full);
    if need_to_flush
    {
        if let Err(_) = session.send_command(Command::ToiletFlush).await
        {
            return Ok(String::from("Pulled toilet"));
        }
    }

    if toilet.sacrifices_left <= 0
    {
        return Ok(String::from(""));
    }

    let mut epic_items: Vec<(usize, &Item)> = sorted_items_with_indices(&gs.character.inventory)
        .into_iter()
        .filter(|(_, item)| {
            let is_weapon = matches!(item.typ, ItemType::Weapon { .. });
            let is_weapon_with_filled_gem = is_weapon && matches!(item.gem_slot, Some(GemSlot::Filled(_)));

            
            if exclude_weapons && is_weapon
            {
                return false;
            }

            
            
            
            

            item.is_epic() && !is_weapon_with_filled_gem
        })
        .collect();

    epic_items.sort_by_key(|&(_, item)| item.price);

    let any_item_price_100 = epic_items.iter().any(|&(_, item)| item.price == 100);
    let can_use_toilet = toilet.mana_currently != toilet.mana_total && (toilet.aura < MAX_AURA_THRESHOLD || any_item_price_100);

    if can_use_toilet
    {
        if let Some(&(index, _item)) = epic_items.first()
        {
            let toilet_command = Command::ToiletDrop {
                item_pos: PlayerItemPosition {
                    place: PlayerItemPlace::MainInventory,
                    position: index,
                },
            };

            if let Err(_) = session.send_command(toilet_command).await
            {
                return Ok(String::from("Error: throw_cheapest_epic_into_toilet -> ToiletDrop command failed"));
            }

            return Ok(String::from(""));
        }
    }

    Ok(String::from("throw cheapest item did nothing it seems"))
}

async fn throw_cheapest_item_into_toilet(session: &mut SimpleSession, flush_when_full: bool) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let free_slot_count = gs.character.inventory.count_free_slots();

    
    let Some(toilet) = gs.tavern.toilet.as_ref()
    else
    {
        return Ok(());
    };

    let need_to_flush = should_flush(*toilet, free_slot_count, flush_when_full);
    if need_to_flush
    {
        if let Err(_) = session.send_command(Command::ToiletFlush).await
        {
            return Ok(());
        }
    }

    if toilet.sacrifices_left == 0
    {
        return Ok(());
    }

    let mut items: Vec<(usize, &Item)> = sorted_items_with_indices(&gs.character.inventory).into_iter().filter(|(_, item)| !matches!(item.typ, ItemType::Potion(_))).collect();

    items.sort_by_key(|&(_, item)| item.price);

    let any_item_price_100 = items.iter().any(|&(_, item)| item.price == 100);
    let can_use_toilet = toilet.mana_currently != toilet.mana_total && (toilet.aura < MAX_AURA_THRESHOLD || any_item_price_100);

    if can_use_toilet
    {
        if let Some(&(index, item)) = items.first()
        {
            let toilet_command = Command::ToiletDrop {
                item_pos: PlayerItemPosition {
                    place: PlayerItemPlace::MainInventory,
                    position: index,
                },
            };

            if let Err(err) = session.send_command(toilet_command).await
            {
                eprintln!("Error: throw_cheapest_item_into_toilet -> ToiletDrop command failed: {}", err);
                return Ok(());
            }
        }
    }

    Ok(())
}

pub fn should_flush(toilet: Toilet, amount_of_free_slots: usize, flush_when_full: bool) -> bool
{
    
    if toilet.mana_currently >= toilet.mana_total && toilet.mana_total > 0
    {
        if (amount_of_free_slots <= 0)
        {
            return false;
        }

        if (flush_when_full)
        {
            return true;
        }
    }
    return false;
}
