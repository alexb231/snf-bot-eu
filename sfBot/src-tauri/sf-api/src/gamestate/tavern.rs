use chrono::{DateTime, Local};
use log::{error, warn};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use super::{
    CCGet, CFPGet, CSTGet, ExpeditionSetting, SFError, ServerTime, items::Item,
};
use crate::{
    command::{DiceReward, DiceType},
    gamestate::rewards::Reward,
    misc::soft_into,
};


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tavern {
    
    pub quests: [Quest; 3],
    
    #[doc(alias = "alu")]
    pub thirst_for_adventure_sec: u32,
    
    pub mushroom_skip_allowed: bool,
    
    pub beer_drunk: u8,
    
    pub quicksand_glasses: u32,
    
    pub current_action: CurrentAction,
    
    pub guard_wage: u64,
    
    pub toilet: Option<Toilet>,
    
    pub dice_game: DiceGame,
    
    pub expeditions: ExpeditionsEvent,
    
    
    pub questing_preference: ExpeditionSetting,
    
    pub gamble_result: Option<GambleResult>,
    
    pub beer_max: u8,
}


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExpeditionsEvent {
    
    pub start: Option<DateTime<Local>>,
    
    pub end: Option<DateTime<Local>>,
    
    pub available: Vec<AvailableExpedition>,
    
    
    pub(crate) active: Option<Expedition>,
}

impl ExpeditionsEvent {
    
    
    #[must_use]
    pub fn is_event_ongoing(&self) -> bool {
        let now = Local::now();
        matches!((self.start, self.end), (Some(start), Some(end)) if end > now && start < now)
    }

    
    
    
    #[must_use]
    pub fn active(&self) -> Option<&Expedition> {
        self.active.as_ref().filter(|a| !a.is_finished())
    }

    
    
    
    #[must_use]
    pub fn active_mut(&mut self) -> Option<&mut Expedition> {
        self.active.as_mut().filter(|a| !a.is_finished())
    }
}


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiceGame {
    
    pub remaining: u8,
    
    pub next_free: Option<DateTime<Local>>,
    
    
    pub current_dice: Vec<DiceType>,
    
    pub reward: Option<DiceReward>,
}




#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum AvailableTasks<'a> {
    Quests(&'a [Quest; 3]),
    Expeditions(&'a [AvailableExpedition]),
}

impl Tavern {
    
    
    
    
    
    #[must_use]
    pub fn is_idle(&self) -> bool {
        match self.current_action {
            CurrentAction::Idle => true,
            CurrentAction::Expedition => self.expeditions.active.is_none(),
            _ => false,
        }
    }

    
    
    
    
    #[must_use]
    pub fn available_tasks(&self) -> AvailableTasks<'_> {
        if self.questing_preference == ExpeditionSetting::PreferExpeditions
            && self.expeditions.is_event_ongoing()
        {
            AvailableTasks::Expeditions(&self.expeditions.available)
        } else {
            AvailableTasks::Quests(&self.quests)
        }
    }

    
    
    #[must_use]
    pub fn can_change_questing_preference(&self) -> bool {
        self.thirst_for_adventure_sec == 6000 && self.beer_drunk == 0
    }
}


#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Quest {
    
    pub base_length: u32,
    
    pub base_silver: u32,
    
    pub base_experience: u32,
    
    pub item: Option<Item>,
    
    pub location_id: Location,
    
    pub monster_id: u16,
}


#[derive(Debug, Default, Clone, PartialEq, Eq, Copy, FromPrimitive, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Location {
    #[default]
    SprawlingJungle = 1,
    SkullIsland,
    EvernightForest,
    StumbleSteppe,
    ShadowrockMountain,
    SplitCanyon,
    BlackWaterSwamp,
    FloodedCaldwell,
    TuskMountain,
    MoldyForest,
    Nevermoor,
    BustedLands,
    Erogenion,
    Magmaron,
    SunburnDesert,
    Gnarogrim,
    Northrunt,
    BlackForest,
    Maerwynn,
    PlainsOfOzKorr,
    RottenLands,
}

impl Quest {
    
    
    #[must_use]
    pub fn is_red(&self) -> bool {
        matches!(self.monster_id, 139 | 145 | 148 | 152 | 155 | 157)
    }

