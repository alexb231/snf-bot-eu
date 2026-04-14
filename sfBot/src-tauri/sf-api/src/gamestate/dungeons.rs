use chrono::{DateTime, Local};
use enum_map::{Enum, EnumArray, EnumMap};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum::{EnumCount, EnumIter};

use super::{
    AttributeType, CCGet, Class, EnumMapGet, Item, SFError, ServerTime,
    items::Equipment,
};
use crate::misc::soft_into;


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Portal {
    
    pub finished: u16,
    
    
    
    pub can_fight: bool,
    
    
    pub enemy_level: u32,
    
    pub enemy_hp_percentage: u8,
    
    pub player_hp_bonus: u16,
}

impl Portal {
    pub(crate) fn update(
        &mut self,
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        self.finished = data.csiget(0, "portal fights", 10_000)?;
        self.enemy_hp_percentage = data.csiget(1, "portal hp", 0)?;

        let current_day = chrono::Datelike::ordinal(&server_time.current());
        let last_portal_day: u32 = data.csiget(2, "portal day", 0)?;
        self.can_fight = last_portal_day != current_day;

        Ok(())
    }
}



#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dungeons {
    
    pub next_free_fight: Option<DateTime<Local>>,
    
    pub light: EnumMap<LightDungeon, DungeonProgress>,
    
    
    pub shadow: EnumMap<ShadowDungeon, DungeonProgress>,
    pub portal: Option<Portal>,
    
    
    pub companions: Option<EnumMap<CompanionClass, Companion>>,
}

impl Dungeons {
    
    pub fn progress(&self, dungeon: impl Into<Dungeon>) -> DungeonProgress {
        let dungeon: Dungeon = dungeon.into();
        match dungeon {
            Dungeon::Light(dungeon) => *self.light.get(dungeon),
            Dungeon::Shadow(dungeon) => *self.shadow.get(dungeon),
        }
    }

    
    
    
    
    #[cfg(feature = "simulation")]
    pub fn current_enemy(
        &self,
        dungeon: impl Into<Dungeon> + Copy,
    ) -> Option<&'static crate::simulate::Monster> {
        get_dungeon_monster(dungeon, self.progress(dungeon))
    }
}


