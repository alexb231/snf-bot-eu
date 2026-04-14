use std::collections::HashMap;

use chrono::{DateTime, Local};
use enum_map::EnumMap;
use log::warn;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use super::{
    AttributeType, Class, Emblem, Flag, Item, Potion, Race, Reward, SFError,
    ServerTime,
    character::{Mount, Portrait},
    guild::GuildRank,
    items::Equipment,
};
use crate::{PlayerId, misc::*};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Mail {
    
    pub combat_log: Vec<CombatLogEntry>,
    
    pub inbox_capacity: u16,
    
    pub inbox: Vec<InboxEntry>,
    
    pub claimables: Vec<ClaimableMail>,
    
    
    pub open_msg: Option<String>,
    
    
    pub open_claimable: Option<ClaimablePreview>,
}



#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HallOfFames {
    
    pub players_total: u32,
    
    pub players: Vec<HallOfFamePlayer>,

    
    
    pub guilds_total: Option<u32>,
    
    pub guilds: Vec<HallOfFameGuild>,

    
    
    pub fortresses_total: Option<u32>,
    
    pub fortresses: Vec<HallOfFameFortress>,

    
    
    pub pets_total: Option<u32>,
    
    pub pets: Vec<HallOfFamePets>,

    pub hellevator_total: Option<u32>,
    pub hellevator: Vec<HallOfFameHellevator>,

    
    
    pub underworlds_total: Option<u32>,
    
    pub underworlds: Vec<HallOfFameUnderworld>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HallOfFameHellevator {
    pub rank: usize,
    pub name: String,
    pub tokens: u64,
}



#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lookup {
    
    
    players: HashMap<PlayerId, OtherPlayer>,
    name_to_id: HashMap<String, PlayerId>,

    
    pub guilds: HashMap<String, OtherGuild>,
}

impl Lookup {
    pub(crate) fn insert_lookup(&mut self, other: OtherPlayer) {
        if other.name.is_empty() || other.player_id == 0 {
            warn!("Skipping invalid player insert");
            return;
        }
        self.name_to_id.insert(other.name.clone(), other.player_id);
        self.players.insert(other.player_id, other);
    }

    
    #[must_use]
    pub fn lookup_pid(&self, pid: PlayerId) -> Option<&OtherPlayer> {
        self.players.get(&pid)
    }

    
    #[must_use]
    pub fn lookup_name(&self, name: &str) -> Option<&OtherPlayer> {
        let other_pos = self.name_to_id.get(name)?;
        self.players.get(other_pos)
    }

    
    #[allow(clippy::must_use_unit)]
    pub fn remove_pid(&mut self, pid: PlayerId) -> Option<OtherPlayer> {
        self.players.remove(&pid)
    }

    
    #[allow(clippy::must_use_unit)]
    pub fn remove_name(&mut self, name: &str) -> Option<OtherPlayer> {
        let other_pos = self.name_to_id.remove(name)?;
        self.players.remove(&other_pos)
    }

    
    pub fn reset_lookups(&mut self) {
        self.players = HashMap::default();
        self.name_to_id = HashMap::default();
    }
}



#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HallOfFamePlayer {
    
    pub rank: u32,
    
    pub name: String,
    
    
    pub guild: Option<String>,
    
    pub level: u32,
    
    pub honor: u32,
    
    pub class: Class,
    
    pub flag: Option<Flag>,
}

impl HallOfFamePlayer {
    pub(crate) fn parse(val: &str) -> Result<Self, SFError> {
        let data: Vec<_> = val.split(',').collect();
        let rank = data.cfsuget(0, "hof player rank")?;
        let name = data.cget(1, "hof player name")?.to_string();
        let guild = Some(data.cget(2, "hof player guild")?.to_string())
            .filter(|a| !a.is_empty());
        let level = data.cfsuget(3, "hof player level")?;
        let honor = data.cfsuget(4, "hof player fame")?;
        let class: i64 = data.cfsuget(5, "hof player class")?;
        let Some(class) = FromPrimitive::from_i64(class - 1) else {
            warn!("Invalid hof class: {class} - {data:?}");
            return Err(SFError::ParsingError(
                "hof player class",
                class.to_string(),
            ));
        };

        let raw_flag = data.get(6).copied().unwrap_or_default();
        let flag = Flag::parse(raw_flag);

        Ok(HallOfFamePlayer {
            rank,
            name,
            guild,
            level,
            honor,
            class,
            flag,
        })
    }
}



