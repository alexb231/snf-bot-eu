#![allow(warnings)]
use std::{any::type_name, collections::HashSet, error::Error, fmt::Debug};

use sf_api::{
    command::{Command, Command::BrewPotion, ShopType},
    gamestate::{
        character::Class,
        fortress::{Fortress, FortressBuildingType},
        guild::Guild,
        items::{BagPosition, EquipmentSlot, GemSlot, GemType, Inventory, InventoryType, Item, ItemCommandIdent, ItemPlace, ItemPlace::MainInventory, ItemType, PetItem, PlayerItemPlace, Potion, PotionSize, PotionType},
        rewards::Event,
        unlockables::{HabitatType, Pets},
        GameState,
    },
    SimpleSession,
};

use crate::{bot_runner::write_character_log, equipment_swapping::check_and_swap_equipment, fetch_character_setting, lottery::sleep_between_commands};

fn check_type<T: std::fmt::Debug>(x: T)
{
    println!("{:?} is of type {}", x, type_name::<T>());
}

// TODO think about dismantling
pub async fn manage_inventory(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let mut gs = session.send_command(Command::Update).await?.clone();

    let amount_of_slots_to_keep_free: i32 = std::cmp::min(fetch_character_setting(&gs, "itemsInventorySlotsToBeLeft").unwrap_or(0), 1);
    let sell_items_to_witch: bool = fetch_character_setting(&gs, "itemsImmediatelyThrowIntoCauldron").unwrap_or(false);
    let exclude_epics_from_witch_selling: bool = fetch_character_setting(&gs, "itemsImmediatelyThrowIntoCauldronExceptEpics").unwrap_or(false); // excludes epics
    let equip_before_selling: bool = fetch_character_setting(&gs, "itemsEquipBeforeSelling").unwrap_or(false);

    if equip_before_selling
    {
        check_and_swap_equipment(session).await?;
        gs = session.send_command(Command::Update).await?.clone();
    }

    let mut free_slots = gs.character.inventory.count_free_slots();
    drink_potions(session).await?;
    sell_potions(session).await?;
    sell_gems(session).await?;
    buy_potions_and_hourglasses(session).await?;
    check_for_pet_egg(session).await?;

    // witch selling
    if (sell_items_to_witch)
    {
        let mut events = &gs.specials.events.active;
        let witch_event_active = events.contains(&Event::WitchesDance);
        sell_item_to_witch(session, witch_event_active, exclude_epics_from_witch_selling).await?;
    }
    gs = session.send_command(Command::Update).await?.clone();
    free_slots = gs.character.inventory.count_free_slots();

    if free_slots > amount_of_slots_to_keep_free as usize
    {
        return Ok(String::from(""));
    }

    // only when inventory is full
    let result = sell_two_cheapest_items(session, exclude_epics_from_witch_selling).await?;

    Ok(result)
}

pub async fn buy_potions_and_hourglasses(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let mut msg = String::new();

    let magic_shop = &gs.shops[ShopType::Magic];
    let magic_shop_items = &magic_shop.items;

    let free_slot = match gs.character.inventory.free_slot()
    {
        Some(slot) => slot,
        None => return Ok(msg),
    };

    // command-safe:
    let (inv_type, inv_pos) = free_slot.inventory_pos();
    let inventory: PlayerItemPlace = inv_type.player_item_position();

    for (shop_pos, item) in magic_shop_items.iter().enumerate()
    {
        if !potion_mapping_from_string_settings_for_buying(&gs, item)
        {
            continue;
        }

        if gs.character.silver <= item.price as u64
        {
            continue;
        }

        if item.mushroom_price > 0
        {
            continue;
        }

        if item.typ == ItemType::QuickSandGlass
        {
            session.send_command(Command::BuyShopHourglas { shop_type: ShopType::Magic, shop_pos }).await?;
            msg.push_str("Bought Hourglas from magic shop");
            return Ok(msg);
        }

        if let Err(e) = session
            .send_command(Command::BuyShop {
                shop_type: ShopType::Magic,
                shop_pos,
                inventory,
                inventory_pos: inv_pos,
                item_ident: item.command_ident(),
            })
            .await
        {
            eprintln!("BuyShop failed: {:?}", e);
            return Err(e.into());
        }
        msg.push_str("Bought Potion from magic shop");
        return Ok(msg);
    }
    Ok(msg)
}

