#![allow(clippy::module_name_repetitions)]
use chrono::{DateTime, Local, NaiveTime};
use enum_map::EnumMap;
use log::warn;
use num_derive::FromPrimitive;

use super::{
    items::{ItemType, PotionSize, PotionType},
    update_enum_map, ArrSkip, AttributeType, CCGet, CFPGet, CGet, CSTGet,
    NormalCost, Potion, SFError, ServerTime,
};
use crate::misc::{from_sf_string, soft_into, warning_parse};


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Guild {
    
    pub id: u32,
    
    pub name: String,
    
    pub description: String,
    
    
    pub emblem: Emblem,

    
    pub honor: u32,
    
    pub rank: u32,
    
    pub joined: Option<DateTime<Local>>,

    
    pub own_treasure_skill: u16,
    
    pub own_treasure_upgrade: NormalCost,
    
    pub total_treasure_skill: u16,
    
    pub own_instructor_skill: u16,
    
    pub own_instructor_upgrade: NormalCost,

    
    pub total_instructor_skill: u16,

    
    pub finished_raids: u16,

    
    
    pub defending: Option<PlanedBattle>,
    
    
    pub attacking: Option<PlanedBattle>,
    
    pub next_attack_possible: Option<DateTime<Local>>,

    
    pub pet_id: u32,
    
    pub own_pet_lvl: u16,
    
    
    pub pet_max_lvl: u16,
    
    pub hydra: GuildHydra,
    
    pub portal: GuildPortal,

    
    
    member_count: u8,
    
    pub members: Vec<GuildMemberData>,
    
    pub chat: Vec<ChatMessage>,
    
    pub whispers: Vec<ChatMessage>,
    
    
    pub fightable_guilds: Vec<FightableGuild>,
}


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GuildHydra {
    
    pub last_battle: Option<DateTime<Local>>,
    
    pub last_full: Option<DateTime<Local>>,
    
    
    pub next_battle: Option<DateTime<Local>>,
    
    pub remaining_fights: u16,
    
    pub current_life: u64,
    
    pub max_life: u64,
    
    pub attributes: EnumMap<AttributeType, u32>,
}



#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FightableGuild {
    
    pub id: u32,
    
    pub name: String,
    
    pub emblem: Emblem,
    
    pub number_of_members: u8,
    
    pub members_min_level: u32,
    
    pub members_max_level: u32,
    
    pub members_average_level: u32,
    
    pub rank: u32,
    
    pub honor: u32,
}


#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Emblem {
    raw: String,
}

impl Emblem {
    
    #[must_use]
    pub fn server_encode(&self) -> String {
        
        self.raw.clone()
    }

    pub(crate) fn update(&mut self, str: &str) {
        self.raw.clear();
        self.raw.push_str(str);
    }
}


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChatMessage {
    
    
    pub user: String,
    
    
    pub time: NaiveTime,
    
    pub message: String,
}

impl ChatMessage {
    pub(crate) fn parse_messages(data: &str) -> Vec<ChatMessage> {
        data.split('/')
            .filter_map(|msg| {
                let (time, rest) = msg.split_once(' ')?;
                let (name, msg) = rest.split_once(':')?;
                let msg = from_sf_string(msg.trim_start_matches(['§', ' ']));
                let time = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
                Some(ChatMessage {
                    user: name.to_string(),
                    time,
                    message: msg,
                })
            })
            .collect()
    }
}