#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HallOfFameGuild {
    
    pub name: String,
    
    pub rank: u32,
    
    pub leader: String,
    
    pub member_count: u32,
    
    pub honor: u32,
    
    pub is_attacked: bool,
}

impl HallOfFameGuild {
    pub(crate) fn parse(val: &str) -> Result<Self, SFError> {
        let data: Vec<_> = val.split(',').collect();
        let rank = data.cfsuget(0, "hof guild rank")?;
        let name = data.cget(1, "hof guild name")?.to_string();
        let leader = data.cget(2, "hof guild leader")?.to_string();
        let member = data.cfsuget(3, "hof guild member")?;
        let honor = data.cfsuget(4, "hof guild fame")?;
        let attack_status: u8 = data.cfsuget(5, "hof guild atk")?;

        Ok(HallOfFameGuild {
            rank,
            name,
            leader,
            member_count: member,
            honor,
            is_attacked: attack_status == 1u8,
        })
    }
}

impl HallOfFamePets {
    pub(crate) fn parse(val: &str) -> Result<Self, SFError> {
        let data: Vec<_> = val.split(',').collect();
        let rank = data.cfsuget(0, "hof pet rank")?;
        let name = data.cget(1, "hof pet player")?.to_string();
        let guild = Some(data.cget(2, "hof pet guild")?.to_string())
            .filter(|a| !a.is_empty());
        let collected = data.cfsuget(3, "hof pets collected")?;
        let honor = data.cfsuget(4, "hof pets fame")?;
        let unknown = data.cfsuget(5, "hof pets uk")?;

        Ok(HallOfFamePets {
            name,
            rank,
            guild,
            collected,
            honor,
            unknown,
        })
    }
}

impl HallOfFameFortress {
    pub(crate) fn parse(val: &str) -> Result<Self, SFError> {
        let data: Vec<_> = val.split(',').collect();
        let rank = data.cfsuget(0, "hof ft rank")?;
        let name = data.cget(1, "hof ft player")?.to_string();
        let guild = Some(data.cget(2, "hof ft guild")?.to_string())
            .filter(|a| !a.is_empty());
        let upgrade = data.cfsuget(3, "hof ft collected")?;
        let honor = data.cfsuget(4, "hof ft fame")?;

        Ok(HallOfFameFortress {
            name,
            rank,
            guild,
            upgrade,
            honor,
        })
    }
}

impl HallOfFameUnderworld {
    pub(crate) fn parse(val: &str) -> Result<Self, SFError> {
        let data: Vec<_> = val.split(',').collect();
        let rank = data.cfsuget(0, "hof ft rank")?;
        let name = data.cget(1, "hof ft player")?.to_string();
        let guild = Some(data.cget(2, "hof ft guild")?.to_string())
            .filter(|a| !a.is_empty());
        let upgrade = data.cfsuget(3, "hof ft collected")?;
        let honor = data.cfsuget(4, "hof ft fame")?;
        let unknown = data.cfsuget(5, "hof pets uk")?;

        Ok(HallOfFameUnderworld {
            rank,
            name,
            guild,
            upgrade,
            honor,
            unknown,
        })
    }
}


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HallOfFameFortress {
    
    pub name: String,
    
    pub rank: u32,
    
    
    pub guild: Option<String>,
    
    pub upgrade: u32,
    
    pub honor: u32,
}


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HallOfFamePets {
    
    pub name: String,
    
    pub rank: u32,
    
    
    pub guild: Option<String>,
    
    pub collected: u32,
    
    pub honor: u32,
    
    
    pub unknown: i64,
}


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HallOfFameUnderworld {
    
    pub rank: u32,
    
    pub name: String,
    
    
    pub guild: Option<String>,
    
    pub upgrade: u32,
    
    pub honor: u32,
    
    
    pub unknown: i64,
}



