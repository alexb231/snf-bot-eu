use std::cmp::Ordering;

use chrono::{DateTime, Local};
use enum_map::{Enum, EnumMap};
use log::warn;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum::{EnumCount, EnumIter};

use super::{
    CFPGet, Class, EnumMapGet, HabitatType, SFError, ServerTime,
    unlockables::EquipmentIdent,
};
use crate::{
    command::{AttributeType, ShopType},
    gamestate::{CCGet, CGet, ShopPosition},
};


#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inventory {
    pub backpack: Vec<Option<Item>>,
}


#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BagPosition(pub(crate) usize);

impl BagPosition {
    
    #[must_use]
    pub fn backpack_pos(&self) -> usize {
        self.0
    }
    
    
    
    #[must_use]
    pub fn inventory_pos(&self) -> (InventoryType, usize) {
        let pos = self.0;
        if pos <= 4 {
            (InventoryType::MainInventory, pos)
        } else {
            (InventoryType::ExtendedInventory, pos - 5)
        }
    }
}

impl Inventory {
    
    
    
    
    #[must_use]
    pub fn as_split(&self) -> (&[Option<Item>], &[Option<Item>]) {
        if self.backpack.len() < 5 {
            return (&[], &[]);
        }
        self.backpack.split_at(5)
    }

    
    
    
    
    #[must_use]
    pub fn as_split_mut(
        &mut self,
    ) -> (&mut [Option<Item>], &mut [Option<Item>]) {
        if self.backpack.len() < 5 {
            return (&mut [], &mut []);
        }
        self.backpack.split_at_mut(5)
    }

    
    
    
    #[must_use]
    pub fn free_slot(&self) -> Option<BagPosition> {
        for (pos, item) in self.iter() {
            if item.is_none() {
                return Some(pos);
            }
        }
        None
    }

    #[must_use]
    pub fn count_free_slots(&self) -> usize {
        self.backpack.iter().filter(|slot| slot.is_none()).count()
    }

    
    pub fn iter(&self) -> impl Iterator<Item = (BagPosition, Option<&Item>)> {
        self.backpack
            .iter()
            .enumerate()
            .map(|(pos, item)| (BagPosition(pos), item.as_ref()))
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum PlayerItemPlace {
    Equipment = 1,
    MainInventory = 2,
    ExtendedInventory = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemPosition {
    pub place: ItemPlace,
    pub position: usize,
}

impl std::fmt::Display for ItemPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.place as usize, self.position + 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerItemPosition {
    pub place: PlayerItemPlace,
    pub position: usize,
}

impl std::fmt::Display for PlayerItemPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.place as usize, self.position + 1)
    }
}

impl From<PlayerItemPosition> for ItemPosition {
    fn from(value: PlayerItemPosition) -> Self {
        Self {
            place: value.place.item_position(),
            position: value.position,
        }
    }
}

impl From<BagPosition> for ItemPosition {
    fn from(value: BagPosition) -> Self {
        let player: PlayerItemPosition = value.into();
        player.into()
    }
}

impl From<EquipmentSlot> for ItemPosition {
    fn from(value: EquipmentSlot) -> Self {
        let player: PlayerItemPosition = value.into();
        player.into()
    }
}

impl From<ShopPosition> for ItemPosition {
    fn from(value: ShopPosition) -> Self {
        Self {
            place: value.typ.into(),
            position: value.pos,
        }
    }
}

impl From<ShopType> for ItemPlace {
    fn from(value: ShopType) -> Self {
        match value {
            ShopType::Weapon => ItemPlace::WeaponShop,
            ShopType::Magic => ItemPlace::MageShop,
        }
    }
}

impl From<BagPosition> for PlayerItemPosition {
    fn from(value: BagPosition) -> Self {
        let p = value.inventory_pos();
        Self {
            place: p.0.player_item_position(),
            position: p.1,
        }
    }
}

impl From<EquipmentSlot> for PlayerItemPosition {
    fn from(value: EquipmentSlot) -> Self {
        Self {
            place: PlayerItemPlace::Equipment,
            position: value as usize - 1,
        }
    }
}