#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DungeonProgress {
    
    #[default]
    Locked,
    
    Open {
        
        finished: u16,
    },
    
    Finished,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum DungeonType {
    Light,
    Shadow,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Dungeon {
    Light(LightDungeon),
    Shadow(ShadowDungeon),
}

impl Dungeon {
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn is_with_companions(self) -> bool {
        match self {
            Dungeon::Light(LightDungeon::Tower) => true,
            Dungeon::Shadow(ShadowDungeon::Twister) => false,
            Dungeon::Light(_) => false,
            Dungeon::Shadow(_) => true,
        }
    }
}




#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumCount,
    EnumIter,
    Enum,
    FromPrimitive,
    Hash,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum LightDungeon {
    DesecratedCatacombs = 0,
    MinesOfGloria = 1,
    RuinsOfGnark = 2,
    CutthroatGrotto = 3,
    EmeraldScaleAltar = 4,
    ToxicTree = 5,
    MagmaStream = 6,
    FrostBloodTemple = 7,
    PyramidsofMadness = 8,
    BlackSkullFortress = 9,
    CircusOfHorror = 10,
    Hell = 11,
    The13thFloor = 12,
    Easteros = 13,
    Tower = 14,
    TimeHonoredSchoolofMagic = 15,
    Hemorridor = 16,
    NordicGods = 18,
    MountOlympus = 19,
    TavernoftheDarkDoppelgangers = 20,
    DragonsHoard = 21,
    HouseOfHorrors = 22,
    ThirdLeagueOfSuperheroes = 23,
    DojoOfChildhoodHeroes = 24,
    MonsterGrotto = 25,
    CityOfIntrigues = 26,
    SchoolOfMagicExpress = 27,
    AshMountain = 28,
    PlayaGamesHQ = 29,
    TrainingCamp = 30,
    Sandstorm = 31,
    ArcadeOfTheOldPixelIcons = 32,
    TheServerRoom = 33,
    WorkshopOfTheHunters = 34,
    RetroTVLegends = 35,
    MeetingRoom = 36,
}

impl From<LightDungeon> for Dungeon {
    fn from(val: LightDungeon) -> Self {
        Dungeon::Light(val)
    }
}



#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumCount,
    EnumIter,
    Enum,
    FromPrimitive,
    Hash,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum ShadowDungeon {
    DesecratedCatacombs = 0,
    MinesOfGloria = 1,
    RuinsOfGnark = 2,
    CutthroatGrotto = 3,
    EmeraldScaleAltar = 4,
    ToxicTree = 5,
    MagmaStream = 6,
    FrostBloodTemple = 7,
    PyramidsOfMadness = 8,
    BlackSkullFortress = 9,
    CircusOfHorror = 10,
    Hell = 11,
    The13thFloor = 12,
    Easteros = 13,
    Twister = 14,
    TimeHonoredSchoolOfMagic = 15,
    Hemorridor = 16,
    ContinuousLoopofIdols = 17,
    NordicGods = 18,
    MountOlympus = 19,
    TavernOfTheDarkDoppelgangers = 20,
    DragonsHoard = 21,
    HouseOfHorrors = 22,
    ThirdLeagueofSuperheroes = 23,
    DojoOfChildhoodHeroes = 24,
    MonsterGrotto = 25,
    CityOfIntrigues = 26,
    SchoolOfMagicExpress = 27,
    AshMountain = 28,
    PlayaGamesHQ = 29,
    
    ArcadeOfTheOldPixelIcons = 32,
    TheServerRoom = 33,
    WorkshopOfTheHunters = 34,
    RetroTVLegends = 35,
    MeetingRoom = 36,
}

impl From<ShadowDungeon> for Dungeon {
    fn from(val: ShadowDungeon) -> Self {
        Dungeon::Shadow(val)
    }
}

fn update_progress<T: FromPrimitive + EnumArray<DungeonProgress>>(
    data: &[i64],
    dungeons: &mut EnumMap<T, DungeonProgress>,
) {
    for (dungeon_id, progress) in data.iter().copied().enumerate() {
        let Some(dungeon_typ) = FromPrimitive::from_usize(dungeon_id) else {
            continue;
        };
        let dungeon = dungeons.get_mut(dungeon_typ);
        *dungeon = match progress {
            -1 => DungeonProgress::Locked,
            x => {
                let stage = soft_into(x, "dungeon progress", 0);
                if stage == 10 || stage == 100 && dungeon_id == 14 {
                    DungeonProgress::Finished
                } else {
                    DungeonProgress::Open { finished: stage }
                }
            }
        };
    }
}

impl Dungeons {
    
    #[must_use]
    pub fn can_companion_equip(
        &self,
        companion: CompanionClass,
        item: &Item,
    ) -> bool {
        
        if self.companions.is_none() {
            return false;
        }
        item.can_be_equipped_by_companion(companion)
    }

    pub(crate) fn update_progress(
        &mut self,
        data: &[i64],
        dungeon_type: DungeonType,
    ) {
        match dungeon_type {
            DungeonType::Light => update_progress(data, &mut self.light),
            DungeonType::Shadow => {
                update_progress(data, &mut self.shadow);
                for (dungeon, limit) in [
                    (ShadowDungeon::ContinuousLoopofIdols, 21),
                    (ShadowDungeon::Twister, 1000),
                ] {
                    let d = self.shadow.get_mut(dungeon);
                    if let DungeonProgress::Open { finished, .. } = d
                        && *finished >= limit
                    {
                        *d = DungeonProgress::Finished;
                    }
                }
            }
        }
    }
}



#[derive(
    Debug, Clone, Copy, PartialEq, Eq, EnumCount, Enum, EnumIter, Hash,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CompanionClass {
    
    Warrior = 0,
    
    Mage = 1,
    
    Scout = 2,
}

impl From<CompanionClass> for Class {
    fn from(value: CompanionClass) -> Self {
        match value {
            CompanionClass::Warrior => Class::Warrior,
            CompanionClass::Mage => Class::Mage,
            CompanionClass::Scout => Class::Scout,
        }
    }
}



#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Companion {
    
    
    pub level: i64,
    
    pub equipment: Equipment,
    
    pub attributes: EnumMap<AttributeType, u32>,
}

#[cfg(feature = "simulation")]
pub fn get_dungeon_monster(
    dungeon: impl Into<Dungeon>,
    progress: DungeonProgress,
) -> Option<&'static crate::simulate::Monster> {
    let stage = match progress {
        DungeonProgress::Open { finished } => finished,
        DungeonProgress::Locked | DungeonProgress::Finished => return None,
    };
    crate::simulate::constants::get_dungeon_enemies(dungeon.into())
        .get(stage as usize)
}
