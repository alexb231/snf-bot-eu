use std::error::Error;

use sf_api::{
    command::{Command, Command::Update},
    gamestate::{items::EquipmentSlot, GameState},
    SimpleSession,
};

use crate::fetch_character_setting;

pub async fn enchant_items(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = &session.send_command(Command::Update).await?.clone();
    let witch = &gs.witch;
    let mut result = String::from("");
    if gs.character.level < 66 || witch.is_none()
    {
        return Ok(result);
    }

    let equipment = &gs.character.equipment.0;
    for (slot, item) in equipment.iter()
    {
        if (should_enchant(&gs, slot))
        {
            if let Some(item) = item
            {
                if item.enchantment.is_some()
                {
                    continue;
                }
                else
                {
                    let new_gs = session.send_command(Update).await?;

                    if let Some(optional_witch) = witch
                    {
                        if item.enchantment.is_none()
                        {
                            if let Some(slot_enchantment) = slot.enchantment()
                            {
                                let enchantment_indent = optional_witch.enchantments[slot_enchantment];
                                if let Some(enchantment_indent_unwrapped) = enchantment_indent
                                {
                                    if (new_gs.character.silver > optional_witch.enchantment_price)
                                    {
                                        session.send_command(Command::WitchEnchant { enchantment: enchantment_indent_unwrapped }).await?;
                                        result += slot_to_string(slot);
                                        return Ok(format!("Enchanted item: {}", result));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

pub fn should_enchant(gs: &GameState, equipment_slot: EquipmentSlot) -> bool
{
    let result = match (equipment_slot)
    {
        EquipmentSlot::Weapon => "witchEnchantItemWeapon",
        EquipmentSlot::Hat => "witchEnchantItemHat",
        EquipmentSlot::BreastPlate => "witchEnchantItemChest",
        EquipmentSlot::Gloves => "witchEnchantItemGloves",
        EquipmentSlot::FootWear => "witchEnchantItemBoots",
        EquipmentSlot::Amulet => "witchEnchantItemNecklace",
        EquipmentSlot::Belt => "witchEnchantItemBelt",
        EquipmentSlot::Ring => "witchEnchantItemRing",
        EquipmentSlot::Talisman => "witchEnchantItemTalisman",
        _ => "false",
    };

    if (result == "false")
    {
        return false;
    }
    let enchant_setting_for_this_item: bool = fetch_character_setting(&gs, &*result).unwrap_or(false);
    return enchant_setting_for_this_item;
}

pub fn slot_to_string(equipment_slot: EquipmentSlot) -> &'static str
{
    match equipment_slot
    {
        EquipmentSlot::Weapon => "Weapon",
        EquipmentSlot::Hat => "Hat",
        EquipmentSlot::BreastPlate => "Chest",
        EquipmentSlot::Gloves => "Gloves",
        EquipmentSlot::FootWear => "Footwear",
        EquipmentSlot::Amulet => "Amulet",
        EquipmentSlot::Belt => "Belt",
        EquipmentSlot::Ring => "Ring",
        EquipmentSlot::Talisman => "Talisman",
        _ => "None",
    }
}
