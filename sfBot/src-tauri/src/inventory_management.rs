#![allow(warnings)]
use std::{any::type_name, error::Error, fmt::Debug};

use sf_api::{
    command::{Command, ShopType},
    gamestate::{
        character::Class,
        fortress::{Fortress, FortressBuildingType},
        guild::Guild,
        items::{EquipmentSlot, GemSlot, GemType, Inventory, InventoryType, Item, ItemPlace::MainInventory, ItemType, PetItem, PlayerItemPlace, Potion, PotionSize, PotionType},
        rewards::Event,
        unlockables::{HabitatType, Pets},
        GameState,
    },
    SimpleSession,
};

use crate::{fetch_character_setting, lottery::sleep_between_commands};

fn check_type<T: std::fmt::Debug>(x: T)
{
    println!("{:?} is of type {}", x, type_name::<T>());
}

// TODO think about dismantling
pub async fn manage_inventory(session: &mut SimpleSession) -> Result<String, Box<dyn Error>>
{
    let gs = &session.send_command(Command::Update).await?.clone();
    let free_slots = gs.character.inventory.count_free_slots();

    let amount_of_slots_to_keep_free: i32 = std::cmp::min(fetch_character_setting(&gs, "itemsInventorySlotsToBeLeft").unwrap_or(0), 1);
    let sell_items_to_witch: bool = fetch_character_setting(&gs, "itemsImmediatelyThrowIntoCauldron").unwrap_or(false);
    let exclude_epics_from_witch_selling: bool = fetch_character_setting(&gs, "itemsImmediatelyThrowIntoCauldronExceptEpics").unwrap_or(false); // excludes epics
    drink_potions(session).await?;
    sell_potions(session).await?;
    sell_gems(session).await?;
    buy_potions_and_hourglasses(session).await?;
    // check_for_pet_egg(session).await?;

    // witch selling
    if (sell_items_to_witch)
    {
        let mut events = &gs.specials.events.active;
        let witch_event_active = events.contains(&Event::WitchesDance);
        sell_item_to_witch(session, witch_event_active, exclude_epics_from_witch_selling).await?;
    }
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
    let mut msg = std::string::String::from("");

    let magic_shop = &gs.shops[ShopType::Magic];
    let magic_shop_items = &magic_shop.items;
    let free_slot = gs.character.inventory.free_slot();
    let free_slot_index = match free_slot
    {
        Some(slot) => slot.backpack_pos(),
        None => return Ok(msg),
    };

    for (pos, item) in magic_shop_items.iter().enumerate()
    {
        if (potion_mapping_from_string_settings_for_buying(&gs, item))
        {
            if &gs.character.silver > &(item.price as u64) && free_slot.is_some() && item.mushroom_price <= 0
            {
                if (item.typ == ItemType::QuickSandGlass)
                {
                    session.send_command(Command::BuyShopHourglas { shop_type: ShopType::Magic, shop_pos: pos }).await?;
                    msg.push_str("Bought Hourglas from magic shop");
                    return Ok(msg);
                }
                println!("{} {:?}", pos, item);
                println!("{}", free_slot_index);
                if let Err(e) = session
                    .send_command(Command::BuyShop {
                        shop_type: ShopType::Magic,
                        shop_pos: pos,
                        inventory: get_inventory_based_on_index(free_slot_index),
                        inventory_pos: adjust_free_slot_index(free_slot_index),
                    })
                    .await
                {
                    eprintln!("BuyShop failed: {:?}", e);
                    return Err(e.into());
                }
                msg.push_str("Bought Potion from magic shop");
                return Ok(msg);
            }
        }
    }
    return Ok(msg);
}

pub async fn check_whether_hourglas_is_in_inv(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let char_inventory = gs.character.inventory;
    let sorted_items_with_indices = sorted_items_with_indices(&char_inventory);

    for (pos, item) in sorted_items_with_indices
    {
        if item.typ == ItemType::QuickSandGlass
        {
            session.send_command(Command::UsePotion { from: MainInventory, from_pos: pos }).await?;
        }
    }
    return Ok(());
}

