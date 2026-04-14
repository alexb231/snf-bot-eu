use std::fmt::Debug;

use chrono::{DateTime, Local};
use enum_map::EnumMap;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use super::{NormalCost, RelationEntry, SFError, ScrapBook};
use crate::{PlayerId, command::*, gamestate::items::*, misc::*};



#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Character {
    
    pub player_save_id: u64,
    
    
    
    pub player_id: PlayerId,
    
    pub name: String,
    
    pub level: u16,
    
    pub silver: u64,
    
    pub mushrooms: u32,

    
    pub class: Class,

    
    
    pub race: Race,
    
    pub portrait: Portrait,
    
    pub description: String,

    
    pub experience: u64,
    
    
    pub next_level_xp: u64,
    
    pub honor: u32,
    
    pub rank: u32,

    
    
    pub inventory: Inventory,
    
    pub equipment: Equipment,

    
    
    pub mannequin: Option<Equipment>,
    
    pub active_potions: [Option<Potion>; 3],

    
    pub armor: u64,

    
    pub min_damage: u32,
    
    pub max_damage: u32,

    
    pub attribute_basis: EnumMap<AttributeType, u32>,
    
    pub attribute_additions: EnumMap<AttributeType, u32>,
    
    
    pub attribute_times_bought: EnumMap<AttributeType, u32>,

    
    pub mount: Option<Mount>,
    
    
    pub mount_end: Option<DateTime<Local>>,
    
    pub mount_dragon_refund: u64,

    
    pub scrapbook: Option<ScrapBook>,

    
    
    pub relations: Vec<RelationEntry>,

    
    pub sf_home_id: Option<String>,
    
    pub webshop_id: Option<String>,
}




#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub struct Portrait {
    
    pub gender: Gender,
    pub hair_color: u8,
    pub hair: u8,
    pub mouth: u8,
    pub brows: u8,
    pub eyes: u8,
    pub beards: u8,
    pub nose: u8,
    pub ears: u8,
    pub extra: u8,
    pub horns: u8,
    
    pub special_portrait: i64,
}

