use std::collections::HashMap;
use enum_map::Enum;
use sf_api::{
    command::{AttributeType, Command},
    gamestate::{
        character::Class,
        items::{
            BagPosition, EquipmentSlot, Gem, GemSlot, GemType, Item, ItemCommandIdent, ItemType,
            PlayerItemPlace, RuneType,
        },
        GameState,
    },
    SimpleSession,
};
use sf_api::misc::EnumMapGet;
use crate::{bot_runner::write_character_log, fetch_character_setting};

const EXTRA_HP_RUNE_MAX: u32 = 15;

fn total_attribute(gs: &GameState, attr: AttributeType) -> u32 {
    gs.character.attribute_basis[attr]
        + gs.character.attribute_additions[attr]
        + gs.character.attribute_times_bought[attr]
}

fn item_attribute(item: &Item, attr: AttributeType) -> u32 {
    *item.attributes.get(attr)
}

fn item_has_socket(item: &Item) -> bool {
    item.gem_slot.is_some()
}

fn item_gem(item: &Item) -> Option<Gem> {
    match item.gem_slot {
        Some(GemSlot::Filled(gem)) => Some(gem),
        _ => None,
    }
}

fn average_weapon_damage(item: &Item) -> Option<f64> {
    match item.typ {
        ItemType::Weapon { min_dmg, max_dmg } => {
            Some((f64::from(min_dmg) + f64::from(max_dmg)) / 2.0)
        }
        _ => None,
    }
}

fn restrict_luck_settings(gs: &GameState) -> (bool, u16) {
    let restrict = fetch_character_setting(gs, "itemsRestrictLuckItems").unwrap_or(false);
    let level_diff: i32 = fetch_character_setting(gs, "itemsRestrictLuckItemsLevelDiff").unwrap_or(0);
    (restrict, level_diff.max(0) as u16)
}

fn can_equip_item(gs: &GameState, item: &Item) -> bool {
    item.can_be_equipped_by(gs.character.class)
}

fn is_item_useful_for(gs: &GameState, item_to_check: &Item, old_item: Option<&Item>) -> bool {
    if !can_equip_item(gs, item_to_check) {
        return false;
    }

    let Some(slot) = item_to_check.typ.equipment_slot() else {
        return false;
    };

    if gs.character.level >= 25 && !item_has_socket(item_to_check) && slot != EquipmentSlot::Shield {
        return false;
    }
    if old_item.is_none() {
        return true;
    }

    let str_val = item_attribute(item_to_check, AttributeType::Strength);
    let dex_val = item_attribute(item_to_check, AttributeType::Dexterity);
    let int_val = item_attribute(item_to_check, AttributeType::Intelligence);
    let con_val = item_attribute(item_to_check, AttributeType::Constitution);
    let luck_val = item_attribute(item_to_check, AttributeType::Luck);

    if con_val > 0 {
        return true;
    }

    if luck_val > 0 {
        let (restrict, level_diff) = restrict_luck_settings(gs);
        if !restrict {
            return true;
        }

        let current_luck = total_attribute(gs, AttributeType::Luck) as f64;
        let mut old_luck = item_attribute(old_item.unwrap(), AttributeType::Luck) as f64;
        if let Some(gem) = item_gem(old_item.unwrap()) {
            if matches!(gem.typ, GemType::Luck | GemType::All) {
                old_luck += gem.value as f64;
            }
        }

        let new_luck = current_luck + f64::from(luck_val) - old_luck;
        let level_cap = f64::from(gs.character.level + level_diff);
        if level_cap > 0.0 {
            let crit_chance = new_luck * 5.0 / (level_cap * 2.0);
            if crit_chance <= 50.0 && f64::from(luck_val) > old_luck {
                return true;
            }
        }
    }

    let main_attr = gs.character.class.main_attribute();
    if str_val > 0 && main_attr == AttributeType::Strength {
        return true;
    }
    if dex_val > 0 && main_attr == AttributeType::Dexterity {
        return true;
    }
    if int_val > 0 && main_attr == AttributeType::Intelligence {
        return true;
    }

    if str_val == 0 && dex_val == 0 {
        return int_val == 0;
    }

    false
}

