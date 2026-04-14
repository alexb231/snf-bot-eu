use chrono::{DateTime, Local};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::{error::SFError, gamestate::items::Item, misc::*};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub struct LegendaryDungeonEvent {
    
    pub theme: Option<LegendaryDungeonEventTheme>,
    
    
    pub start: Option<DateTime<Local>>,
    
    pub end: Option<DateTime<Local>>,
    
    
    pub close: Option<DateTime<Local>>,

    pub(crate) active: Option<LegendaryDungeon>,
}

impl LegendaryDungeonEvent {
    #[must_use]
    
    
    pub fn status(&self) -> LegendaryDungeonStatus<'_> {
        use LegendaryDungeonStage as Stage;
        use LegendaryDungeonStatus as Status;

        let now = Local::now();
        if self.start.is_none_or(|a| a > now) {
            return Status::Unavailable;
        }
        if self.close.is_none_or(|a| a < now) {
            return Status::Unavailable;
        }

        let Some(theme) = self
            .theme
            .filter(|a| !matches!(a, LegendaryDungeonEventTheme::Unknown))
        else {
            return Status::Unavailable;
        };

        let Some(active) = &self.active else {
            return if self.end.is_some_and(|a| a > now) {
                Status::NotEntered(theme)
            } else {
                Status::Unavailable
            };
        };

        if !active.pending_items.is_empty() {
            return Status::TakeItem {
                dungeon: active,
                items: &active.pending_items,
            };
        }

        let room_status = |status| Status::Room {
            dungeon: active,
            status,
            encounter: active.encounter,
            typ: active.room_type,
        };