pub async fn drink_potions(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let character_pots = &gs.character.active_potions;
    let amount_of_active_potions = character_pots.iter().filter(|p| p.is_some()).count();
    let all_pots_are_none = character_pots.iter().all(|p| p.is_none());
    // println!("here im");

    let char_inventory = &gs.character.inventory.clone();

    let char_class = &gs.character.class;
    let sorted_items_with_indices = sorted_items_with_indices(char_inventory);
    let mut potion_to_drink = Vec::new();

    for (pos, item) in sorted_items_with_indices
    {
        // check if the item is a potion and should not be sold
        if let ItemType::Potion(potion) = &item.typ
        {
            if should_drink_potion_from_settings(&gs, potion.typ, potion.size)
            {
                potion_to_drink.push((pos, MainInventory, item.typ.clone()));
            }
        }
    }

    if !potion_to_drink.is_empty()
    {
        if (amount_of_active_potions < 3 || all_pots_are_none)
        {
            for (pos, inventory_type, item_type) in potion_to_drink
            {
                let drink_potion_command = Command::UsePotion { from: MainInventory, from_pos: pos };
                if let Err(err) = session.send_command(drink_potion_command).await
                {
                    eprintln!("Error: func drink_potions while executing UsePotion command: {}", err);
                    return Ok(());
                }
            }
        }
        else
        {
            for (pos, inventory_type, item_type) in &potion_to_drink
            {
                if let ItemType::Potion(potion) = item_type
                {
                    for pot in character_pots.iter()
                    {
                        if let Some(p) = pot
                        {
                            if p.typ == potion.typ && p.size == potion.size
                            {
                                let drink_potion_command = Command::UsePotion { from: MainInventory, from_pos: *pos };
                                session.send_command(drink_potion_command).await?;
                            }
                        }
                    }
                }
            }
        }
        println!("drank pot");
    }
    return Ok(());
}

pub async fn check_for_pet_egg(session: &mut SimpleSession) -> Result<std::string::String, Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let mut msg = std::string::String::from("");

    let weapon_shop = &gs.shops[ShopType::Weapon];
    let magic_shop_items = &weapon_shop.items;
    let free_slot = gs.character.inventory.free_slot();
    let free_slot_unpacked = match free_slot
    {
        Some(slot) => slot,
        None => return Ok(msg),
    };
    for (pos, item) in magic_shop_items.iter().enumerate()
    {
        if (potion_mapping_from_string_settings_for_buying(&gs, item))
        {
            if &gs.character.silver > &(item.price as u64) && free_slot.is_some() && item.mushroom_price <= 0
            {
                if (item.typ == ItemType::PetItem { typ: PetItem::Egg(HabitatType::Water) })
                {
                    session.send_command(Command::BuyShopHourglas { shop_type: ShopType::Weapon, shop_pos: pos }).await?;
                    msg.push_str("Bought pet egg from weapon shop.");
                    return Ok(msg);
                }

                if (item.typ == ItemType::PetItem { typ: PetItem::GoldenEgg })
                {
                    session.send_command(Command::BuyShopHourglas { shop_type: ShopType::Weapon, shop_pos: pos }).await?;
                    msg.push_str("Bought pet egg from weapon shop.");
                    return Ok(msg);
                }

                if (item.typ == ItemType::PetItem { typ: PetItem::SpecialEgg(HabitatType::Water) })
                {
                    session.send_command(Command::BuyShopHourglas { shop_type: ShopType::Weapon, shop_pos: pos }).await?;
                    msg.push_str("Bought pet egg from weapon shop.");
                    return Ok(msg);
                }
            }
        }
    }
    return Ok(msg);
}

pub async fn sell_potions(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let char_inventory = &gs.character.inventory.clone();
    let sorted_items_with_indices = sorted_items_with_indices(char_inventory);

    let mut potions_to_sell = Vec::new();

    for (pos, item) in sorted_items_with_indices
    {
        // check if the item is a potion and should not be sold
        if let ItemType::Potion(potion) = &item.typ
        {
            if should_sell_potion_from_settings(&gs, potion.typ, potion.size)
            {
                potions_to_sell.push((pos, MainInventory, item.typ.clone()));
            }
        }
    }

    for (pos, inventory_type, item_type) in potions_to_sell
    {
        let sell_command = Command::SellShop {
            inventory: PlayerItemPlace::MainInventory,
            inventory_pos: pos,
        };
        session.send_command(sell_command).await?;
    }

    return Ok(());
}

pub async fn sell_gems(session: &mut SimpleSession) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?.clone();
    let char_inventory = &gs.character.inventory.clone();

    let sorted_items_with_indices = sorted_items_with_indices(char_inventory);
    let mut gems_to_sell = Vec::new();

    for (pos, item) in sorted_items_with_indices
    {
        // check if the item is a gem and should be sold
        if let ItemType::Gem(gem) = &item.typ
        {
            if should_sell_gem(&gs, gem.typ, gem.value)
            {
                gems_to_sell.push((pos, MainInventory, item.typ.clone()));
            }
        }
    }

    for (pos, inventory_type, item_type) in gems_to_sell
    {
        let sell_command = Command::SellShop {
            inventory: PlayerItemPlace::MainInventory,
            inventory_pos: pos,
        };
        session.send_command(sell_command).await?;
    }

    return Ok(());
}