impl PlayerItemPlace {
    
    
    #[must_use]
    pub fn item_position(&self) -> ItemPlace {
        match self {
            PlayerItemPlace::Equipment => ItemPlace::Equipment,
            PlayerItemPlace::MainInventory => ItemPlace::MainInventory,
            PlayerItemPlace::ExtendedInventory => ItemPlace::FortressChest,
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum InventoryType {
    MainInventory = 2,
    ExtendedInventory = 5,
}

impl InventoryType {
    
    
    #[must_use]
    pub fn item_position(&self) -> ItemPlace {
        match self {
            InventoryType::MainInventory => ItemPlace::MainInventory,
            InventoryType::ExtendedInventory => ItemPlace::FortressChest,
        }
    }
    
    
    #[must_use]
    pub fn player_item_position(&self) -> PlayerItemPlace {
        match self {
            InventoryType::MainInventory => PlayerItemPlace::MainInventory,
            InventoryType::ExtendedInventory => {
                PlayerItemPlace::ExtendedInventory
            }
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ItemPlace {
    
    Equipment = 1,
    
    MainInventory = 2,
    
    WeaponShop = 3,
    
    MageShop = 4,
    
    FortressChest = 5,
}


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Equipment(pub EnumMap<EquipmentSlot, Option<Item>>);

impl Equipment {
    
    #[must_use]
    pub fn has_enchantment(&self, enchantment: Enchantment) -> bool {
        let item = self.0.get(enchantment.equipment_slot());
        if let Some(item) = item {
            return item.enchantment == Some(enchantment);
        }
        false
    }

    
    #[allow(clippy::indexing_slicing)]
    pub(crate) fn parse(
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<Equipment, SFError> {
        let mut res = Equipment::default();
        if !data.len().is_multiple_of(ITEM_PARSE_LEN) {
            return Err(SFError::ParsingError(
                "Invalid Equipment",
                format!("{data:?}"),
            ));
        }
        for (chunk, slot) in
            data.chunks_exact(ITEM_PARSE_LEN).zip(res.0.as_mut_slice())
        {
            *slot = Item::parse(chunk, server_time)?;
        }
        Ok(res)
    }
}

pub(crate) const ITEM_PARSE_LEN: usize = 19;



#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Item {
    
    pub typ: ItemType,
    
    pub price: u32,
    
    
    
    pub mushroom_price: u32,
    
    
    
    pub full_model_id: u32,
    
    pub model_id: u16,
    
    
    pub class: Option<Class>,
    
    
    
    pub type_specific_val: u32,
    
    pub attributes: EnumMap<AttributeType, u32>,
    
    pub gem_slot: Option<GemSlot>,
    
    pub rune: Option<Rune>,
    
    pub enchantment: Option<Enchantment>,
    
    
    pub color: u8,
    
    pub upgrade_count: u8,
    
    pub item_quality: u32,
    
    pub is_washed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemCommandIdent {
    typ: u8,
    full_model_id: u32,
    price: u32,
    mush_price: u32,
}

impl std::fmt::Display for ItemCommandIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}/{}",
            self.typ, self.full_model_id, self.price, self.mush_price
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlacksmithPayment {
    pub metal: u64,
    pub arcane: u64,
}

impl Item {
    
    
    
    
    
    
    #[must_use]
    pub fn dismantle_reward(&self) -> BlacksmithPayment {
        let mut attribute_val =
            f64::from(*self.attributes.values().max().unwrap_or(&0));
        let item_stats = self.attributes.values().filter(|a| **a > 0).count();
        let is_scout_or_mage_weapon = self
            .class
            .is_some_and(|a| a == Class::Scout || a == Class::Mage)
            && self.typ.is_weapon();

        if self.price != 0 {
            for _ in 0..self.upgrade_count {
                attribute_val = (attribute_val / 1.04).round();
            }
        }

        if item_stats >= 4 {
            attribute_val *= 1.2;
        }
        if is_scout_or_mage_weapon {
            attribute_val /= 2.0;
        }
        
        if (item_stats == 1) && attribute_val > 66.0 {
            attribute_val = attribute_val.round() * 0.75;
        }

        attribute_val = attribute_val.round().powf(1.2).floor();

        let (min_dmg, max_dmg) = match self.typ {
            ItemType::Weapon { min_dmg, max_dmg } => (min_dmg, max_dmg),
            _ => (0, 0),
        };

        let price = (u32::from(self.typ.raw_id()) * 37)
            + (self.full_model_id * 83)
            + (min_dmg * 1731)
            + (max_dmg * 162);

        let (metal_price, arcane_price) = match item_stats {
            1 => (75 + (price % 26), price % 2),
            2 => (50 + (price % 31), 5 + (price % 6)),
            
            _ => (25 + (price % 26), 50 + (price % 51)),
        };

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let calc_result = |rng: u32| {
            ((attribute_val * f64::from(rng)) / 100.0).floor() as u64
        };
        let mut metal_result = calc_result(metal_price);
        let mut arcane_result = calc_result(arcane_price);

        if is_scout_or_mage_weapon {
            metal_result *= 2;
            arcane_result *= 2;
        }
        BlacksmithPayment {
            metal: metal_result * 2,
            arcane: arcane_result * 2,
        }
    }

    
    
    
    
    
    
    
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn upgrade_costs(&self) -> Option<BlacksmithPayment> {
        if self.upgrade_count >= 20 || self.equipment_ident().is_none() {
            return None;
        }

        let item_stats = self.attributes.values().filter(|a| **a > 0).count();
        let is_scout_or_mage_weapon = self
            .class
            .is_some_and(|a| a == Class::Scout || a == Class::Mage)
            && self.typ.is_weapon();

        
        let mut price =
            f64::from(*self.attributes.values().max().unwrap_or(&0));

        
        if item_stats >= 4 {
            price *= 1.2;
        }

        if is_scout_or_mage_weapon {
            price /= 2.0;
        }

        
        if item_stats == 1 && price > 66.0 {
            price = (price * 0.75).ceil();
        }

        price = price.round().powf(1.2).floor();

        let mut metal_price = 50;
        let mut arcane_price = match item_stats {
            1 => 25,
            2 => 50,
            
            _ => 75,
        };

        let i = i64::from(self.upgrade_count);
        match i {
            0 => {
                metal_price *= 3;
                arcane_price = 0;
            }
            1 => {
                metal_price *= 4;
                arcane_price = 1;
            }
            2..=7 => {
                metal_price *= i + 3;
                arcane_price *= i - 1;
            }
            8 => {
                metal_price *= 12;
                arcane_price *= 8;
            }
            9 => {
                metal_price *= 15;
                arcane_price *= 10;
            }
            _ => {
                metal_price *= i + 6;
                arcane_price *= 10 + 2 * (i - 9);
            }
        }

        metal_price = ((price * (metal_price as f64)) / 100.0).floor() as i64;
        arcane_price = ((price * (arcane_price as f64)) / 100.0).floor() as i64;

        if is_scout_or_mage_weapon {
            metal_price *= 2;
            arcane_price *= 2;
        }

        Some(BlacksmithPayment {
            metal: metal_price.try_into().unwrap_or(0),
            arcane: arcane_price.try_into().unwrap_or(0),
        })
    }

    
    
    #[must_use]
    pub fn equipment_ident(&self) -> Option<EquipmentIdent> {
        Some(EquipmentIdent {
            class: self.class,
            typ: self.typ.equipment_slot()?,
            model_id: self.model_id,
            color: self.color,
        })
    }

    
    
    
    #[must_use]
    pub fn command_ident(&self) -> ItemCommandIdent {
        ItemCommandIdent {
            typ: self.typ.raw_id(),
            full_model_id: self.full_model_id,
            price: self.price,
            mush_price: self.mushroom_price,
        }
    }

    
    
    #[must_use]
    pub fn is_unique(&self) -> bool {
        self.typ.is_unique()
    }

    
    #[must_use]
    pub fn is_epic(&self) -> bool {
        self.model_id >= 50
    }

    
    #[must_use]
    pub fn is_legendary(&self) -> bool {
        self.model_id >= 90
    }

    
    #[must_use]
    pub fn armor(&self) -> u32 {
        #[allow(clippy::enum_glob_use)]
        use ItemType::*;
        match self.typ {
            Hat | BreastPlate | Gloves | FootWear | Amulet | Belt | Ring
            | Talisman => self.type_specific_val,
            _ => 0,
        }
    }

    
    #[must_use]
    pub fn is_enchantable(&self) -> bool {
        self.typ.is_enchantable()
    }

    
    
    
    
    #[must_use]
    pub fn can_be_equipped_by_companion(
        &self,
        class: impl Into<Class>,
    ) -> bool {
        !self.typ.is_shield() && self.can_be_equipped_by(class.into())
    }

    
    
    
    
    
    
    
    #[must_use]
    pub fn can_be_equipped_by(&self, class: Class) -> bool {
        self.typ.equipment_slot().is_some() && self.can_be_used_by(class)
    }

    
    
    
    
    
    #[must_use]
    #[allow(clippy::enum_glob_use, clippy::match_same_arms)]
    pub fn can_be_used_by(&self, class: Class) -> bool {
        use Class::*;

        
        let Some(class_requirement) = self.class else {
            return true;
        };

        match class {
            Warrior | Paladin => class_requirement == Warrior,
            Berserker => class_requirement == Warrior && !self.typ.is_shield(),
            Scout => class_requirement == Scout,
            Mage | Necromancer => class_requirement == Mage,
            Assassin => match class_requirement {
                Warrior => self.typ.is_weapon(),
                Scout => !self.typ.is_weapon(),
                _ => false,
            },
            Bard | Druid => match class_requirement {
                Mage => self.typ.is_weapon(),
                Scout => !self.typ.is_weapon(),
                _ => false,
            },
            BattleMage | PlagueDoctor => match class_requirement {
                Warrior => self.typ.is_weapon(),
                Mage => !self.typ.is_weapon(),
                _ => false,
            },
            DemonHunter => match class_requirement {
                Scout => self.typ.is_weapon(),
                Warrior => !self.typ.is_weapon() && !self.typ.is_shield(),
                _ => false,
            },
        }
    }

    
    pub(crate) fn parse(
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<Option<Self>, SFError> {
        let Some(typ) = ItemType::parse(data, server_time)? else {
            return Ok(None);
        };

        let enchantment = data.cfpget(2, "item enchantment", |a| a)?;
        let gem_slot_val = data.cimget(1, "gem slot val", |a| a)?;
        let gem_pwr = data.cimget(16, "gem pwr", |a| a)?;

        let gem_slot = GemSlot::parse(gem_slot_val, gem_pwr);

        let class = if typ.is_class_item() {
            data.cfpget(3, "item class", |x| (x & 0xFFFF) / 1000)?
        } else {
            None
        };
        let mut rune = None;
        let mut attributes: EnumMap<AttributeType, u32> = EnumMap::default();
        let price = data.csiget(13, "item price", u32::MAX)?;

        if typ.equipment_slot().is_some() {
            for i in 0..3 {
                let atr_typ = data.cget(i + 7, "item atr typ")?;
                let Ok(atr_typ) = atr_typ.try_into() else {
                    warn!("Invalid attribute typ: {atr_typ}, {typ:?}");
                    continue;
                };
                let atr_val = data.cget(i + 10, "item atr val")?;
                let Ok(atr_val): Result<u32, _> = atr_val.try_into() else {
                    warn!("Invalid attribute value: {atr_val}, {typ:?}");
                    continue;
                };
                match atr_typ {
                    0 => {}
                    1..=5 => {
                        let Some(atr_typ) = FromPrimitive::from_usize(atr_typ)
                        else {
                            continue;
                        };
                        *attributes.get_mut(atr_typ) += atr_val;
                    }
                    6 => {
                        for atr in attributes.values_mut() {
                            *atr += atr_val;
                        }
                    }
                    21 => {
                        for atr in [
                            AttributeType::Strength,
                            AttributeType::Constitution,
                            AttributeType::Luck,
                        ] {
                            *attributes.get_mut(atr) += atr_val;
                        }
                    }
                    22 => {
                        for atr in [
                            AttributeType::Dexterity,
                            AttributeType::Constitution,
                            AttributeType::Luck,
                        ] {
                            *attributes.get_mut(atr) += atr_val;
                        }
                    }
                    23 => {
                        for atr in [
                            AttributeType::Intelligence,
                            AttributeType::Constitution,
                            AttributeType::Luck,
                        ] {
                            *attributes.get_mut(atr) += atr_val;
                        }
                    }
                    rune_typ => {
                        let Some(typ) = FromPrimitive::from_usize(rune_typ)
                        else {
                            warn!(
                                "Unhandled item val: {atr_typ} -> {atr_val} \
                                 for {class:?} {typ:?}",
                            );
                            continue;
                        };
                        let Ok(value) = atr_val.try_into() else {
                            warn!("Rune value too big for a u8: {atr_val}");
                            continue;
                        };
                        rune = Some(Rune { typ, value });
                    }
                }
            }
        }
        let model_id: u16 =
            data.cimget(3, "item model id", |x| (x & 0xFFFF) % 1000)?;

        let color = match model_id {
            ..=49 if typ != ItemType::Talisman => data
                .get(5..=12)
                .map(|a| a.iter().sum::<i64>())
                .map(|a| (a % 5) + 1)
                .and_then(|a| a.try_into().ok())
                .unwrap_or(1),
            _ => 1,
        };

        let item = Item {
            typ,
            model_id,
            rune,
            type_specific_val: data.csiget(5, "effect value", 0)?,
            gem_slot,
            enchantment,
            class,
            attributes,
            color,
            price,
            mushroom_price: data.csiget(14, "mushroom price", u32::MAX)?,
            upgrade_count: data.csiget(15, "upgrade count", u8::MAX)?,
            item_quality: data.csiget(17, "item quality", 0)?,
            is_washed: data.csiget(18, "is washed", 0)? != 0,
            full_model_id: data.csiget(3, "raw model id", 0)?,
        };
        Ok(Some(item))
    }
}


#[derive(
    Debug, Clone, Copy, FromPrimitive, PartialEq, Eq, EnumIter, Hash, Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Enchantment {
    
    SwordOfVengeance = 11,
    
    MariosBeard = 31,
    
    ManyFeetBoots = 41,
    
    ShadowOfTheCowboy = 51,
    
    AdventurersArchaeologicalAura = 61,
    
    ThirstyWanderer = 71,
    
    UnholyAcquisitiveness = 81,
    
    TheGraveRobbersPrayer = 91,
    
    RobberBaronRitual = 101,
}

impl Enchantment {
    #[must_use]
    pub fn equipment_slot(&self) -> EquipmentSlot {
        match self {
            Enchantment::SwordOfVengeance => EquipmentSlot::Weapon,
            Enchantment::MariosBeard => EquipmentSlot::BreastPlate,
            Enchantment::ManyFeetBoots => EquipmentSlot::FootWear,
            Enchantment::ShadowOfTheCowboy => EquipmentSlot::Gloves,
            Enchantment::AdventurersArchaeologicalAura => EquipmentSlot::Hat,
            Enchantment::ThirstyWanderer => EquipmentSlot::Belt,
            Enchantment::UnholyAcquisitiveness => EquipmentSlot::Amulet,
            Enchantment::TheGraveRobbersPrayer => EquipmentSlot::Ring,
            Enchantment::RobberBaronRitual => EquipmentSlot::Talisman,
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rune {
    
    pub typ: RuneType,
    
    
    pub value: u8,
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq, EnumIter, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]

pub enum RuneType {
    QuestGold = 31,
    EpicChance,
    ItemQuality,
    QuestXP,
    ExtraHitPoints,
    FireResistance,
    ColdResistence,
    LightningResistance,
    TotalResistence,
    FireDamage,
    ColdDamage,
    LightningDamage,
}


#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GemSlot {
    
    Filled(Gem),
    
    Empty,
}

impl GemSlot {
    pub(crate) fn parse(slot_val: i64, gem_pwr: i64) -> Option<GemSlot> {
        match slot_val {
            0 => return None,
            1 => return Some(GemSlot::Empty),
            _ => {}
        }

        let Ok(value) = gem_pwr.try_into() else {
            warn!("Invalid gem power {gem_pwr}");
            return None;
        };

        match GemType::parse(slot_val, value) {
            Some(typ) => Some(GemSlot::Filled(Gem { typ, value })),
            None => Some(GemSlot::Empty),
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Potion {
    
    pub typ: PotionType,
    
    pub size: PotionSize,
    
    
    pub expires: Option<DateTime<Local>>,
}




#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum ItemType {
    Hat,
    BreastPlate,
    Gloves,
    FootWear,
    Weapon {
        min_dmg: u32,
        max_dmg: u32,
    },
    Amulet,
    Belt,
    Ring,
    Talisman,
    Shield {
        block_chance: u32,
    },
    Shard {
        piece: u32,
    },
    Potion(Potion),
    Scrapbook,
    DungeonKey {
        id: u32,
        shadow_key: bool,
    },
    Gem(Gem),
    PetItem {
        typ: PetItem,
    },
    QuickSandGlass,
    HeartOfDarkness,
    WheelOfFortune,
    Mannequin,
    Resource {
        amount: u32,
        typ: ResourceType,
    },
    ToiletKey,
    Gral,
    EpicItemBag,
    
    
    Unknown(u8),
}

impl ItemType {
    
    #[must_use]
    pub const fn is_weapon(self) -> bool {
        matches!(self, ItemType::Weapon { .. })
    }

    
    #[must_use]
    pub const fn is_shield(self) -> bool {
        matches!(self, ItemType::Shield { .. })
    }

    
    #[must_use]
    pub fn is_class_item(&self) -> bool {
        matches!(
            self,
            ItemType::Hat
                | ItemType::Belt
                | ItemType::Gloves
                | ItemType::FootWear
                | ItemType::Shield { .. }
                | ItemType::Weapon { .. }
                | ItemType::BreastPlate
        )
    }

    
    
    
    #[must_use]
    pub fn is_unique(&self) -> bool {
        matches!(
            self,
            ItemType::Scrapbook
                | ItemType::HeartOfDarkness
                | ItemType::WheelOfFortune
                | ItemType::Mannequin
                | ItemType::ToiletKey
                | ItemType::Gral
                | ItemType::EpicItemBag
                | ItemType::DungeonKey { .. }
        )
    }

    
    #[must_use]
    pub fn equipment_slot(&self) -> Option<EquipmentSlot> {
        Some(match self {
            ItemType::Hat => EquipmentSlot::Hat,
            ItemType::BreastPlate => EquipmentSlot::BreastPlate,
            ItemType::Gloves => EquipmentSlot::Gloves,
            ItemType::FootWear => EquipmentSlot::FootWear,
            ItemType::Weapon { .. } => EquipmentSlot::Weapon,
            ItemType::Amulet => EquipmentSlot::Amulet,
            ItemType::Belt => EquipmentSlot::Belt,
            ItemType::Ring => EquipmentSlot::Ring,
            ItemType::Talisman => EquipmentSlot::Talisman,
            ItemType::Shield { .. } => EquipmentSlot::Shield,
            _ => return None,
        })
    }

    
    #[must_use]
    pub fn is_enchantable(&self) -> bool {
        self.equipment_slot()
            .is_some_and(|e| e.enchantment().is_some())
    }

    pub(crate) fn parse(
        data: &[i64],
        _server_time: ServerTime,
    ) -> Result<Option<Self>, SFError> {
        let raw_typ: u8 = data.csimget(0, "item type", 255, |a| a & 0xFF)?;
        let unknown_item = |name: &'static str| {
            warn!("Could no parse item of type: {raw_typ}. {name} is faulty");
            Ok(Some(ItemType::Unknown(raw_typ)))
        };

        let sub_ident = data.cget(3, "item sub type")?;

        Ok(Some(match raw_typ {
            0 => return Ok(None),
            1 => ItemType::Weapon {
                min_dmg: data.csiget(5, "weapon min dmg", 0)?,
                max_dmg: data.csiget(6, "weapon min dmg", 0)?,
            },
            2 => ItemType::Shield {
                block_chance: data.csiget(5, "shield block chance", 0)?,
            },
            3 => ItemType::BreastPlate,
            4 => ItemType::FootWear,
            5 => ItemType::Gloves,
            6 => ItemType::Hat,
            7 => ItemType::Belt,
            8 => ItemType::Amulet,
            9 => ItemType::Ring,
            10 => ItemType::Talisman,
            11 => {
                let id = sub_ident & 0xFFFF;
                let Ok(id) = id.try_into() else {
                    return unknown_item("unique sub ident");
                };
                match id {
                    1..=11 | 17 | 19 | 22 | 69 | 70 => ItemType::DungeonKey {
                        id,
                        shadow_key: false,
                    },
                    20 => ItemType::ToiletKey,
                    51..=64 | 67..=68 => ItemType::DungeonKey {
                        id,
                        shadow_key: true,
                    },
                    10000 => ItemType::EpicItemBag,
                    piece => ItemType::Shard { piece },
                }
            }
            12 => {
                let id = sub_ident & 0xFF;
                if id > 16 {
                    let Some(typ) = FromPrimitive::from_i64(id) else {
                        return unknown_item("resource type");
                    };
                    ItemType::Resource {
                        
                        
                        amount: 0,
                        typ,
                    }
                } else {
                    let Some(typ) = PotionType::parse(id) else {
                        return unknown_item("potion type");
                    };
                    let Some(size) = PotionSize::parse(id) else {
                        return unknown_item("potion size");
                    };
                    ItemType::Potion(Potion {
                        typ,
                        size,
                        
                        expires: None,
                        
                        
                        
                        
                        
                    })
                }
            }
            13 => ItemType::Scrapbook,
            15 => {
                let gem_value = data.csiget(16, "gem pwr", 0)?;
                let Some(typ) = GemType::parse(sub_ident, gem_value) else {
                    return unknown_item("gem type");
                };
                let gem = Gem {
                    typ,
                    value: gem_value,
                };
                ItemType::Gem(gem)
            }
            16 => {
                let Some(typ) = PetItem::parse(sub_ident & 0xFFFF) else {
                    return unknown_item("pet item");
                };
                ItemType::PetItem { typ }
            }
            17 if (sub_ident & 0xFFFF) == 4 => ItemType::Gral,
            17 => ItemType::QuickSandGlass,
            18 => ItemType::HeartOfDarkness,
            19 => ItemType::WheelOfFortune,
            20 => ItemType::Mannequin,
            _ => {
                return unknown_item("main ident");
            }
        }))
    }

    
    
    #[must_use]
    pub fn raw_id(&self) -> u8 {
        match self {
            ItemType::Weapon { .. } => 1,
            ItemType::Shield { .. } => 2,
            ItemType::BreastPlate => 3,
            ItemType::FootWear => 4,
            ItemType::Gloves => 5,
            ItemType::Hat => 6,
            ItemType::Belt => 7,
            ItemType::Amulet => 8,
            ItemType::Ring => 9,
            ItemType::Talisman => 10,
            ItemType::Shard { .. }
            | ItemType::DungeonKey { .. }
            | ItemType::ToiletKey
            | ItemType::EpicItemBag => 11,
            ItemType::Potion { .. } | ItemType::Resource { .. } => 12,
            ItemType::Scrapbook => 13,
            ItemType::Gem(_) => 15,
            ItemType::PetItem { .. } => 16,
            ItemType::QuickSandGlass | ItemType::Gral => 17,
            ItemType::HeartOfDarkness => 18,
            ItemType::WheelOfFortune => 19,
            ItemType::Mannequin => 20,
            ItemType::Unknown(u) => *u,
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum PotionType {
    Strength,
    Dexterity,
    Intelligence,
    Constitution,
    Luck,
    EternalLife,
}

impl From<AttributeType> for PotionType {
    fn from(value: AttributeType) -> Self {
        match value {
            AttributeType::Strength => PotionType::Strength,
            AttributeType::Dexterity => PotionType::Dexterity,
            AttributeType::Intelligence => PotionType::Intelligence,
            AttributeType::Constitution => PotionType::Constitution,
            AttributeType::Luck => PotionType::Luck,
        }
    }
}

impl PotionType {
    pub(crate) fn parse(id: i64) -> Option<PotionType> {
        if id == 0 {
            return None;
        }
        if id == 16 {
            return Some(PotionType::EternalLife);
        }
        Some(match id % 5 {
            0 => PotionType::Luck,
            1 => PotionType::Strength,
            2 => PotionType::Dexterity,
            3 => PotionType::Intelligence,
            _ => PotionType::Constitution,
        })
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum PotionSize {
    Small,
    Medium,
    Large,
}

impl PartialOrd for PotionSize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.effect().partial_cmp(&other.effect())
    }
}

impl PotionSize {
    #[must_use]
    pub fn effect(&self) -> f64 {
        match self {
            PotionSize::Small => 0.1,
            PotionSize::Medium => 0.15,
            PotionSize::Large => 0.25,
        }
    }

    pub(crate) fn parse(id: i64) -> Option<Self> {
        Some(match id {
            1..=5 => PotionSize::Small,
            6..=10 => PotionSize::Medium,
            11..=16 => PotionSize::Large,
            _ => return None,
        })
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Copy, FromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum ResourceType {
    Wood = 17,
    Stone,
    Souls,
    Arcane,
    Metal,
}


#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Gem {
    
    pub typ: GemType,
    
    pub value: u32,
}


#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum GemType {
    Strength,
    Dexterity,
    Intelligence,
    Constitution,
    Luck,
    All,
    Legendary,
}

impl GemType {
    pub(crate) fn parse(id: i64, debug_value: u32) -> Option<GemType> {
        Some(match id {
            0 | 1 => return None,
            10..=40 => match id % 10 {
                0 => GemType::Strength,
                1 => GemType::Dexterity,
                2 => GemType::Intelligence,
                3 => GemType::Constitution,
                4 => GemType::Luck,
                5 => GemType::All,
                
                
                6 => GemType::Legendary,
                _ => {
                    return None;
                }
            },
            _ => {
                warn!("Unknown gem: {id} - {debug_value}");
                return None;
            }
        })
    }
}


#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Hash, Enum, EnumIter, EnumCount,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum EquipmentSlot {
    Hat = 1,
    BreastPlate,
    Gloves,
    FootWear,
    Amulet,
    Belt,
    Ring,
    Talisman,
    Weapon,
    Shield,
}

impl EquipmentSlot {
    
    
    #[must_use]
    pub fn raw_id(&self) -> u8 {
        match self {
            EquipmentSlot::Weapon => 1,
            EquipmentSlot::Shield => 2,
            EquipmentSlot::BreastPlate => 3,
            EquipmentSlot::FootWear => 4,
            EquipmentSlot::Gloves => 5,
            EquipmentSlot::Hat => 6,
            EquipmentSlot::Belt => 7,
            EquipmentSlot::Amulet => 8,
            EquipmentSlot::Ring => 9,
            EquipmentSlot::Talisman => 10,
        }
    }

    
    
    #[must_use]
    pub const fn enchantment(&self) -> Option<Enchantment> {
        match self {
            EquipmentSlot::Hat => {
                Some(Enchantment::AdventurersArchaeologicalAura)
            }
            EquipmentSlot::BreastPlate => Some(Enchantment::MariosBeard),
            EquipmentSlot::Gloves => Some(Enchantment::ShadowOfTheCowboy),
            EquipmentSlot::FootWear => Some(Enchantment::ManyFeetBoots),
            EquipmentSlot::Amulet => Some(Enchantment::UnholyAcquisitiveness),
            EquipmentSlot::Belt => Some(Enchantment::ThirstyWanderer),
            EquipmentSlot::Ring => Some(Enchantment::TheGraveRobbersPrayer),
            EquipmentSlot::Talisman => Some(Enchantment::RobberBaronRitual),
            EquipmentSlot::Weapon => Some(Enchantment::SwordOfVengeance),
            EquipmentSlot::Shield => None,
        }
    }
}


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum PetItem {
    Egg(HabitatType),
    SpecialEgg(HabitatType),
    GoldenEgg,
    Nest,
    Fruit(HabitatType),
}

impl PetItem {
    pub(crate) fn parse(val: i64) -> Option<Self> {
        Some(match val {
            1..=5 => PetItem::Egg(HabitatType::from_typ_id(val)?),
            11..=15 => PetItem::SpecialEgg(HabitatType::from_typ_id(val - 10)?),
            21 => PetItem::GoldenEgg,
            22 => PetItem::Nest,
            31..=35 => PetItem::Fruit(HabitatType::from_typ_id(val - 30)?),
            _ => return None,
        })
    }
}

pub(crate) fn parse_active_potions(
    data: &[i64],
    server_time: ServerTime,
) -> [Option<Potion>; 3] {
    if data.len() < 10 {
        return Default::default();
    }
    #[allow(clippy::indexing_slicing)]
    core::array::from_fn(move |i| {
        Some(Potion {
            typ: PotionType::parse(data[i + 1])?,
            size: PotionSize::parse(data[i + 1])?,
            expires: server_time.convert_to_local(data[4 + i], "potion exp"),
            
        })
    })
}