fn check_item_boost(gs: &GameState, item_to_check: &Item, check_all_possible_slots: bool) -> f64 {
    let Some(slot) = item_to_check.typ.equipment_slot() else {
        return 0.0;
    };

    if !can_equip_item(gs, item_to_check) {
        return 0.0;
    }

    let current_item = gs.character.equipment.0[slot].as_ref();
    let mut same_type_items = vec![current_item];
    if check_all_possible_slots && slot == EquipmentSlot::Weapon && gs.character.class == Class::Assassin {
        same_type_items.push(gs.character.equipment.0[EquipmentSlot::Shield].as_ref());
    }

    let mut item_boost = 0.0;
    for same_type_item in same_type_items {
        let boost = get_item_value(gs, same_type_item, Some(item_to_check));
        if boost > item_boost {
            item_boost = boost;
        }
    }

    item_boost.min(999.99)
}

fn get_item_value(gs: &GameState, current_item: Option<&Item>, item_to_check: Option<&Item>) -> f64 {
    let Some(item_to_check) = item_to_check else {
        return 0.0;
    };

    if current_item.is_some()
        && item_has_socket(current_item.unwrap())
        && !item_has_socket(item_to_check)
    {
        return 0.0;
    }

    let socket_boost =
        if current_item.is_some() && !item_has_socket(current_item.unwrap()) && item_has_socket(item_to_check) {
            1.0
        } else {
            0.0
        };

    let main_attr = gs.character.class.main_attribute();
    let current_item_value = current_item.map_or(0, |item| {
        item_attribute(item, main_attr) + item_attribute(item, AttributeType::Constitution)
    });
    let new_item_value = item_attribute(item_to_check, main_attr)
        + item_attribute(item_to_check, AttributeType::Constitution);

    let attribute_boost = if current_item_value == 0 && new_item_value == 0 {
        0.0
    } else if current_item_value == 0 {
        f64::from(new_item_value) / 1000.0
    } else if new_item_value == 0 {
        -(f64::from(current_item_value) / 1000.0)
    } else {
        f64::from(new_item_value) / f64::from(current_item_value) - 1.0
    };

    let mut damage_boost = 0.0;
    if let Some(new_avg) = average_weapon_damage(item_to_check) {
        let ratio = current_item
            .and_then(average_weapon_damage)
            .filter(|avg| *avg > 0.0)
            .map(|avg| new_avg / avg)
            .unwrap_or(0.2);
        damage_boost = ratio * 5.0;
    }

    let hp_rune_boost = get_hp_rune_value(gs, current_item, item_to_check);
    let current_item_boost = attribute_boost + hp_rune_boost + socket_boost + damage_boost;

    current_item_boost * 100.0
}

fn get_hp_rune_value(gs: &GameState, current_item: Option<&Item>, new_item: &Item) -> f64 {
    let total_hp_rune_bonus: u32 = gs
        .character
        .equipment
        .0
        .values()
        .flatten()
        .filter(|item| {
            if let Some(current_item) = current_item {
                if *item == current_item {
                    return false;
                }
            }
            if *item == new_item {
                return false;
            }
            matches!(item.rune, Some(rune) if rune.typ == RuneType::ExtraHitPoints)
        })
        .map(|item| item.rune.unwrap().value as u32)
        .sum();

    if total_hp_rune_bonus >= EXTRA_HP_RUNE_MAX {
        return 0.0;
    }

    let new_item_rune_hp = match new_item.rune {
        Some(rune) if rune.typ == RuneType::ExtraHitPoints => rune.value as u32,
        _ => 0,
    };
    let remaining = EXTRA_HP_RUNE_MAX - total_hp_rune_bonus;
    f64::from(new_item_rune_hp.min(remaining))
}

/// Equipment slot index expected by InventoryMove (0-based)
fn equipment_slot_index(slot: EquipmentSlot) -> usize {
    slot.into_usize()
}

fn inventory_place_for_bag_pos(pos: BagPosition) -> (PlayerItemPlace, usize) {
    let (inventory_type, inventory_pos) = pos.inventory_pos();
    (inventory_type.player_item_position(), inventory_pos)
}

/// Find an item in current inventory by ident and return its BagPosition + ref.
/// (This is the key to avoid stale BagPositions after swaps.)
fn find_item_in_inventory_by_ident<'a>(
    gs: &'a GameState,
    ident: ItemCommandIdent,
) -> Option<(BagPosition, &'a Item)> {
    gs.character
        .inventory
        .iter()
        .filter_map(|(bag_pos, maybe_item)| maybe_item.map(|it| (bag_pos, it)))
        .find(|(_, it)| it.command_ident() == ident)
}