pub async fn sell_item_to_witch(session: &mut SimpleSession, witch_event_active: bool, exclude_epics_from_witch_selling: bool) -> Result<(), Box<dyn Error>>
{
    let gs = session.send_command(Command::Update).await?;
    if gs.character.level < 66
    {
        return Ok(());
    }

    loop
    {
        let gs = session.send_command(Command::Update).await?;
        let inventory = &gs.character.inventory;

        let the_witch = match &gs.witch
        {
            Some(witch) => witch,
            None =>
            {
                return Ok(());
            }
        };

        if witch_event_active
        {
            if (exclude_epics_from_witch_selling)
            {
                // doesnt sell epics nor legys
                let items_to_sell = collect_items_to_sell_no_epics_no_legendary(inventory);
                for (pos) in items_to_sell
                {
                    let drop_command = Command::WitchDropCauldron {
                        inventory_t: PlayerItemPlace::MainInventory,
                        position: pos,
                    };

                    session.send_command(drop_command).await?;
                }
                return Ok(());
            }
            else
            {
                // sells epics and legys as well
                let items_to_sell = collect_items_to_sell_including_epics_and_legendaries(inventory);
                for (pos) in items_to_sell
                {
                    let drop_command = Command::WitchDropCauldron {
                        inventory_t: PlayerItemPlace::MainInventory,
                        position: pos,
                    };
                    session.send_command(drop_command).await?;
                }
                return Ok(());
            }
        }
        if let Some(required_slot) = the_witch.required_item
        {
            if let Some((pos, item)) = find_required_item_for_witch(inventory, required_slot, exclude_epics_from_witch_selling)
            {
                let drop_command = Command::WitchDropCauldron {
                    inventory_t: PlayerItemPlace::MainInventory,
                    position: pos,
                };

                session.send_command(drop_command).await?;
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

fn collect_items_to_sell_including_epics_and_legendaries(inventory: &Inventory) -> Vec<usize>
{
    inventory
        .backpack
        .iter()
        .enumerate()
        .filter_map(|(pos, item_option)| item_option.as_ref().filter(|item| items_to_sell_or_should_sell_to_witch_includes_epics_and_legendaries(item)).map(|_| pos))
        .collect()
}

fn collect_items_to_sell_no_epics_no_legendary(inventory: &Inventory) -> Vec<usize>
{
    inventory
        .backpack
        .iter()
        .enumerate()
        .filter_map(|(pos, item_option)| item_option.as_ref().filter(|item| items_to_sell_or_should_sell_to_witch_no_epic_no_legendarys(item)).map(|_| pos))
        .collect()
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
    let character_inventory = &gs.character.inventory;
    let sorted_items_with_indices = sorted_items_with_indices(character_inventory);

    let mut items_to_sell = Vec::new();

    for (pos, item) in sorted_items_with_indices
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

        items_to_sell.push((pos, MainInventory));

        if items_to_sell.len() == 2
        {
            break;
        }
    }

    for (pos, _inventory_type) in items_to_sell
    {
        let sell_command = Command::SellShop {
            inventory: PlayerItemPlace::MainInventory,
            inventory_pos: pos,
        };
        session.send_command(sell_command).await?;
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

pub fn sorted_items_with_indices(inventory: &Inventory) -> Vec<(usize, &Item)>
{
    let mut items_with_indices: Vec<_> = inventory.backpack.iter().enumerate().filter_map(|(pos, item_option)| item_option.as_ref().map(|item| (pos, item))).collect();

    items_with_indices.sort_by_key(|(_, item)| item.price);

    items_with_indices
}

pub fn find_required_item_for_witch<'a>(inventory: &'a Inventory, required_slot: EquipmentSlot, exclude_epics_from_witch_selling: bool) -> Option<(usize, &'a Item)>
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

    inventory.backpack.iter().enumerate().filter_map(|(pos, item_option)| item_option.as_ref().map(|item| (pos, item))).find(|(_, item)| {
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
    let free_slot = gs.character.inventory.free_slot();
    if free_slot.is_none()
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

        if habitat.1.fruits > 10
        {
            session
                .send_command(Command::Custom {
                    cmd_name: "PlayerWitchBrewPotion".to_string(),
                    arguments: vec![index.to_string()],
                })
                .await?;

            brewed_any = true;

            if let Some(bag_pos) = gs.character.inventory.free_slot()
            {
                println!("{:?}", bag_pos);
                if let Err(e) = session
                    .send_command(Command::SellShop {
                        inventory: PlayerItemPlace::MainInventory,
                        inventory_pos: bag_pos.backpack_pos(),
                    })
                    .await
                {
                    eprintln!("[ERROR] SellShop failed at pos {}: {}", bag_pos.backpack_pos(), e);
                    return Err(e.into());
                }
            }

            manage_inventory(session).await?;
        }
    }

    if brewed_any
    {
        Ok("brewed and sold potion".to_string())
    }
    else
    {
        Ok("".to_string())
    }
}

fn adjust_free_slot_index(index: usize) -> usize
{
    if index >= 5
    {
        return index - 5;
    }
    return index;
}

fn get_inventory_based_on_index(index: usize) -> PlayerItemPlace
{
    if index >= 5
    {
        return PlayerItemPlace::ExtendedInventory;
    }
    return PlayerItemPlace::MainInventory;
}