        match active.stage {
            Stage::NotEntered => Status::NotEntered(theme),
            Stage::DoorSelect => Status::DoorSelect {
                dungeon: active,
                doors: &active.doors,
            },
            Stage::RoomSpecial if active.room_type == RoomType::BossRoom => {
                Status::PickGem {
                    dungeon: active,
                    available_gems: &active.available_gems,
                }
            }
            #[allow(clippy::pedantic)]
            Stage::Healing => {
                let started = active.healing_start.unwrap_or_default();
                let now = Local::now();
                let elapsed = now - started;
                let elapsed_minuted = elapsed.num_minutes() as f64;

                let heal_per_day = 100.0;
                let heal_per_hour = heal_per_day / 24.0;
                let heal_per_minute = heal_per_hour / 60.0;

                let healed = elapsed_minuted * heal_per_minute;
                let current_health_percent = healed.clamp(0.0, 100.0) as u8;

                Status::Healing {
                    dungeon: active,
                    started,
                    current_health_percent,
                }
            }
            Stage::RoomEntered => room_status(RoomStatus::Entered),
            Stage::RoomInteracted => room_status(RoomStatus::Interacted),
            Stage::RoomSpecial => room_status(RoomStatus::Special),
            Stage::RoomFinished => room_status(RoomStatus::Finished),
            Stage::Unknown => Status::Unknown,
            Stage::Finished => Status::Ended(&active.total_stats),
        }
    }
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum LegendaryDungeonEventTheme {
    
    DiabolicalCompanyParty = 1,
    
    LordOfTheThings = 2,
    
    FantasticLegendaries = 3,
    
    ShadyBirthdayBash = 4,
    
    MassiveWinterSpectacle = 5,
    
    AbyssOfMadness = 6,
    
    HuntForBlazingEasterEgg = 7,
    
    VileVacation = 8,

    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub struct LegendaryDungeon {
    
    pub stats: Stats,
    
    pub total_stats: TotalStats,

    
    pub current_hp: i64,
    
    
    
    
    pub pre_battle_hp: i64,
    
    pub max_hp: i64,

    
    pub blessings: [Option<DungeonEffect>; 3],
    
    pub curses: [Option<DungeonEffect>; 3],

    pub(crate) stage: LegendaryDungeonStage,

    
    pub current_floor: u32,
    
    pub max_floor: u32,
    
    pub keys: u32,

    
    
    pub heal_quarter_cost: u32,
    
    pub merchant_offers: Vec<MerchantOffer>,
    
    pub active_gems: Vec<GemOfFate>,

    
    pub(crate) doors: [Door; 2],
    pub(crate) room_type: RoomType,
    
    pub(crate) encounter: RoomEncounter,
    
    pub(crate) pending_items: Vec<Item>,
    
    pub(crate) available_gems: Vec<GemOfFate>,

    
    
    
    health_status: i64,

    
    pub(crate) healing_start: Option<DateTime<Local>>,
}

impl LegendaryDungeon {
    pub(crate) fn update(&mut self, data: &[i64]) -> Result<(), SFError> {
        
        self.health_status = data.cget(1, "ld unknown")?;

        self.current_hp = data.cget(2, "ld current hp")?;
        self.pre_battle_hp = data.cget(3, "ld pre hp")?;
        self.max_hp = data.cget(4, "ld max hp")?;

        for (pos, v) in self.blessings.iter_mut().enumerate() {
            let s = data.csiget(11 + pos, "ld blessing rem", 0)?;
            *v = DungeonEffect::parse(
                data.csiget(5 + pos, "ld blessing typ", 0)?,
                s / 10_000,
                data.csiget(42 + pos, "ld blessing max", 0)?,
                s % 10_000,
            );
        }
        for (pos, v) in self.curses.iter_mut().enumerate() {
            let s_pos = match pos {
                0 => 14,
                1 => 40,
                _ => 41,
            };
            let s = data.csiget(s_pos, "ld blessing rem", 0)?;

            *v = DungeonEffect::parse(
                data.csiget(8 + pos, "ld blessing typ", 0)?,
                s / 10_000,
                data.csiget(45 + pos, "ld blessing max", 0)?,
                s % 10_000,
            );
        }

        self.stage =
            data.cfpget(15, "dungeon stage", |a| a)?.unwrap_or_default();

        

        self.current_floor = data.csiget(17, "ld floor", 0)?;
        self.max_floor = data.csiget(18, "ld max floor", 0)?;

        if self.stage == LegendaryDungeonStage::DoorSelect {
            for (pos, v) in self.doors.iter_mut().enumerate() {
                v.typ = data
                    .cfpget(19 + pos, "ld door typ", |a| a)?
                    .unwrap_or_default();

                let raw_trap = data.cget(25 + pos, "ld door trap")?;
                v.trap = match raw_trap {
                    0 => None,
                    x => FromPrimitive::from_i64(x),
                }
            }
        } else {
            self.room_type =
                data.cfpget(19, "ld room type", |a| a)?.unwrap_or_default();
        }

        let raw_enc = data.csiget(22, "ld encounter", 999)?;
        self.encounter = RoomEncounter::parse(raw_enc);

        

        self.keys = data.csiget(39, "ld keys", 0)?;

        
        

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum LegendaryDungeonStatus<'a> {
    
    
    Unavailable,
    
    
    NotEntered(LegendaryDungeonEventTheme),
    
    
    Ended(&'a TotalStats),
    
    
    
    DoorSelect {
        
        dungeon: &'a LegendaryDungeon,
        
        doors: &'a [Door; 2],
    },
    
    
    
    PickGem {
        
        dungeon: &'a LegendaryDungeon,
        
        available_gems: &'a [GemOfFate],
    },
    
    Healing {
        
        dungeon: &'a LegendaryDungeon,
        
        started: DateTime<Local>,
        
        
        current_health_percent: u8,
    },
    
    Room {
        
        dungeon: &'a LegendaryDungeon,
        
        status: RoomStatus,
        
        encounter: RoomEncounter,
        
        typ: RoomType,
    },
    
    TakeItem {
        
        dungeon: &'a LegendaryDungeon,
        
        items: &'a [Item],
    },
    
    
    
    Unknown,
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum LegendaryDungeonStage {
    
    NotEntered = 0,

    
    DoorSelect = 1,

    
    RoomEntered = 10,
    
    RoomInteracted = 11,
    
    RoomSpecial = 12,

    
    RoomFinished = 100,
    
    Healing = 101,
    
    Finished = 102,

    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum RoomType {
    
    Generic = 1,
    
    BossRoom = 4,
    
    FinalBossRoom = 5,

    
    Encounter = 100,
    
    Empty = 200,
    
    
    FountainOfLife = 301,
    
    HoleInTheFloor = 302,
    
    
    PileOfRocks = 303,
    
    
    
    TheFloorIsLava = 304,
    
    
    
    
    DungeonNarrator = 305,
    
    
    
    
    FloodedRoom = 306,
    
    
    WishingWell = 307,
    
    
    
    
    
    
    
    
    RockPaperScissors = 308,
    
    
    
    
    Sewers = 309,
    
    
    UndeadFiend = 310,
    
    LockerRoom = 311,
    
    UnlockedSarcophagus = 312,
    
    
    
    Valaraukar = 313,
    
    
    PileOfWood = 314,
    
    KeyMasterShop = 315,
    
    
    
    
    WheelOfFortune = 316,
    
    
    
    
    
    
    
    
    
    
    SpiderWeb = 317,

    
    
    BetaRoom = 319,
    
    FlyingTube = 320,
    
    SoulBath = 321,
    
    ArcaneSplintersCave = 322,
    
    
    
    
    KeyToFailureShop = 323,
    
    
    
    RainbowRoom = 324,
    
    
    
    
    PigRoom = 325,
    
    
    AuctionHouse = 326,
    
    MonsterCat1 = 327,
    
    MonsterCat2 = 328,
    
    Armory = 329,
    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum RoomStatus {
    
    Entered,
    
    Interacted,
    
    Special,
    
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum RoomEncounter {
    
    BronzeChest,
    
    SilverChest,
    
    EpicChest,
    
    Crate1,
    
    Crate2,
    
    Crate3,

    
    WarriorSkeleton,
    
    MageSkeleton,
    
    Barrel,

    
    MimicChest,
    
    SacrificialChest,
    
    CurseChest,
    
    PrizeChest,
    
    SatedChest,

    
    Monster(u16),
    
    #[default]
    Unknown,
}

impl RoomEncounter {
    pub(crate) fn parse(val: i64) -> RoomEncounter {
        match val {
            0 => RoomEncounter::BronzeChest,
            1 => RoomEncounter::SilverChest,
            2 => RoomEncounter::EpicChest,
            100 => RoomEncounter::Crate1,
            101 => RoomEncounter::Crate2,
            102 => RoomEncounter::Crate3,
            300 => RoomEncounter::MageSkeleton,
            301 => RoomEncounter::WarriorSkeleton,
            400 => RoomEncounter::Barrel,
            500 => RoomEncounter::MimicChest,
            600 => RoomEncounter::SacrificialChest,
            601 => RoomEncounter::CurseChest,
            602 => RoomEncounter::PrizeChest,
            603 => RoomEncounter::SatedChest,
            x if x.is_negative() => {
                RoomEncounter::Monster(x.abs().try_into().unwrap_or_default())
            }
            _ => {
                log::warn!("Unknown room encounter: {val}");
                RoomEncounter::Unknown
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub struct Door {
    
    pub typ: DoorType,
    
    pub trap: Option<DoorTrap>,
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum DoorType {
    
    Monster1 = 1,
    
    Monster2 = 2,
    
    Monster3 = 3,
    
    Boss1 = 4,
    
    Boss2 = 5,

    
    
    Blocked = 1000,
    
    
    MysteryDoor = 1001,
    
    LockedDoor = 1002,
    
    OpenDoor = 1003,
    
    
    EpicDoor = 1004,
    
    DoubleLockedDoor = 1005,
    
    GoldenDoor = 1006,
    
    
    SacrificialDoor = 1007,
    
    
    CursedDoor = 1008,
    
    KeyMasterShop = 1009,
    
    BlessingDoor = 1010,
    
    
    
    
    
    
    
    #[doc(alias = "Destiny")]
    Wheel = 1011,
    
    
    
    
    
    Wood = 1012,
    
    
    
    
    
    Stone = 1013,
    
    
    
    
    
    Souls = 1014,
    
    
    
    
    
    Metal = 1015,
    
    
    
    
    
    Arcane = 1016,
    
    
    
    
    
    QuicksandGlasses = 1017,
    
    TrialRoom1 = 1018,
    
    TrialRoom2 = 1019,
    
    TrialRoom3 = 1020,
    
    TrialRoom4 = 1021,
    
    TrialRoom5 = 1022,
    
    TrialRoomExit = 1023,

    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum DoorTrap {
    
    PoisonedDaggers = 1,
    
    SwingingAxe = 2,
    
    PaintBucket = 3,
    
    BearTrap = 4,
    
    Guillotine = 5,
    
    HammerAmbush = 6,
    
    TripWire = 7,
    
    TopSpikes = 8,
    
    Shark = 9,

    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DungeonEffect {
    
    pub typ: DungeonEffectType,
    
    pub remaining_uses: u32,
    
    
    pub max_uses: u32,
    
    pub strength: u32,
}

impl DungeonEffect {
    pub(crate) fn parse(
        typ: i64,
        remaining: i64,
        max_uses: i64,
        strength: i64,
    ) -> Option<Self> {
        if typ <= 0 {
            return None;
        }
        let typ: DungeonEffectType =
            FromPrimitive::from_i64(typ).unwrap_or_default();

        let remaining_uses: u32 = remaining.try_into().unwrap_or(0);
        let max_uses: u32 = max_uses.try_into().unwrap_or(0);
        let strength: u32 = strength.try_into().unwrap_or(0);

        Some(DungeonEffect {
            typ,
            remaining_uses,
            max_uses,
            strength,
        })
    }
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum DungeonEffectType {
    
    Raider = 1,
    
    OneHitWonder = 2,
    
    EscapeAssistant = 3,
    
    DisarmTraps = 4,
    
    LockPick = 5,
    
    KeyMoment = 6,
    
    ElixirOfLife = 7,
    
    RoadToRecovery = 8,

    
    BrokenArmor = 101,
    
    Poisoned = 102,
    
    Panderous = 103,
    
    GoldRushHangover = 104,
    
    HardLock = 105,

    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub struct Stats {
    
    pub items_found: u32,
    
    pub epics_found: u32,
    
    pub keys_found: u32,
    
    pub silver_found: u64,
    
    pub attempts: u32,
}

impl Stats {
    pub(crate) fn parse(data: &[i64]) -> Result<Self, SFError> {
        Ok(Stats {
            items_found: data.csiget(0, "ld item found", 0)?,
            epics_found: data.csiget(1, "ld epic found", 0)?,
            keys_found: data.csiget(2, "ld keys found", 0)?,
            silver_found: data.csiget(3, "ld silver found", 0)?,
            attempts: data.csiget(4, "ld attempts", 0)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub struct MerchantOffer {
    
    pub typ: DungeonEffectType,
    
    pub max_uses: u32,
    
    pub strength: u32,
    
    pub keys: u32,
}

impl MerchantOffer {
    pub(crate) fn parse(data: &[i64]) -> Result<Option<Self>, SFError> {
        if data.iter().all(|a| *a == 0) {
            return Ok(None);
        }
        let typ: DungeonEffectType = data
            .cfpget(0, "ld merchant offer type", |a| a)?
            .unwrap_or_default();

        let s: u32 = data.csiget(1, "ld merchant effect", 0)?;
        let price = data.csiget(2, "ld merchant price", u32::MAX)?;
        Ok(Some(Self {
            typ,
            max_uses: s / 10_000,
            strength: s % 10_000,
            keys: price,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub struct TotalStats {
    
    pub legendaries_found: u32,
    
    pub attempts_best_run: u32,
    
    pub enemies_defeated: u32,
    
    pub epics_found: u32,
    
    pub gold_found: u64,
}

impl TotalStats {
    pub(crate) fn parse(data: &[i64]) -> Result<Self, SFError> {
        
        Ok(TotalStats {
            legendaries_found: data.csiget(0, "ld total legendaries", 0)?,
            attempts_best_run: data.csiget(1, "ld best attempts", 0)?,
            enemies_defeated: data.csiget(2, "ld enemies defeated", 0)?,
            epics_found: data.csiget(3, "ld total epics", 0)?,
            gold_found: data.csiget(4, "ld total gold", 0)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]


pub struct GemOfFate {
    
    pub typ: GemOfFateType,
    
    pub advantage: Option<GemOfFateEffect>,
    
    pub advantage_pwr: i64,
    
    pub disadvantage: Option<GemOfFateEffect>,
    
    pub disadvantage_pwr: i64,
    
    pub disadvantage_effect: Option<GemOfFateSpecialDisadvantage>,
}

impl GemOfFate {
    pub(crate) fn parse(data: &[i64]) -> Result<Option<GemOfFate>, SFError> {
        if data.iter().all(|a| *a == 0) {
            return Ok(None);
        }
        Ok(Some(Self {
            typ: data.cfpget(0, "ld gof typ", |a| a)?.unwrap_or_default(),
            advantage: data.cfpget(1, "ld gof adv", |a| a)?,
            advantage_pwr: data.cget(2, "ld gof dis val")?,
            disadvantage: data.cfpget(3, "ld gof dis", |a| a)?,
            disadvantage_pwr: data.cget(4, "ld gof dis val")?,
            disadvantage_effect: data.cfpget(5, "ld gof dis effect", |a| a)?,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum GemOfFateType {
    
    EyeOfTheBull = 1,
    
    SoulOfTheRabbit = 2,
    
    BoulderOfGreed = 3,
    
    EmeraldOfTheExplorer = 4,
    
    PearlOfTheMasochist = 5,
    
    PendantOfTheKeyMaster = 6,
    
    PebbleOfDeceit = 7,
    
    GreasyHealingStone = 8,
    
    SpyingGem = 9,
    
    LodeStone = 10,
    
    BoulderOfTheGambler = 11,
    
    OldSacrificialStone = 12,
    
    BloodDropOfSacrifice = 13,
    
    KidneyStoneOfDetermination = 14,
    
    HopeOfTheThirstyOne = 15,
    
    ErraticBoulderOfTheHip = 16,
    
    SaphireOfTheMisadventurer = 17,
    
    CursedMoonstone = 18,
    
    DiamondOfTheTimetraveler = 19,
    
    TreasureOfTheHero = 20,
    
    CrownJewelOfTheDevil = 21,
    
    CursedPearl = 22,
    
    RustyHealingStone = 23,

    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum GemOfFateEffect {
    
    
    ChanceOfKeys = 1,

    
    
    EscapeChance = 10,
    
    DamageFromEscape = 11,
    
    ChanceOfKeyAfterEscape = 12,
    
    ChanceOfCurseAfterEscape = 13,

    
    
    DurationOfBlessings = 30,
    
    DurationOfCurses = 31,
    
    ChanceOfStrongerCurses = 32,

    
    
    DamageFromMonsters = 40,
    
    ChanceOfBlessingAfterFight = 41,
    
    ChanceOfCurseAfterFight = 42,

    
    
    ChanceOfBlessingsInBarrels = 50,
    
    ChanceOfBetterBlessingsInBarrels = 51,

    
    
    DamageFromSacDoors = 70,
    
    DamageFromChests = 71,

    
    DamageFromTraps = 90,

    
    BlessingOrCurseAfterRevive = 100,
    
    BlessingsInBarrelsChestsCorpses = 110,
    
    HealingFromBlessings = 130,

    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum GemOfFateSpecialDisadvantage {
    
    WeakerMonstersSpawn = 1,
    
    StrongerMonstersSpawn = 2,
    
    MoreTrapsSpawn = 3,
    
    
    SacChestsSpawnBehindClosedDoors = 6,

    
    MoreSacDoors = 8,
    
    FewerSacDoors = 9,
    
    CursedChestsSpawnBehindClosedDoors = 10,

    
    MoreCursedDoors = 12,
    
    FewerCursedDoors = 13,

    
    
    ChanceOfEpicDoors = 17,
    
    ChanceOfUnlockedDoors = 18,
    
    ChanceOfDoubleLockedDoor = 19,

    
    MoreMysteriousRooms = 20,
    
    FewerMysteriousRooms = 21,
    
    AlwaysOneTrap = 22,
    
    AlwaysOneLock = 23,
    
    MonstersBehindDoors = 24,
    
    NoMoreEpicChests = 25,
    
    TrapsInflictCurse = 26,

    
    #[default]
    Unknown = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum RPSChoice {
    
    Rock = 90,
    
    Paper = 91,
    
    Scissors = 92,
}