impl Portrait {
    pub(crate) fn parse(data: &[i64]) -> Result<Portrait, SFError> {
        Ok(Self {
            mouth: data.csiget(0, "mouth", 1)?,
            hair_color: data.csimget(1, "hair color", 100, |a| a / 100)?,
            hair: data.csimget(1, "hair", 1, |a| a % 100)?,
            brows: data.csimget(2, "brows", 1, |a| a % 100)?,
            eyes: data.csiget(3, "eyes", 1)?,
            beards: data.csimget(4, "beards", 1, |a| a % 100)?,
            nose: data.csiget(5, "nose", 1)?,
            ears: data.csiget(6, "ears", 1)?,
            extra: data.csiget(7, "extra", 1)?,
            horns: data.csimget(8, "horns", 1, |a| a % 100)?,
            special_portrait: data.cget(9, "special portrait")?,
            gender: Gender::from_i64(data.csimget(11, "gender", 1, |a| a % 2)?)
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, FromPrimitive, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Gender {
    #[default]
    Female = 0,
    Male,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, FromPrimitive, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Class {
    #[default]
    Warrior = 0,
    Mage,
    Scout,
    Assassin,
    BattleMage,
    Berserker,
    DemonHunter,
    Druid,
    Bard,
    Necromancer,
    Paladin,
    PlagueDoctor,
}

#[allow(clippy::enum_glob_use)]
impl Class {
    #[must_use]
    #[allow(clippy::enum_glob_use)]
    pub fn main_attribute(&self) -> AttributeType {
        use Class::*;
        match self {
            Paladin | BattleMage | Berserker | Warrior => {
                AttributeType::Strength
            }
            Assassin | DemonHunter | Scout | PlagueDoctor => {
                AttributeType::Dexterity
            }
            Druid | Bard | Necromancer | Mage => AttributeType::Intelligence,
        }
    }

    #[must_use]
    pub fn weapon_multiplier(self) -> f64 {
        use Class::*;
        match self {
            PlagueDoctor | Paladin | Warrior | Assassin | BattleMage
            | Berserker => 2.0,
            
            Scout | DemonHunter => 2.5,
            Mage | Druid | Bard | Necromancer => 4.5,
        }
    }

    #[must_use]
    pub fn weapon_gem_multiplier(&self) -> i32 {
        match self {
            Class::Warrior | Class::Assassin | Class::Berserker => 1,
            _ => 2,
        }
    }

    #[must_use]
    pub fn weapon_attribute_multiplier(&self) -> i32 {
        match self {
            Class::Warrior
            | Class::BattleMage
            | Class::Berserker
            | Class::Paladin
            | Class::PlagueDoctor
            | Class::Assassin => 1,
            _ => 2,
        }
    }

    #[cfg(feature = "simulation")]
    #[must_use]
    pub(crate) fn health_multiplier(self, is_companion: bool) -> f64 {
        use Class::*;

        match self {
            Warrior if is_companion => 6.1,
            Warrior | BattleMage | Druid => 5.0,
            Paladin => 6.0,
            PlagueDoctor | Scout | Assassin | Berserker | DemonHunter
            | Necromancer => 4.0,
            Mage | Bard => 2.0,
        }
    }

    #[must_use]
    pub fn item_armor_multiplier(&self) -> f64 {
        match self {
            Class::Warrior
            | Class::Berserker
            | Class::DemonHunter
            | Class::Paladin => 15.0,
            Class::Scout | Class::Assassin | Class::Druid | Class::Bard => 7.5,
            Class::Mage
            | Class::BattleMage
            | Class::Necromancer
            | Class::PlagueDoctor => 3.0,
        }
    }

    #[must_use]
    pub fn item_bonus_multiplier(&self) -> f64 {
        match self {
            Class::BattleMage | Class::PlagueDoctor => 1.11,
            Class::Berserker => 1.1,
            _ => 1.0,
        }
    }

    #[must_use]
    pub fn armor_multiplier(&self) -> f64 {
        match self {
            Class::BattleMage => 5.0,
            Class::Bard | Class::Necromancer | Class::PlagueDoctor => 2.0,
            Class::Berserker => 0.5,
            _ => 1.0,
        }
    }

    #[must_use]
    pub fn max_armor_reduction(&self) -> u32 {
        match self {
            Class::Mage => 10,
            Class::Warrior
            | Class::BattleMage
            | Class::DemonHunter
            | Class::Bard => 50,
            Class::Paladin => 45,
            Class::Scout
            | Class::Assassin
            | Class::Berserker
            | Class::Druid => 25,
            Class::Necromancer | Class::PlagueDoctor => 20,
        }
    }

    #[must_use]
    pub fn damage_multiplier(&self) -> f64 {
        match self {
            Class::Assassin => 0.625,
            Class::Berserker | Class::PlagueDoctor => 1.25,
            Class::Druid => 1.0 / 3.0,
            Class::Bard => 1.125,
            Class::Necromancer => 5.0 / 9.0,
            Class::Paladin => 0.833,
            _ => 1.0,
        }
    }

    #[must_use]
    pub fn can_wear_shield(self) -> bool {
        matches!(self, Self::Paladin | Self::Warrior)
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy, FromPrimitive, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Race {
    #[default]
    Human = 1,
    Elf,
    Dwarf,
    Gnome,
    Orc,
    DarkElf,
    Goblin,
    Demon,
}

impl Race {
    
    
    
    
    #[must_use]
    pub fn stat_modifiers(self) -> EnumMap<AttributeType, i32> {
        let raw = match self {
            Race::Human => [0, 0, 0, 0, 0],
            Race::Elf => [-1, 2, 0, -1, 0],
            Race::Dwarf => [0, -2, -1, 2, 1],
            Race::Gnome => [-2, 3, -1, -1, 1],
            Race::Orc => [1, 0, -1, 0, 0],
            Race::DarkElf => [-2, 2, 1, -1, 0],
            Race::Goblin => [-2, 2, 0, -1, 1],
            Race::Demon => [3, -1, 0, 1, -3],
        };
        EnumMap::from_array(raw)
    }
}

#[derive(Debug, Copy, Clone, FromPrimitive, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Mount {
    Cow = 1,
    Horse = 2,
    Tiger = 3,
    Dragon = 4,
}

impl Mount {
    
    #[must_use]
    pub fn cost(&self) -> NormalCost {
        match self {
            Mount::Cow => NormalCost {
                silver: 100,
                mushrooms: 0,
            },
            Mount::Horse => NormalCost {
                silver: 500,
                mushrooms: 0,
            },
            Mount::Tiger => NormalCost {
                silver: 1000,
                mushrooms: 1,
            },
            Mount::Dragon => NormalCost {
                silver: 0,
                mushrooms: 25,
            },
        }
    }
}