#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OtherPlayer {
    
    
    pub player_id: PlayerId,
    
    pub name: String,
    
    pub level: u16,
    
    pub description: String,
    
    pub guild: Option<String>,
    
    #[deprecated = "v29.500 overhauled the parsing of normal & other players. \
                    This field is not longer available in the new data. As \
                    such, this field may become unavailable at any point, \
                    once the old data is on longer served by the server"]
    pub guild_joined: Option<DateTime<Local>>,
    
    pub mount: Option<Mount>,
    
    pub mount_end: Option<DateTime<Local>>,
    
    pub portrait: Portrait,
    
    pub relationship: Relationship,
    
    pub wall_combat_lvl: u16,
    
    pub equipment: Equipment,

    pub experience: u64,
    pub next_level_xp: u64,

    pub honor: u32,
    pub rank: u32,
    
    pub portal_hp_bonus: u32,
    
    pub portal_dmg_bonus: u32,
    
    
    pub attribute_basis: EnumMap<AttributeType, u32>,
    
    pub attribute_additions: EnumMap<AttributeType, u32>,
    
    pub attribute_times_bought: EnumMap<AttributeType, u32>,
    
    pub attribute_pet_bonus: EnumMap<AttributeType, u32>,
    
    pub class: Class,
    
    pub race: Race,
    
    pub scrapbook_count: Option<u32>,
    
    pub active_potions: [Option<Potion>; 3],
    
    pub armor: u64,
    
    pub min_damage: u32,
    
    pub max_damage: u32,
    
    pub fortress: Option<OtherFortress>,
    
    pub gladiator_lvl: u32,
    
    pub is_vip: bool,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OtherFortress {
    
    pub upgrade_count: u32,
    
    
    pub soldier_advice: u16,
    
    
    pub lootable_wood: u64,
    
    
    pub lootable_stone: u64,
    
    pub archer_count: u16,
    
    pub mage_count: u16,
    
    pub rank: u32,
}

#[derive(Debug, Default, Clone, FromPrimitive, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Relationship {
    #[default]
    Ignored = -1,
    Normal = 0,
    Friend = 1,
}

impl OtherPlayer {
    pub(crate) fn update_pet_bonus(
        &mut self,
        data: &[u32],
    ) -> Result<(), SFError> {
        let atr = &mut self.attribute_pet_bonus;
        
        
        *atr.get_mut(AttributeType::Constitution) = data.cget(1, "pet con")?;
        *atr.get_mut(AttributeType::Dexterity) = data.cget(2, "pet dex")?;
        *atr.get_mut(AttributeType::Intelligence) = data.cget(3, "pet int")?;
        *atr.get_mut(AttributeType::Luck) = data.cget(4, "pet luck")?;
        *atr.get_mut(AttributeType::Strength) = data.cget(5, "pet str")?;
        Ok(())
    }

    pub(crate) fn update_fortress(
        &mut self,
        data: &[i64],
    ) -> Result<(), SFError> {
        let ft = self.fortress.get_or_insert_default();
        ft.upgrade_count = data.csiget(0, "other ft upgrades", 0)?;
        ft.soldier_advice = data.csiget(1, "other soldier advice", 0)?;
        ft.mage_count = data.csiget(2, "other mage count", 0)?;
        ft.archer_count = data.csiget(3, "other soldier advice", 0)?;
        ft.lootable_wood = data.csiget(4, "other lootable wood", 0)?;
        ft.lootable_stone = data.csiget(5, "other lootable stone", 0)?;
        Ok(())
    }