    pub(crate) fn update(&mut self, data: &[i64]) -> Result<(), SFError> {
        
        self.monster_id = data.csimget(2, "quest monster id", 0, |a| -a)?;
        self.location_id = data
            .cfpget(3, "quest location id", |a| a)?
            .unwrap_or_default();
        self.base_length = data.csiget(4, "quest length", 100_000)?;
        self.base_experience = data.csiget(5, "quest xp", 0)?;
        self.base_silver = data.csiget(6, "quest silver", 0)?;
        Ok(())
    }
}


#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CurrentAction {
    
    #[default]
    Idle,
    
    
    CityGuard {
        
        hours: u8,
        
        busy_until: DateTime<Local>,
    },
    
    
    Quest {
        
        quest_idx: u8,
        
        busy_until: DateTime<Local>,
    },
    
    
    Expedition,
    
    
    Unknown(Option<DateTime<Local>>),
}

impl CurrentAction {
    pub(crate) fn parse(
        id: i64,
        sec: i64,
        busy_until: Option<DateTime<Local>>,
    ) -> Self {
        
        
        
        let busy_until = busy_until.unwrap_or_default();
        match id {
            0 => CurrentAction::Idle,
            1 => CurrentAction::CityGuard {
                hours: soft_into(sec, "city guard time", 10),
                busy_until,
            },
            2 => CurrentAction::Quest {
                quest_idx: soft_into(sec, "quest index", 0),
                busy_until,
            },
            4 => CurrentAction::Expedition,
            _ => {
                error!("Unknown action id combination: {id}, {busy_until:?}");
                CurrentAction::Unknown(Some(busy_until))
            }
        }
    }
}


#[derive(Debug, Clone, Default, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Toilet {
    
    pub aura: u32,
    
    pub mana_currently: u32,
    
    pub mana_total: u32,
    
    pub sacrifices_left: u32,
}

impl Toilet {
    pub(crate) fn update(
        &mut self,
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        self.aura = data.csiget(0, "aura level", 0)?;
        self.mana_currently = data.csiget(1, "mana now", 0)?;
        
        let _unknown_time = data.cstget(2, "mana time", server_time)?;
        self.mana_total = data.csiget(3, "mana missing", 1000)?;
        Ok(())
    }
}


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Expedition {
    
    pub items: [Option<ExpeditionThing>; 4],

    
    pub target_thing: ExpeditionThing,
    
    pub target_current: u8,
    
    pub target_amount: u8,

    
    pub current_floor: u8,
    
    pub heroism: i32,

    pub(crate) floor_stage: i64,

    
    pub(crate) rewards: Vec<Reward>,
    pub(crate) halftime_for_boss_id: i64,
    
    pub(crate) boss: ExpeditionBoss,
    
    pub(crate) encounters: Vec<ExpeditionEncounter>,
    pub(crate) busy_until: Option<DateTime<Local>>,
    pub(crate) busy_since: Option<DateTime<Local>>,
}

impl Expedition {
    pub(crate) fn update_encounters(&mut self, data: &[i64]) {
        if !data.len().is_multiple_of(2) {
            warn!("weird encounters: {data:?}");
        }
        let default_ecp = |ci| {
            warn!("Unknown encounter: {ci}");
            ExpeditionThing::Unknown
        };
        self.encounters = data
            .chunks_exact(2)
            .filter_map(|ci| {
                let raw = *ci.first()?;
                let typ = FromPrimitive::from_i64(raw)
                    .unwrap_or_else(|| default_ecp(raw));
                let heroism = soft_into(*ci.get(1)?, "e heroism", 0);
                Some(ExpeditionEncounter { typ, heroism })
            })
            .collect();
    }

    
    
    
    #[must_use]
    pub fn current_stage(&self) -> ExpeditionStage {
        let cross_roads =
            || ExpeditionStage::Encounters(self.encounters.clone());

        match self.floor_stage {
            1 => cross_roads(),
            2 => ExpeditionStage::Boss(self.boss),
            3 => ExpeditionStage::Rewards(self.rewards.clone()),
            4 => match self.busy_until {
                Some(x) if x > Local::now() => ExpeditionStage::Waiting {
                    busy_until: x,
                    busy_since: self.busy_since.unwrap_or_default(),
                },
                _ if self.current_floor == 10 => ExpeditionStage::Finished,
                _ => cross_roads(),
            },
            _ => ExpeditionStage::Unknown,
        }
    }

    
    #[must_use]
    pub fn is_finished(&self) -> bool {
        matches!(self.current_stage(), ExpeditionStage::Finished)
    }
}