impl Guild {
    pub(crate) fn update_group_save(
        &mut self,
        val: &str,
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        let data: Vec<_> = val
            .split('/')
            .map(|c| c.trim().parse::<i64>().unwrap_or_default())
            .collect();

        let member_count = data.csiget(3, "member count", 0)?;
        self.member_count = member_count;
        self.members
            .resize_with(member_count as usize, Default::default);

        for (offset, member) in self.members.iter_mut().enumerate() {
            member.battles_joined =
                data.cfpget(445 + offset, "member fights joined", |x| x % 100)?;
            member.level = data.csiget(64 + offset, "member level", 0)?;
            member.last_online =
                data.cstget(114 + offset, "member last online", server_time)?;
            member.treasure_skill =
                data.csiget(214 + offset, "member treasure skill", 0)?;
            member.instructor_skill =
                data.csiget(264 + offset, "member master skill", 0)?;
            member.guild_rank = match data.cget(314 + offset, "member rank")? {
                1 => GuildRank::Leader,
                2 => GuildRank::Officer,
                3 => GuildRank::Member,
                4 => GuildRank::Invited,
                x => {
                    warn!("Unknown guild rank: {x}");
                    GuildRank::Invited
                }
            };
            member.portal_fought =
                data.cstget(164 + offset, "member portal fought", server_time)?;
            member.guild_pet_lvl =
                data.csiget(390 + offset, "member pet skill", 0)?;
        }

        self.honor = data.csiget(13, "guild honor", 0)?;
        self.id = data.csiget(0, "guild id", 0)?;

        self.finished_raids = data.csiget(8, "finished raids", 0)?;

        self.attacking = PlanedBattle::parse(
            data.skip(364, "attacking guild")?,
            server_time,
        )?;

        self.defending = PlanedBattle::parse(
            data.skip(366, "attacking guild")?,
            server_time,
        )?;

        self.next_attack_possible =
            data.cstget(365, "guild next attack time", server_time)?;

        self.pet_id = data.csiget(377, "gpet id", 0)?;
        self.pet_max_lvl = data.csiget(378, "gpet max lvl", 0)?;

        self.hydra.last_battle =
            data.cstget(382, "hydra pet lb", server_time)?;
        self.hydra.last_full =
            data.cstget(381, "hydra last defeat", server_time)?;

        self.hydra.current_life = data.csiget(383, "ghydra clife", u64::MAX)?;
        self.hydra.max_life = data.csiget(384, "ghydra max clife", u64::MAX)?;

        update_enum_map(
            &mut self.hydra.attributes,
            data.skip(385, "hydra attributes")?,
        );
        self.total_treasure_skill =
            data.csimget(6, "guild total treasure skill", 0, |x| x & 0xFFFF)?;
        self.total_instructor_skill =
            data.csimget(7, "guild total instructor skill", 0, |x| x & 0xFFFF)?;
        self.portal.life_percentage =
            data.csimget(6, "guild portal life p", 100, |x| x >> 16)?;
        self.portal.defeated_count =
            data.csimget(7, "guild portal progress", 0, |x| x >> 16)?;
        Ok(())
    }

    pub(crate) fn update_member_names(&mut self, val: &str) {
        let names: Vec<_> = val
            .split(',')
            .map(std::string::ToString::to_string)
            .collect();
        self.members.resize_with(names.len(), Default::default);
        for (member, name) in self.members.iter_mut().zip(names) {
            member.name = name;
        }
    }

    pub(crate) fn update_group_knights(&mut self, val: &str) {
        let data: Vec<i64> = val
            .trim_end_matches(',')
            .split(',')
            .flat_map(str::parse)
            .collect();

        self.members.resize_with(data.len(), Default::default);
        for (member, count) in self.members.iter_mut().zip(data) {
            member.knights = soft_into(count, "guild knight", 0);
        }
    }