pub async fn check_and_swap_equipment(
    session: &mut SimpleSession,
) -> Result<String, Box<dyn std::error::Error>>
{
    // Initial snapshot for scanning
    let gs = session.send_command(Command::Update).await?.clone();
    println!("hi");
    let min_boost_percent: f64 = fetch_character_setting(&gs, "itemsEquipSwapMinBoostPercent")
        .unwrap_or(0)
        .max(0) as f64;

    // Best candidate per equipment slot:
    // store IDENT instead of BagPosition (BagPosition becomes stale after swaps)
    let mut best_by_slot: HashMap<EquipmentSlot, (ItemCommandIdent, f64)> = HashMap::new();
    let mut scanned = 0usize;
    let mut equipable = 0usize;
    let mut useful = 0usize;
    let mut boosted = 0usize;
    let mut min_passed = 0usize;

    for (_pos, opt_item) in gs.character.inventory.iter() {
        let Some(item) = opt_item else { continue; };
        scanned += 1;

        let Some(slot) = item.typ.equipment_slot() else { continue; };

        if !can_equip_item(&gs, item) {
            continue;
        }
        equipable += 1;

        let old_item = gs.character.equipment.0[slot].as_ref();
        let slot_empty = old_item.is_none();
        if !slot_empty && !is_item_useful_for(&gs, item, old_item) {
            continue;
        }
        useful += 1;

        let boost = check_item_boost(&gs, item, true);
        if boost > 0.0 {
            boosted += 1;
        }

        if !slot_empty {
            if boost <= 0.0 || boost < min_boost_percent {
                continue;
            }
            min_passed += 1;
        }

        let keep = match best_by_slot.get(&slot) {
            None => true,
            Some((_, best_boost)) => boost > *best_boost,
        };

        if keep {
            best_by_slot.insert(slot, (item.command_ident(), boost));
        }
    }

    // Apply changes sequentially with fresh gamestate each time
    let mut changes: Vec<String> = Vec::new();
    let mut swapped_slots: Vec<String> = Vec::new();

    // deterministic order for stable logs
    let ordered_slots = [
        EquipmentSlot::Hat,
        EquipmentSlot::BreastPlate,
        EquipmentSlot::Gloves,
        EquipmentSlot::FootWear,
        EquipmentSlot::Amulet,
        EquipmentSlot::Belt,
        EquipmentSlot::Ring,
        EquipmentSlot::Talisman,
        EquipmentSlot::Weapon,
        EquipmentSlot::Shield,
    ];

    for slot in ordered_slots {
        let Some((ident, _boost)) = best_by_slot.get(&slot).copied() else { continue; };

        // Refresh state to avoid stale positions after earlier swaps
        let fresh = session.send_command(Command::Update).await?.clone();

        let Some((bag_pos, item)) = find_item_in_inventory_by_ident(&fresh, ident) else {
            changes.push(format!("Skipped {:?}: item not found in inventory anymore.", slot));
            continue;
        };

        // re-check equipability just in case
        if !can_equip_item(&fresh, item) {
            changes.push(format!("Skipped {:?}: cannot equip anymore.", slot));
            continue;
        }

        let (from_place, from_pos) = inventory_place_for_bag_pos(bag_pos);

        session
            .send_command(Command::InventoryMove {
                inventory_from: from_place,
                inventory_from_pos: from_pos,
                inventory_to: PlayerItemPlace::Equipment,
                inventory_to_pos: equipment_slot_index(slot),
                item_ident: item.command_ident(),
            })
            .await?;

        swapped_slots.push(format!("{:?}", slot));
        changes.push(format!("Equipped {:?} from {:?} pos {}.", slot, from_place, from_pos));
    }

    if changes.is_empty() {
        println!("sadsadsad");
        Ok("No better gear found to equip from main/extended inventory.".to_string())
    } else {
        write_character_log(
            &gs.character.name,
            gs.character.player_id,
            &format!(
                "EQUIP_SWAP: scanned={} equipable={} useful={} boosted={} min_passed={} swapped={} slots=[{}]",
                scanned,
                equipable,
                useful,
                boosted,
                min_passed,
                swapped_slots.len(),
                swapped_slots.join(", ")
            ),
        );

        Ok(format!("Swapped equipment:\n{}", changes.join("\n")))
    }
}
