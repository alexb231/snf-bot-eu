use std::{collections::HashMap, error::Error};

use enum_map::Enum;
use sf_api::{
    command::{AttributeType, Command},
    gamestate::{
        character::Class,
        items::{BagPosition, EquipmentSlot, GemSlot, InventoryType, Item, PlayerItemPlace},
        GameState,
    },
    misc::EnumMapGet,
    SimpleSession,
};
// for EquipmentSlot::into_usize()

fn main_attribute_for_class(class: Class) -> AttributeType
{
    use Class::*;
    match class
    {
        Bard | Mage | Druid | Necromancer => AttributeType::Intelligence,
        Scout | Assassin | PlagueDoctor | DemonHunter => AttributeType::Dexterity,
        Warrior | BattleMage | Berserker | Paladin => AttributeType::Strength,
    }
}

fn score_item_for_class(item: &Item, main_attr: AttributeType) -> i64
{
    let rarity_score = if item.is_legendary()
    {
        2
    }
    else if item.is_epic()
    {
        1
    }
    else
    {
        0
    } as i64;

    let gem_bonus = match item.gem_slot
    {
        Some(GemSlot::Filled(_)) => 2,
        Some(GemSlot::Empty) => 1,
        None => 0,
    } as i64;

    let main_val = *item.attributes.get(main_attr) as i64;

    rarity_score * 1_000_000 + gem_bonus * 100_000 + main_val
}

fn is_better_for_slot(candidate: &Item, current: Option<&Item>, main_attr: AttributeType) -> bool
{
    match current
    {
        None => true,
        Some(old) =>
        {
            let cand_is_epicish = candidate.is_epic() || candidate.is_legendary();
            let old_is_epicish = old.is_epic() || old.is_legendary();

            if cand_is_epicish && !old_is_epicish
            {
                return true;
            }
            if !cand_is_epicish && old_is_epicish
            {
                return false;
            }

            let cand_has_gem = candidate.gem_slot.is_some();
            let old_has_gem = old.gem_slot.is_some();
            if cand_has_gem && !old_has_gem
            {
                return true;
            }
            if !cand_has_gem && old_has_gem
            {
                return false;
            }

            score_item_for_class(candidate, main_attr) > score_item_for_class(old, main_attr)
        }
    }
}

/// Equipment slot index expected by InventoryMove (0-based)
fn equipment_slot_index(slot: EquipmentSlot) -> usize { slot.into_usize() }

pub async fn check_and_swap_equipment(session: &mut SimpleSession) -> Result<String, Box<dyn std::error::Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    let class = gs.character.class;
    let main_attr = main_attribute_for_class(class);

    // Best candidate per equipment slot: (from_place, from_index)
    let mut best_by_slot: HashMap<EquipmentSlot, usize> = HashMap::new();

    for (idx, opt_item) in gs.character.inventory.backpack.iter().enumerate()
    {
        let Some(item) = opt_item
        else
        {
            continue;
        };
        if !item.can_be_equipped_by(class)
        {
            continue;
        }
        let Some(slot) = item.typ.equipment_slot()
        else
        {
            continue;
        };

        let keep = match best_by_slot.get(&slot)
        {
            None => true,
            Some(&cur_idx) =>
            {
                let current_best_item = get_item_ref_from(&gs, cur_idx);
                is_better_for_slot(item, current_best_item, main_attr)
            }
        };

        if keep
        {
            best_by_slot.insert(slot, idx);
        }
    }

    // Plan swaps where candidate beats what's currently equipped
    let mut planned_swaps: Vec<(PlayerItemPlace, usize, EquipmentSlot)> = Vec::new();
    for (&slot, &idx) in best_by_slot.iter()
    {
        let candidate = get_item_ref_from(&gs, idx);
        let Some(candidate) = candidate
        else
        {
            continue;
        };

        let current_equipped = gs.character.equipment.0[slot].as_ref();

        if is_better_for_slot(candidate, current_equipped, main_attr)
        {
            session
                .send_command(Command::InventoryMove {
                    inventory_from: PlayerItemPlace::MainInventory,
                    inventory_from_pos: idx,
                    inventory_to: PlayerItemPlace::Equipment,
                    inventory_to_pos: slot.into_usize(),
                })
                .await?;
        }
    }

    // Execute moves from correct inventory → Equipment
    let mut changes: Vec<String> = Vec::new();

    for (from_place, from_idx, slot) in planned_swaps
    {
        session
            .send_command(Command::InventoryMove {
                inventory_from: from_place,
                inventory_from_pos: from_idx,
                inventory_to: PlayerItemPlace::Equipment,
                inventory_to_pos: slot.into_usize(),
            })
            .await?;

        changes.push(format!("Equipped {:?} from {:?} pos {}.", slot, from_place, from_idx));
    }

    if changes.is_empty()
    {
        Ok("No better gear found to equip from main/extended inventory.".to_string())
    }
    else
    {
        Ok(format!("Swapped equipment:\n{}", changes.join("\n")))
    }
}

fn get_item_ref_from(gs: &GameState, idx: usize) -> Option<&Item> { gs.character.inventory.backpack.get(idx).and_then(|o| o.as_ref()) }