pub async fn check_whether_hourglas_is_in_inv(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let inv = &gs.character.inventory;

    for (bag_pos, item) in sorted_items_with_bagpos(inv)
    {
        if item.typ == ItemType::QuickSandGlass
        {
            let (from, from_pos) = bag_to_itemplace(bag_pos);

            session.send_command(Command::UsePotion { from, from_pos, item_ident: item.command_ident() }).await?;
        }
    }
    Ok(())
}

pub async fn drink_potions(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    let character_name = gs.character.name.clone();
    let character_id = gs.character.player_id;

    let character_pots = &gs.character.active_potions;
    let amount_of_active_potions = character_pots.iter().filter(|p| p.is_some()).count();
    let all_pots_are_none = character_pots.iter().all(|p| p.is_none());

    let inv = &gs.character.inventory;

    let mut potion_to_drink: Vec<(BagPosition, ItemCommandIdent, ItemType)> = Vec::new();

    for (bag_pos, item) in sorted_items_with_bagpos(inv)
    {
        if let ItemType::Potion(potion) = &item.typ
        {
            if should_drink_potion_from_settings(&gs, potion.typ, potion.size)
            {
                potion_to_drink.push((bag_pos, item.command_ident(), item.typ.clone()));
            }
        }
    }

    if potion_to_drink.is_empty()
    {
        return Ok(());
    }

    if amount_of_active_potions < 3 || all_pots_are_none
    {
        for (bag_pos, ident, item_type) in potion_to_drink
        {
            let (from, from_pos) = bag_to_itemplace(bag_pos);

            if let Err(err) = session.send_command(Command::UsePotion { from, from_pos, item_ident: ident }).await
            {
                eprintln!("Error: drink_potions UsePotion failed: {}", err);
                return Ok(());
            }

            if let ItemType::Potion(p) = item_type
            {
                write_character_log(&character_name, character_id, &format!("POTION: drank typ={:?} size={:?} pos={}", p.typ, p.size, from_pos));
            }
        }
    }
    else
    {
        for (bag_pos, ident, item_type) in &potion_to_drink
        {
            if let ItemType::Potion(potion) = item_type
            {
                for pot in character_pots.iter().flatten()
                {
                    if pot.typ == potion.typ && pot.size == potion.size
                    {
                        let (from, from_pos) = bag_to_itemplace(*bag_pos);

                        session.send_command(Command::UsePotion { from, from_pos, item_ident: *ident }).await?;

                        write_character_log(&character_name, character_id, &format!("POTION: drank typ={:?} size={:?} pos={}", potion.typ, potion.size, from_pos));
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn check_for_pet_egg(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let mut msg = String::new();

    // Need space to receive item
    let free_slot = match gs.character.inventory.free_slot()
    {
        Some(slot) => slot,
        None => return Ok(msg),
    };

    // command-safe slot mapping
    let (inv_type, inv_pos) = free_slot.inventory_pos();
    let inventory = inv_type.player_item_position();

    let weapon_shop = &gs.shops[ShopType::Weapon];
    let weapon_shop_items = &weapon_shop.items;

    for (shop_pos, item) in weapon_shop_items.iter().enumerate()
    {
        // keep your "buy filter" (even if the name is potion_...)
        if !potion_mapping_from_string_settings_for_buying(&gs, item)
        {
            continue;
        }

        if gs.character.silver <= item.price as u64
        {
            continue;
        }

        if item.mushroom_price > 0
        {
            continue;
        }

        // only eggs you wanted (same as your old checks)
        let is_target_egg = matches!(
            item.typ,
            ItemType::PetItem { typ: PetItem::Egg(HabitatType::Water) } | ItemType::PetItem { typ: PetItem::SpecialEgg(HabitatType::Water) } | ItemType::PetItem { typ: PetItem::GoldenEgg }
        );

        if !is_target_egg
        {
            continue;
        }

        // You said this command is correct in your project:
        // Try it first, but keep a fallback that is definitely command-safe.
        match session.send_command(Command::BuyShopHourglas { shop_type: ShopType::Weapon, shop_pos }).await
        {
            Ok(_) =>
            {
                msg.push_str("Bought pet egg from weapon shop.");
                return Ok(msg);
            }
            Err(e) =>
            {
                // Fallback: classic BuyShop that uses the free slot mapping
                // (prevents future weirdness if BuyShopHourglas rejects eggs on some servers)
                eprintln!("[WARN] BuyShopHourglas failed for egg at shop_pos {}: {}. Falling back to BuyShop.", shop_pos, e);

                session
                    .send_command(Command::BuyShop {
                        shop_type: ShopType::Weapon,
                        shop_pos,
                        inventory,
                        inventory_pos: inv_pos,
                        item_ident: item.command_ident(),
                    })
                    .await?;

                msg.push_str("Bought pet egg from weapon shop.");
                return Ok(msg);
            }
        }
    }

    Ok(msg)
}

pub async fn sell_potions(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    let character_name = gs.character.name.clone();
    let character_id = gs.character.player_id;

    let inv = &gs.character.inventory;

    let mut potions_to_sell: Vec<(BagPosition, ItemCommandIdent, ItemType)> = Vec::new();

    for (bag_pos, item) in sorted_items_with_bagpos(inv)
    {
        if let ItemType::Potion(potion) = &item.typ
        {
            if should_sell_potion_from_settings(&gs, potion.typ, potion.size)
            {
                potions_to_sell.push((bag_pos, item.command_ident(), item.typ.clone()));
            }
        }
    }

    for (bag_pos, ident, item_type) in potions_to_sell
    {
        let (inventory, inventory_pos) = bag_to_playerplace(bag_pos);

        session.send_command(Command::SellShop { inventory, inventory_pos, item_ident: ident }).await?;

        if let ItemType::Potion(potion) = item_type
        {
            write_character_log(&character_name, character_id, &format!("SELL: potion typ={:?} size={:?} pos={}", potion.typ, potion.size, inventory_pos));
        }
    }

    Ok(())
}

pub async fn sell_gems(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    let character_name = gs.character.name.clone();
    let character_id = gs.character.player_id;

    let inv = &gs.character.inventory;

    let mut gems_to_sell: Vec<(BagPosition, ItemCommandIdent, ItemType)> = Vec::new();

    for (bag_pos, item) in sorted_items_with_bagpos(inv)
    {
        if let ItemType::Gem(gem) = &item.typ
        {
            if should_sell_gem(&gs, gem.typ, gem.value)
            {
                gems_to_sell.push((bag_pos, item.command_ident(), item.typ.clone()));
            }
        }
    }

    for (bag_pos, ident, item_type) in gems_to_sell
    {
        let (inventory, inventory_pos) = bag_to_playerplace(bag_pos);

        session.send_command(Command::SellShop { inventory, inventory_pos, item_ident: ident }).await?;

        if let ItemType::Gem(gem) = item_type
        {
            write_character_log(&character_name, character_id, &format!("SELL: gem typ={:?} value={} pos={}", gem.typ, gem.value, inventory_pos));
        }
    }

    Ok(())
}

pub async fn sell_item_to_witch(session: &mut SimpleSession, witch_event_active: bool, exclude_epics_from_witch_selling: bool) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let character_name = gs.character.name.clone();
    let character_id = gs.character.player_id;

    if gs.character.level < 66
    {
        return Ok(());
    }

    loop
    {
        let gs = session.send_command(Command::Update).await?.clone();
        let inventory = &gs.character.inventory;

        let the_witch = match &gs.witch
        {
            Some(witch) => witch,
            None => return Ok(()),
        };

        // --- Witches Dance event: dump items to cauldron ---
        if witch_event_active
        {
            if exclude_epics_from_witch_selling
            {
                // no epics/legys
                let items_to_drop = collect_items_to_sell_no_epics_no_legendary(inventory);

                for bag_pos in items_to_drop
                {
                    // capture info for log
                    let item_info = inventory.iter().find_map(|(p, it)| (p == bag_pos).then_some(it).flatten()).map(|item| (item.typ.clone(), item.price));

                    let (inv_place, inv_pos) = bag_to_playerplace(bag_pos);

                    let drop_command = Command::WitchDropCauldron { inventory_t: inv_place, position: inv_pos };

                    session.send_command(drop_command).await?;

                    if let Some((item_typ, item_price)) = item_info
                    {
                        write_character_log(&character_name, character_id, &format!("WITCH: dropped pos={} item={:?} price={}", inv_pos, item_typ, item_price));
                    }
                }

                return Ok(());
            }
            else
            {
                // includes epics/legys
                let items_to_drop = collect_items_to_sell_including_epics_and_legendaries(inventory);

                for bag_pos in items_to_drop
                {
                    let item_info = inventory.iter().find_map(|(p, it)| (p == bag_pos).then_some(it).flatten()).map(|item| (item.typ.clone(), item.price));

                    let (inv_place, inv_pos) = bag_to_playerplace(bag_pos);

                    let drop_command = Command::WitchDropCauldron { inventory_t: inv_place, position: inv_pos };

                    session.send_command(drop_command).await?;

                    if let Some((item_typ, item_price)) = item_info
                    {
                        write_character_log(&character_name, character_id, &format!("WITCH: dropped pos={} item={:?} price={}", inv_pos, item_typ, item_price));
                    }
                }

                return Ok(());
            }
        }

        // --- Normal Witch requirement (non-event) ---
        if let Some(required_slot) = the_witch.required_item
        {
            if let Some((bag_pos, item)) = find_required_item_for_witch(inventory, required_slot, exclude_epics_from_witch_selling)
            {
                let item_typ = item.typ.clone();
                let item_price = item.price;

                let (inv_place, inv_pos) = bag_to_playerplace(bag_pos);

                let drop_command = Command::WitchDropCauldron { inventory_t: inv_place, position: inv_pos };

                session.send_command(drop_command).await?;

                write_character_log(&character_name, character_id, &format!("WITCH: dropped pos={} item={:?} price={}", inv_pos, item_typ, item_price));

                return Ok(());
            }
            else
            {
                return Ok(());
            }
        }
        else
        {
            return Ok(());
        }
    }
}

fn collect_items_to_sell_including_epics_and_legendaries(inventory: &Inventory) -> Vec<BagPosition>
{
    //
    inventory.iter().filter_map(|(bag_pos, maybe_item)| maybe_item.filter(|item| items_to_sell_or_should_sell_to_witch_includes_epics_and_legendaries(item)).map(|_| bag_pos)).collect()
}

fn collect_items_to_sell_no_epics_no_legendary(inventory: &Inventory) -> Vec<BagPosition>
{
    //
    inventory.iter().filter_map(|(bag_pos, maybe_item)| maybe_item.filter(|item| items_to_sell_or_should_sell_to_witch_no_epic_no_legendarys(item)).map(|_| bag_pos)).collect()
}

// filter logic for later we will most likely need something reusable
fn items_to_sell_or_should_sell_to_witch_no_epic_no_legendarys(item: &Item) -> bool
{
    // > 0 is required for items without value theyll throw errors
    item.price > 0 && !item.is_legendary() && !item.is_epic() && !matches!(item.typ, ItemType::Potion(_)) && !matches!(item.typ, ItemType::Gem(_))
}

fn items_to_sell_or_should_sell_to_witch_includes_epics_and_legendaries(item: &Item) -> bool
{
    // silver value > 0 is required for items without value theyll throw errors
    item.price > 0 && item.price != 100 && !matches!(item.typ, ItemType::Potion(_)) && !matches!(item.typ, ItemType::Gem(_))
}

pub async fn sell_two_cheapest_items(session: &mut SimpleSession, exclude_epics_from_witch_selling: bool) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let character_class = &gs.character.class;
    let inv = &gs.character.inventory;

    let mut items_to_sell: Vec<(BagPosition, ItemCommandIdent, ItemType, i64)> = Vec::new();

    for (bag_pos, item) in sorted_items_with_bagpos(inv)
    {
        if let ItemType::Gem(_) = item.typ
        {
            continue;
        }

        if let ItemType::Potion(potion) = &item.typ
        {
            if !should_sell_potion_non_specific(character_class, potion)
            {
                continue;
            }
        }

        if exclude_epics_from_witch_selling && (item.is_epic() || item.is_legendary())
        {
            continue;
        }

        if matches!(item.gem_slot, Some(GemSlot::Filled(_)))
        {
            continue;
        }

        items_to_sell.push((bag_pos, item.command_ident(), item.typ.clone(), item.price as i64));

        if items_to_sell.len() == 2
        {
            break;
        }
    }

    for (bag_pos, ident, item_type, item_price) in items_to_sell
    {
        let (inventory, inventory_pos) = bag_to_playerplace(bag_pos);

        session.send_command(Command::SellShop { inventory, inventory_pos, item_ident: ident }).await?;

        write_character_log(&gs.character.name, gs.character.player_id, &format!("SELL: item typ={:?} price={} pos={}", item_type, item_price, inventory_pos));
    }

    Ok(String::from("items have been sold"))
}

fn should_sell_potion_non_specific(class: &Class, potion: &Potion) -> bool
{
    // TODO CONFIG unfinished anyway just a workaround for now
    match (potion.typ, potion.size)
    {
        // winged pot: Never sell
        (PotionType::EternalLife, _) => false,

        // hp pot sell if its not a large potion
        (PotionType::Constitution, PotionSize::Large) => false,
        (PotionType::Constitution, _) => true,

        // int pot: only sell large potions if class doesnt use int
        (PotionType::Intelligence, PotionSize::Large) => !matches!(class, Class::Bard | Class::Mage | Class::Druid | Class::Necromancer),
        (PotionType::Intelligence, _) => true,

        // dex pot: oOnly sell large potions if class doesnt use dex
        (PotionType::Dexterity, PotionSize::Large) => !matches!(class, Class::Scout | Class::Assassin | Class::DemonHunter),
        (PotionType::Dexterity, _) => true,

        // str pot: only sell large potions if class doesnt use str
        (PotionType::Strength, PotionSize::Large) => !matches!(class, Class::Warrior | Class::BattleMage | Class::Berserker),
        (PotionType::Strength, _) => true,

        // kinda useless always sell
        (PotionType::Luck, _) => true,
    }
}

fn should_sell_potion_from_settings(gs: &GameState, potion_type: PotionType, potion_size: PotionSize) -> bool
{
    let result = potion_mapping_from_string_settings(potion_type, potion_size);
    let potion_setting: String = fetch_character_setting(&gs, &*result).unwrap_or("keep".to_string());
    return potion_setting == "sell";
}

fn should_sell_gem(gs: &GameState, gem_type: GemType, gem_value: u32) -> bool
{
    let result = gem_mapping_from_string_settings(gem_type);
    let gem_setting: String = fetch_character_setting(&gs, &*result).unwrap_or("keep".to_string());
    if (gem_setting == "keep")
    {
        return check_gems_in_equipment_and_decide_whether_to_sell(gs, gem_value);
    }

    return true;
}

fn gem_mapping_from_string_settings(gem_type: GemType) -> String
{
    let result = match (gem_type)
    {
        GemType::Strength => "itemsGemStrength",
        GemType::Dexterity => "itemsGemDex",
        GemType::Intelligence => "itemsGemInt",
        GemType::Constitution => "itemsGemConst",
        GemType::Luck => "itemsGemLuck",
        GemType::All => "itemsGemBlack",
        GemType::Legendary => "itemsGemLegendary",
        _ => "no_sell",
    };

    return result.to_string();
}

fn should_drink_potion_from_settings(gs: &GameState, potion_type: PotionType, potion_size: PotionSize) -> bool
{
    let result = potion_mapping_from_string_settings(potion_type, potion_size);
    let potion_setting: String = fetch_character_setting(&gs, &*result).unwrap_or("keep".to_string());
    return potion_setting == "drink";
}

fn potion_mapping_from_string_settings(potion_type: PotionType, mut potion_size: PotionSize) -> String
{
    if (potion_type == PotionType::EternalLife)
    {
        potion_size = PotionSize::Large;
    }

    let result = match (potion_type, potion_size)
    {
        (PotionType::EternalLife, PotionSize::Large) => "itemsPotionsWinged",
        (PotionType::Strength, PotionSize::Large) => "itemsPotionsStrLarge",
        (PotionType::Strength, PotionSize::Medium) => "itemsPotionsStrMedium",
        (PotionType::Strength, PotionSize::Small) => "itemsPotionsStrSmall",

        (PotionType::Dexterity, PotionSize::Large) => "itemsPotionsDexLarge",
        (PotionType::Dexterity, PotionSize::Medium) => "itemsPotionsDexMedium",
        (PotionType::Dexterity, PotionSize::Small) => "itemsPotionsDexSmall",

        (PotionType::Intelligence, PotionSize::Large) => "itemsPotionsIntLarge",
        (PotionType::Intelligence, PotionSize::Medium) => "itemsPotionsIntMedium",
        (PotionType::Intelligence, PotionSize::Small) => "itemsPotionsIntSmall",

        (PotionType::Luck, PotionSize::Large) => "itemsPotionsLuckLarge",
        (PotionType::Luck, PotionSize::Medium) => "itemsPotionsLuckMedium",
        (PotionType::Luck, PotionSize::Small) => "itemsPotionsLuckSmall",

        (PotionType::Constitution, PotionSize::Large) => "itemsPotionsConstLarge",
        (PotionType::Constitution, PotionSize::Medium) => "itemsPotionsConstMedium",
        (PotionType::Constitution, PotionSize::Small) => "itemsPotionsConstSmall",
        _ => "keep",
    };

    return result.to_string();
}

fn potion_mapping_from_string_settings_for_buying(gs: &GameState, item: &Item) -> bool
{
    let result = match &item.typ
    {
        ItemType::Potion(potion) => match (&potion.typ, &potion.size)
        {
            (PotionType::EternalLife, _) => "itemsPotionsWingedBuy".to_string(),
            (PotionType::Strength, PotionSize::Large) => "itemsPotionsStrLargeBuy".to_string(),
            (PotionType::Strength, PotionSize::Medium) => "itemsPotionsStrMediumBuy".to_string(),
            (PotionType::Strength, PotionSize::Small) => "itemsPotionsStrSmallBuy".to_string(),

            (PotionType::Dexterity, PotionSize::Large) => "itemsPotionsDexLargeBuy".to_string(),
            (PotionType::Dexterity, PotionSize::Medium) => "itemsPotionsDexMediumBuy".to_string(),
            (PotionType::Dexterity, PotionSize::Small) => "itemsPotionsDexSmallBuy".to_string(),

            (PotionType::Intelligence, PotionSize::Large) => "itemsPotionsIntLargeBuy".to_string(),
            (PotionType::Intelligence, PotionSize::Medium) => "itemsPotionsIntMediumBuy".to_string(),
            (PotionType::Intelligence, PotionSize::Small) => "itemsPotionsIntSmallBuy".to_string(),

            (PotionType::Luck, PotionSize::Large) => "itemsPotionsLuckLargeBuy".to_string(),
            (PotionType::Luck, PotionSize::Medium) => "itemsPotionsLuckMediumBuy".to_string(),
            (PotionType::Luck, PotionSize::Small) => "itemsPotionsLuckSmallBuy".to_string(),

            (PotionType::Constitution, PotionSize::Large) => "itemsPotionsConstLargeBuy".to_string(),
            (PotionType::Constitution, PotionSize::Medium) => "itemsPotionsConstMediumBuy".to_string(),
            (PotionType::Constitution, PotionSize::Small) => "itemsPotionsConstSmallBuy".to_string(),

            _ => "placeholder".to_string(),
        },
        ItemType::QuickSandGlass => "itemsMagicShopBuyHourglasses".to_string(),
        _ => "placeholder".to_string(),
    };

    let setting_result: bool = fetch_character_setting(&gs, &*result).unwrap_or(false);

    return setting_result;
}

pub fn get_gem_mine_level(fortress: &Option<Fortress>) -> i32
{
    let gem_mine_level = match fortress
    {
        Some(fort) => fort.buildings[FortressBuildingType::GemMine].level,
        None => 0,
    };

    return gem_mine_level as i32;
}

pub fn get_hok_points(guild: &Option<Guild>) -> i32
{
    let guild_members = match guild
    {
        Some(g) => &g.members,
        None => &Vec::new(),
    };

    if guild_members.is_empty()
    {
        return 0;
    }

    let mut hok_points: u16 = 0;
    for member in guild_members
    {
        hok_points += member.knights as u16;
    }

    return hok_points as i32;
}

fn gem_formula_gem_mine_up_to_lvl_twenty(char_level: u16, gem_factor: f64, mine_level: i32, hall_of_knights: i32) -> f64
{
    //
    return char_level as f64 * gem_factor * (1.0 + (mine_level as f64 - 1.0) * 0.15) + hall_of_knights as f64 / 3.0;
}

pub fn find_required_item_for_witch<'a>(inventory: &'a Inventory, required_slot: EquipmentSlot, exclude_epics_from_witch_selling: bool) -> Option<(BagPosition, &'a Item)>
{
    let required_type = equipment_slot_to_item_type(required_slot);

    let item_matches = |item: &ItemType| match (&required_type, item)
    {
        (ItemType::Weapon { .. }, ItemType::Weapon { .. }) => true,
        (ItemType::Weapon { .. }, ItemType::Shield { .. }) => true, // treat shield as weapon
        (ItemType::Shield { .. }, ItemType::Shield { .. }) => true,
        (ItemType::Amulet, ItemType::Amulet) => true,
        (ItemType::Belt, ItemType::Belt) => true,
        (ItemType::Ring, ItemType::Ring) => true,
        (ItemType::Talisman, ItemType::Talisman) => true,
        (ItemType::Hat, ItemType::Hat) => true,
        (ItemType::BreastPlate, ItemType::BreastPlate) => true,
        (ItemType::Gloves, ItemType::Gloves) => true,
        (ItemType::FootWear, ItemType::FootWear) => true,
        (a, b) => a == b,
    };

    inventory.iter().filter_map(|(bag_pos, maybe_item)| maybe_item.map(|item| (bag_pos, item))).find(|(_, item)| {
        item_matches(&item.typ)
            && if exclude_epics_from_witch_selling
            {
                items_to_sell_or_should_sell_to_witch_no_epic_no_legendarys(item)
            }
            else
            {
                items_to_sell_or_should_sell_to_witch_includes_epics_and_legendaries(item)
            }
    })
}

fn equipment_slot_to_item_type(slot: EquipmentSlot) -> ItemType
{
    match slot
    {
        EquipmentSlot::Hat => ItemType::Hat,
        EquipmentSlot::BreastPlate => ItemType::BreastPlate,
        EquipmentSlot::Gloves => ItemType::Gloves,
        EquipmentSlot::FootWear => ItemType::FootWear,
        EquipmentSlot::Amulet => ItemType::Amulet,
        EquipmentSlot::Belt => ItemType::Belt,
        EquipmentSlot::Ring => ItemType::Ring,
        EquipmentSlot::Talisman => ItemType::Talisman,
        EquipmentSlot::Weapon => ItemType::Weapon { min_dmg: 0, max_dmg: 0 },
        EquipmentSlot::Shield => ItemType::Shield { block_chance: 0 },
    }
}

#[derive(Debug)]
pub enum GemSize
{
    Small,
    Medium,
    Large,
}

// TODO check for later might be useful to keep legy gems who knows

pub fn check_gems_in_equipment_and_decide_whether_to_sell(gs: &GameState, gem_value: u32) -> bool
{
    let equipment = &gs.character.equipment;
    let mut worst_equipped_legendary_value: u32 = u32::MAX;

    for (slot, item_option) in &equipment.0
    {
        if let Some(item) = item_option
        {
            if let Some(gem_slot) = &item.gem_slot
            {
                if let GemSlot::Filled(gem) = gem_slot
                {
                    if gem.value < worst_equipped_legendary_value
                    {
                        worst_equipped_legendary_value = gem.value;
                    }
                }
            }
        }
    }
    let mut gem_percent_to_keep_from_settings: i32 = fetch_character_setting(&gs, "itemsKeepGemPercent").unwrap_or(0);
    if (gem_percent_to_keep_from_settings == 0)
    {
        return false;
    }

    let percent_settings: f64 = gem_percent_to_keep_from_settings as f64 / 100.0;
    let threshold_value = ((worst_equipped_legendary_value as f64) * (1.0 + percent_settings)).round() as u32;

    // println!("Will we sell the gem with value {}: {}", gem_value, gem_value <=
    // threshold_value);
    return gem_value <= threshold_value;
}

pub async fn brew_potions_using_pet_fruits(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();

    if gs.character.inventory.free_slot().is_none()
    {
        return Ok("".to_string());
    }

    let pets = match gs.pets
    {
        None => return Ok("".to_string()),
        Some(pets) => pets,
    };

    let habitats = &pets.habitats;
    let mut brewed_any = false;

    for (index, habitat) in habitats.iter().enumerate()
    {
        sleep_between_commands(50).await;

        if habitat.1.fruits <= 10
        {
            continue;
        }

        // Refresh before brew
        let pre = session.send_command(Command::Update).await?.clone();
        if pre.character.inventory.free_slot().is_none()
        {
            break;
        }

        // Snapshot idents (Vec)
        let pre_idents: Vec<ItemCommandIdent> = sorted_items_with_bagpos(&pre.character.inventory).into_iter().map(|(_, it)| it.command_ident()).collect();

        // Brew
        session
            .send_command(Command::Custom {
                cmd_name: "PlayerWitchBrewPotion".to_string(),
                arguments: vec![index.to_string()],
            })
            .await?;

        brewed_any = true;

        // Refresh after brew
        let post = session.send_command(Command::Update).await?.clone();

        // Find new potion
        let mut new_potion: Option<(BagPosition, ItemCommandIdent)> = None;

        for (bag_pos, item) in sorted_items_with_bagpos(&post.character.inventory)
        {
            let ident = item.command_ident();
            let existed_before = pre_idents.iter().any(|x| *x == ident);
            if existed_before
            {
                continue;
            }

            if matches!(item.typ, ItemType::Potion(_))
            {
                new_potion = Some((bag_pos, ident));
                break;
            }
        }

        if let Some((bag_pos, ident)) = new_potion
        {
            let (inventory, inventory_pos) = bag_to_playerplace(bag_pos);

            session.send_command(Command::SellShop { inventory, inventory_pos, item_ident: ident }).await?;
        }
        else
        {
            eprintln!("[WARN] Brewed a potion (habitat index {}), but could not detect the new potion in inventory.", index);
        }

        manage_inventory(session).await?;
    }

    Ok(if brewed_any { "brewed and sold potion".to_string() } else { "".to_string() })
}

pub fn sorted_items_with_bagpos(inventory: &Inventory) -> Vec<(BagPosition, &Item)>
{
    let mut items: Vec<(BagPosition, &Item)> = inventory
        .iter() // <- liefert (BagPosition, Option<&Item>)
        .filter_map(|(pos, it)| it.map(|item| (pos, item)))
        .collect();

    items.sort_by_key(|(_, item)| item.price);
    items
}

/// Für Commands, die ItemPlace + pos brauchen (UsePotion, ItemMove, etc.)
pub fn bag_to_itemplace(bag: BagPosition) -> (ItemPlace, usize)
{
    let (inv_type, pos) = bag.inventory_pos();
    (inv_type.item_position(), pos)
}

/// Für Commands, die PlayerItemPlace + pos brauchen (SellShop,
/// WitchDropCauldron, BuyShop, ...)
pub fn bag_to_playerplace(bag: BagPosition) -> (PlayerItemPlace, usize)
{
    let (inv_type, pos) = bag.inventory_pos();
    (inv_type.player_item_position(), pos)
}

pub fn sorted_items_with_indices(inventory: &Inventory) -> Vec<(usize, &Item)>
{
    let mut items_with_indices: Vec<_> = inventory.backpack.iter().enumerate().filter_map(|(pos, item_option)| item_option.as_ref().map(|item| (pos, item))).collect();

    items_with_indices.sort_by_key(|(_, item)| item.price);

    items_with_indices
}

#[inline]
pub fn item_at_bagpos(inventory: &Inventory, pos: BagPosition) -> Option<&Item>
{
    inventory.iter().find_map(|(p, it)| {
        if p == pos
        {
            it
        }
        else
        {
            None
        }
    })
}

pub fn item_at_bagpos_with_ident(inventory: &Inventory, pos: BagPosition) -> Option<(&Item, ItemCommandIdent)>
{
    //
    item_at_bagpos(inventory, pos).map(|it| (it, it.command_ident()))
}