    pub(crate) fn update(
        &mut self,
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        
        self.player_id = data.csiget(1, "player id", 0)?;
        
        self.level = data.csimget(3, "level", 0, |a| a & 0xFFFF)?;
        self.experience = data.csiget(4, "experience", 0)?;
        self.next_level_xp = data.csiget(5, "xp to next lvl", 0)?;
        self.honor = data.csiget(6, "honor", 0)?;
        self.rank = data.csiget(7, "rank", 0)?;
        self.portrait =
            Portrait::parse(data.skip(8, "portrait")?).unwrap_or_default();
        
        
        
        
        
        
        
        
        
        
        
        self.race = data.cfpuget(18, "char race", |a| a)?;
        
        
        self.class = data.cfpuget(20, "character class", |a| a - 1)?;
        self.mount = data.cfpget(21, "character mount", |a| a & 0xFF)?;
        
        
        self.armor = data.csiget(23, "total armor", 0)?;
        self.min_damage = data.csiget(24, "min damage", 0)?;
        self.max_damage = data.csiget(25, "max damage", 0)?;
        self.portal_dmg_bonus = data.cimget(26, "portal dmg bonus", |a| a)?;
        
        self.portal_hp_bonus = data.csimget(28, "portal hp bonus", 0, |a| a)?;
        self.mount_end = data.cstget(29, "mount end", server_time)?;
        update_enum_map(
            &mut self.attribute_basis,
            data.skip(30, "char attr basis")?,
        );
        update_enum_map(
            &mut self.attribute_additions,
            data.skip(35, "char attr adds")?,
        );
        update_enum_map(
            &mut self.attribute_times_bought,
            data.skip(40, "char attr tb")?,
        );
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        

        
        let sb_count = data.cget(66, "scrapbook count")?;
        if sb_count >= 10000 {
            self.scrapbook_count =
                Some(soft_into(sb_count - 10000, "scrapbook count", 0));
        }
        
        
        self.gladiator_lvl = data.csiget(69, "gladiator lvl", 0)?;

        Ok(())
    }
}

#[derive(Debug, Clone, FromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CombatMessageType {
    Arena = 0,
    Quest = 1,
    GuildFight = 2,
    GuildRaid = 3,
    Dungeon = 4,
    TowerFight = 5,
    LostFight = 6,
    WonFight = 7,
    FortressFight = 8,
    FortressDefense = 9,
    ShadowWorld = 12,
    FortressDefenseAlreadyCountered = 109,
    PetAttack = 14,
    PetDefense = 15,
    Underworld = 16,
    Twister = 25,
    GuildFightLost = 26,
    GuildFightWon = 27,
}

#[derive(Debug, Clone, FromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MessageType {
    Normal,
    GuildInvite,
    GuildKicked,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CombatLogEntry {
    pub msg_id: i64,
    pub player_name: String,
    pub won: bool,
    pub battle_type: CombatMessageType,
    pub time: DateTime<Local>,
}