#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExpeditionStage {
    
    Rewards(Vec<Reward>),
    
    Boss(ExpeditionBoss),
    
    Encounters(Vec<ExpeditionEncounter>),
    
    
    
    
    Waiting {
        
        busy_since: DateTime<Local>,
        
        busy_until: DateTime<Local>,
    },
    
    Finished,
    
    
    Unknown,
}

impl Default for ExpeditionStage {
    fn default() -> Self {
        ExpeditionStage::Encounters(Vec::new())
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExpeditionBoss {
    
    pub id: i64,
    
    pub items: u8,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExpeditionEncounter {
    
    pub typ: ExpeditionThing,
    
    
    pub heroism: i32,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs, clippy::doc_markdown)]
pub enum ExpeditionThing {
    #[default]
    Unknown = 0,

    Dummy1 = 1,
    Dummy2 = 2,
    Dummy3 = 3,

    ToiletPaper = 11,

    Bait = 21,
    
    Dragon = 22,

    CampFire = 31,
    Phoenix = 32,
    
    BurntCampfire = 33,

    UnicornHorn = 41,
    Donkey = 42,
    Rainbow = 43,
    
    Unicorn = 44,

    CupCake = 51,
    
    Cake = 61,

    SmallHurdle = 71,
    BigHurdle = 72,
    
    WinnersPodium = 73,

    Socks = 81,
    ClothPile = 82,
    
    RevealingCouple = 83,

    SwordInStone = 91,
    BentSword = 92,
    BrokenSword = 93,

    Well = 101,
    Girl = 102,
    
    Balloons = 103,

    Prince = 111,
    
    RoyalFrog = 112,

    Hand = 121,
    Feet = 122,
    Body = 123,
    
    Klaus = 124,

    Key = 131,
    Suitcase = 132,

    
    DummyBounty = 1000,
    ToiletPaperBounty = 1001,
    DragonBounty = 1002,
    BurntCampfireBounty = 1003,
    UnicornBounty = 1004,
    WinnerPodiumBounty = 1007,
    RevealingCoupleBounty = 1008,
    BrokenSwordBounty = 1009,
    BaloonBounty = 1010,
    FrogBounty = 1011,
    KlausBounty = 1012,
}

impl ExpeditionThing {
    
    
    #[must_use]
    #[allow(clippy::enum_glob_use)]
    pub fn required_bounty(&self) -> Option<ExpeditionThing> {
        use ExpeditionThing::*;
        Some(match self {
            Dummy1 | Dummy2 | Dummy3 => DummyBounty,
            ToiletPaper => ToiletPaperBounty,
            Dragon => DragonBounty,
            BurntCampfire => BurntCampfireBounty,
            Unicorn => UnicornBounty,
            WinnersPodium => WinnerPodiumBounty,
            RevealingCouple => RevealingCoupleBounty,
            BrokenSword => BrokenSwordBounty,
            Balloons => BaloonBounty,
            RoyalFrog => FrogBounty,
            Klaus => KlausBounty,
            _ => return None,
        })
    }

    
    
    #[must_use]
    #[allow(clippy::enum_glob_use)]
    pub fn is_bounty_for(&self) -> Option<&'static [ExpeditionThing]> {
        use ExpeditionThing::*;
        Some(match self {
            DummyBounty => &[Dummy1, Dummy2, Dummy3],
            ToiletPaperBounty => &[ToiletPaper],
            DragonBounty => &[Dragon],
            BurntCampfireBounty => &[BurntCampfire],
            UnicornBounty => &[Unicorn],
            WinnerPodiumBounty => &[WinnersPodium],
            RevealingCoupleBounty => &[RevealingCouple],
            BrokenSwordBounty => &[BrokenSword],
            BaloonBounty => &[Balloons],
            FrogBounty => &[RoyalFrog],
            KlausBounty => &[Klaus],
            _ => return None,
        })
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AvailableExpedition {
    
    pub target: ExpeditionThing,
    
    
    pub thirst_for_adventure_sec: u32,
    
    
    pub location_1: Location,
    
    
    pub location_2: Location,
    
    
    pub special: Option<ExpeditionSpecial>,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum ExpeditionSpecial {
    
    Egg = 1,
    
    DailyTask,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum GambleResult {
    SilverChange(i64),
    MushroomChange(i32),
}