    pub(crate) fn update_member_potions(&mut self, val: &str) {
        let data = val
            .trim_end_matches(',')
            .split(',')
            .map(|c| {
                warning_parse(c, "member potion", |a| a.parse::<i64>().ok())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        let potions = data.len() / 2;
        let member = potions / 3;
        self.members.resize_with(member, Default::default);

        let mut data = data.into_iter();

        let quick_potion = |int: i64| {
            Some(ItemType::Potion(Potion {
                typ: PotionType::parse(int)?,
                size: PotionSize::parse(int)?,
                expires: None,
            }))
        };

        for member in &mut self.members {
            for potion in &mut member.potions {
                *potion = data
                    .next()
                    .or_else(|| {
                        warn!("Invalid member potion len");
                        None
                    })
                    .and_then(quick_potion);
                _ = data.next();
            }
        }
    }

    pub(crate) fn update_description_embed(&mut self, data: &str) {
        let Some((emblem, description)) = data.split_once('§') else {
            self.description = from_sf_string(data);
            return;
        };

        self.description = from_sf_string(description);
        self.emblem.update(emblem);
    }

    pub(crate) fn update_group_prices(
        &mut self,
        data: &[i64],
    ) -> Result<(), SFError> {
        self.own_treasure_upgrade.silver =
            data.csiget(0, "treasure upgr. silver", 0)?;
        self.own_treasure_upgrade.mushrooms =
            data.csiget(1, "treasure upgr. mush", 0)?;
        self.own_instructor_upgrade.silver =
            data.csiget(2, "instr upgr. silver", 0)?;
        self.own_instructor_upgrade.mushrooms =
            data.csiget(3, "instr upgr. mush", 0)?;
        Ok(())
    }

    #[allow(clippy::indexing_slicing)]
    pub(crate) fn update_fightable_targets(
        &mut self,
        data: &str,
    ) -> Result<(), SFError> {
        const SIZE: usize = 9;

        
        self.fightable_guilds.clear();

        let entries = data.trim_end_matches('/').split('/').collect::<Vec<_>>();

        let target_counts = entries.len() / SIZE;

        
        if target_counts * SIZE != entries.len() {
            warn!("Invalid fightable targets len");
            return Err(SFError::ParsingError(
                "Fightable targets invalid length",
                data.to_string(),
            ));
        }

        
        self.fightable_guilds.reserve(entries.len() / SIZE);

        for i in 0..entries.len() / SIZE {
            let offset = i * SIZE;

            self.fightable_guilds.push(FightableGuild {
                id: entries[offset].parse().unwrap_or_default(),
                name: from_sf_string(entries[offset + 1]),
                emblem: Emblem {
                    raw: entries[offset + 2].to_string(),
                },
                number_of_members: entries[offset + 3]
                    .parse()
                    .unwrap_or_default(),
                members_min_level: entries[offset + 4]
                    .parse()
                    .unwrap_or_default(),
                members_max_level: entries[offset + 5]
                    .parse()
                    .unwrap_or_default(),
                members_average_level: entries[offset + 6]
                    .parse()
                    .unwrap_or_default(),
                rank: entries[offset + 7].parse().unwrap_or_default(),
                honor: entries[offset + 8].parse().unwrap_or_default(),
            });
        }

        Ok(())
    }
}


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlanedBattle {
    
    pub other: u32,
    
    pub date: DateTime<Local>,
}

impl PlanedBattle {
    
    #[must_use]
    pub fn is_raid(&self) -> bool {
        self.other == 1_000_000
    }

    #[allow(clippy::similar_names)]
    fn parse(
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<Option<Self>, SFError> {
        let other = data.cget(0, "gbattle other")?;
        let other = match other.try_into() {
            Ok(x) if x > 1 => Some(x),
            _ => None,
        };
        let date = data.cget(1, "gbattle time")?;
        let date = server_time.convert_to_local(date, "next guild fight");
        Ok(match (other, date) {
            (Some(other), Some(date)) => Some(Self { other, date }),
            _ => None,
        })
    }
}


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GuildPortal {
    
    pub damage_bonus: u8,
    
    
    pub defeated_count: u8,
    
    pub life_percentage: u8,
}

#[derive(Debug, PartialEq, Copy, Clone, FromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BattlesJoined {
    
    Defense = 1,
    
    Attack = 10,
    
    
    Both = 11,
}


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GuildMemberData {
    
    pub name: String,
    
    pub battles_joined: Option<BattlesJoined>,
    
    pub level: u16,
    
    
    pub last_online: Option<DateTime<Local>>,
    
    pub treasure_skill: u16,
    
    pub instructor_skill: u16,
    
    pub guild_pet_lvl: u16,

    
    pub guild_rank: GuildRank,
    
    
    pub portal_fought: Option<DateTime<Local>>,
    
    
    
    pub potions: [Option<ItemType>; 3],
    
    pub knights: u8,
}


#[derive(Debug, Clone, Copy, FromPrimitive, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum GuildRank {
    Leader = 1,
    Officer = 2,
    #[default]
    Member = 3,
    Invited = 4,
}


#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum GuildSkill {
    Treasure = 0,
    Instructor,
    Pet,
}