impl CombatLogEntry {
    pub(crate) fn parse(
        data: &[&str],
        server_time: ServerTime,
    ) -> Result<CombatLogEntry, SFError> {
        let msg_id = data.cfsuget(0, "combat msg_id")?;
        let battle_t: i64 = data.cfsuget(3, "battle t")?;
        let time_stamp: i64 = data.cfsuget(4, "combat log time")?;
        let time = server_time
            .convert_to_local(time_stamp, "combat time")
            .ok_or_else(|| {
                SFError::ParsingError("combat time", time_stamp.to_string())
            })?;

        let mt = FromPrimitive::from_i64(battle_t).ok_or_else(|| {
            SFError::ParsingError("combat mt", format!("{battle_t} @ {time:?}"))
        })?;

        Ok(CombatLogEntry {
            msg_id,
            player_name: data.cget(1, "clog player")?.to_string(),
            won: data.cget(2, "clog won")? == "1",
            battle_type: mt,
            time,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InboxEntry {
    pub msg_typ: MessageType,
    pub from: String,
    pub msg_id: i32,
    pub title: String,
    pub date: DateTime<Local>,
    pub read: bool,
}

impl InboxEntry {
    pub(crate) fn parse(
        msg: &str,
        server_time: ServerTime,
    ) -> Result<InboxEntry, SFError> {
        let parts = msg.splitn(4, ',').collect::<Vec<_>>();
        let Some((title, date)) =
            parts.cget(3, "msg title/date")?.rsplit_once(',')
        else {
            return Err(SFError::ParsingError(
                "title/msg comma",
                msg.to_string(),
            ));
        };

        let msg_typ = match title {
            "3" => MessageType::GuildKicked,
            "5" => MessageType::GuildInvite,
            x if x.chars().all(|a| a.is_ascii_digit()) => {
                return Err(SFError::ParsingError(
                    "msg typ",
                    title.to_string(),
                ));
            }
            _ => MessageType::Normal,
        };

        let Some(date) = date
            .parse()
            .ok()
            .and_then(|a| server_time.convert_to_local(a, "msg_date"))
        else {
            return Err(SFError::ParsingError("msg date", date.to_string()));
        };

        Ok(InboxEntry {
            msg_typ,
            date,
            from: parts.cget(1, "inbox from")?.to_string(),
            msg_id: parts.cfsuget(0, "msg_id")?,
            title: from_sf_string(title.trim_end_matches('\t')),
            read: parts.cget(2, "inbox read")? == "1",
        })
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OtherGuild {
    pub name: String,

    pub attacks: Option<String>,
    pub defends_against: Option<String>,

    pub rank: u16,
    pub attack_cost: u32,
    pub description: String,
    pub emblem: Emblem,
    pub honor: u32,
    pub finished_raids: u16,
    
    member_count: u8,
    pub members: Vec<OtherGuildMember>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OtherGuildMember {
    pub name: String,
    pub instructor_lvl: u16,
    pub treasure_lvl: u16,
    pub rank: GuildRank,
    pub level: u16,
    pub pet_lvl: u16,
    pub last_active: Option<DateTime<Local>>,
}
impl OtherGuild {
    pub(crate) fn update(
        &mut self,
        val: &str,
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        let data: Vec<_> = val
            .split('/')
            .map(|c| c.trim().parse::<i64>().unwrap_or_default())
            .collect();

        self.member_count = data.csiget(3, "member count", 0)?;
        let member_count = self.member_count as usize;
        self.finished_raids = data.csiget(8, "raid count", 0)?;
        self.honor = data.csiget(13, "other guild honor", 0)?;

        self.members.resize_with(member_count, Default::default);

        for (i, member) in &mut self.members.iter_mut().enumerate() {
            member.level =
                data.csiget(64 + i, "other guild member level", 0)?;
            member.last_active =
                data.cstget(114 + i, "other guild member active", server_time)?;
            member.treasure_lvl =
                data.csiget(214 + i, "other guild member treasure levels", 0)?;
            member.instructor_lvl = data.csiget(
                264 + i,
                "other guild member instructor levels",
                0,
            )?;
            member.rank = data
                .cfpget(314 + i, "other guild member ranks", |q| q)?
                .unwrap_or_default();
            member.pet_lvl =
                data.csiget(390 + i, "other guild pet levels", 0)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RelationEntry {
    pub id: PlayerId,
    pub name: String,
    pub guild: String,
    pub level: u16,
    pub relation: Relationship,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClaimableMail {
    pub msg_id: i64,
    pub typ: ClaimableMailType,
    pub status: ClaimableStatus,
    pub name: String,
    pub received: Option<DateTime<Local>>,
    pub claimable_until: Option<DateTime<Local>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ClaimableStatus {
    Unread,
    Read,
    Claimed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, FromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ClaimableMailType {
    Coupon = 10,
    SupermanDelivery = 11,
    TwitchDrop = 12,
    #[default]
    GenericDelivery,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClaimablePreview {
    pub items: Vec<Item>,
    pub resources: Vec<Reward>,
}
