use chrono::{DateTime, Local};
use num_traits::FromPrimitive;

use super::{items::*, *};
use crate::PlayerId;


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Arena {
    
    
    pub enemy_ids: [PlayerId; 3],
    
    pub next_free_fight: Option<DateTime<Local>>,
    
    
    pub fights_for_xp: u8,
}



#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fight {
    
    
    pub group_attacker_name: Option<String>,
    
    pub group_attacker_id: Option<u32>,

    
    
    pub group_defender_name: Option<String>,
    
    pub group_defender_id: Option<u32>,

    
    
    pub fights: Vec<SingleFight>,
    
    pub has_player_won: bool,
    
    pub silver_change: i64,
    
    pub xp_change: u64,
    
    pub mushroom_change: u8,
    
    pub honor_change: i64,
    
    pub rank_pre_fight: u32,
    
    pub rank_post_fight: u32,
    
    pub item_won: Option<Item>,
}

impl Fight {
    pub(crate) fn update_result(
        &mut self,
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        self.has_player_won = data.cget(0, "has_player_won")? != 0;
        self.silver_change = data.cget(2, "fight silver change")?;

        if data.len() < 20 {
            
            return Ok(());
        }

        self.xp_change = data.csiget(3, "fight xp", 0)?;
        self.mushroom_change = data.csiget(4, "fight mushrooms", 0)?;
        self.honor_change = data.cget(5, "fight honor")?;

        self.rank_pre_fight = data.csiget(7, "fight rank pre", 0)?;
        self.rank_post_fight = data.csiget(8, "fight rank post", 0)?;
        let item = data.skip(9, "fight item")?;
        self.item_won = Item::parse(item, server_time)?;
        Ok(())
    }

    pub(crate) fn update_groups(&mut self, val: &str) {
        let mut groups = val.split(',');

        let (Some(aid), Some(did), Some(aname), Some(dname)) = (
            groups.next().and_then(|a| a.parse().ok()),
            groups.next().and_then(|a| a.parse().ok()),
            groups.next(),
            groups.next(),
        ) else {
            warn!("Invalid fight group: {val}");
            return;
        };

        self.group_attacker_id = Some(aid);
        self.group_defender_id = Some(did);
        self.group_attacker_name = Some(aname.to_string());
        self.group_defender_name = Some(dname.to_string());
    }
}



#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SingleFight {
    
    pub winner_id: PlayerId,
    
    
    pub fighter_a: Option<Fighter>,
    
    pub fighter_b: Option<Fighter>,
    
    
    pub actions: Vec<FightAction>,
}

impl SingleFight {
    pub(crate) fn update_fighters(&mut self, data: &str) {
        let data = data.split('/').collect::<Vec<_>>();
        if data.len() < 60 {
            self.fighter_a = None;
            self.fighter_b = None;
            warn!("Fighter response too short");
            return;
        }
        
        let (fighter_a, fighter_b) = data.split_at(47);
        self.fighter_a = Fighter::parse(fighter_a);
        self.fighter_b = Fighter::parse(fighter_b);
    }

    pub(crate) fn update_rounds(
        &mut self,
        data: &str,
        fight_version: u32,
    ) -> Result<(), SFError> {
        self.actions.clear();

        if fight_version > 1 {
            
            return Ok(());
        }
        let mut iter = data.split(',');
        while let (Some(player_id), Some(damage_typ), Some(new_life)) =
            (iter.next(), iter.next(), iter.next())
        {
            let action =
                warning_from_str(damage_typ, "fight action").unwrap_or(0);

            self.actions.push(FightAction {
                acting_id: player_id.parse().map_err(|_| {
                    SFError::ParsingError("action pid", player_id.to_string())
                })?,
                action: FightActionType::parse(action),
                other_new_life: new_life.parse().map_err(|_| {
                    SFError::ParsingError(
                        "action new life",
                        player_id.to_string(),
                    )
                })?,
            });
        }

        Ok(())
    }
}



#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fighter {
    
    pub typ: FighterTyp,
    
    
    pub id: i64,
    
    pub name: Option<String>,
    
    pub level: u32,
    
    pub life: u32,
    
    pub attributes: EnumMap<AttributeType, u32>,
    
    pub class: Class,
}

impl Fighter {
    
    pub(crate) fn parse(data: &[&str]) -> Option<Fighter> {
        let fighter_typ: i64 = data.cfsget(5, "fighter typ").ok()??;

        let mut fighter_type = match fighter_typ {
            -391 => FighterTyp::Companion(CompanionClass::Warrior),
            -392 => FighterTyp::Companion(CompanionClass::Mage),
            -393 => FighterTyp::Companion(CompanionClass::Scout),
            1.. => FighterTyp::Player,
            x => {
                let monster_id = soft_into(-x, "monster_id", 0);
                FighterTyp::Monster(monster_id)
            }
        };

        let mut attributes = EnumMap::default();
        let raw_atrs =
            parse_vec(data.get(10..15)?, "fighter attributes", |a| {
                a.parse().ok()
            })
            .ok()?;
        update_enum_map(&mut attributes, &raw_atrs);

        let class: i32 = data.cfsget(27, "fighter class").ok().flatten()?;
        let class: Class = FromPrimitive::from_i32(class - 1)?;

        let id = data.cfsget(5, "fighter id").ok()?.unwrap_or_default();

        let name = match data.cget(6, "fighter name").ok()?.parse::<i64>() {
            Ok(-770..=-740) => {
                
                fighter_type = FighterTyp::FortressWall;
                None
            }
            Ok(-712) => {
                fighter_type = FighterTyp::FortressPillager;
                None
            }
            Ok(..=-1) => None,
            Ok(0) => {
                let id = data.cget(15, "fighter uwm").ok()?;
                
                if ["-910", "-935", "-933", "-924"].contains(&id) {
                    fighter_type = FighterTyp::UnderworldMinion;
                }
                None
            }
            Ok(pid) if pid == id && fighter_type == FighterTyp::Player => {
                fighter_type = FighterTyp::Pet;
                None
            }
            _ => Some(data.cget(6, "fighter name").ok()?.to_string()),
        };

        Some(Fighter {
            typ: fighter_type,
            id,
            name,
            level: data.cfsget(7, "fighter lvl").ok()??,
            life: data.cfsget(8, "fighter life").ok()??,
            attributes,
            class,
        })
    }
}


#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FightAction {
    
    pub acting_id: i64,
    
    
    
    pub other_new_life: i64,
    
    pub action: FightActionType,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum FightActionType {
    
    Attack,
    
    MushroomCatapult,
    
    Blocked,
    
    Evaded,
    
    MinionAttack,
    
    MinionAttackBlocked,
    
    MinionAttackEvaded,
    
    MinionCrit,
    
    SummonSpecial,
    
    
    Unknown,
}

impl FightActionType {
    pub(crate) fn parse(val: u32) -> FightActionType {
        
        match val {
            0 | 1 => FightActionType::Attack,
            2 => FightActionType::MushroomCatapult,
            3 => FightActionType::Blocked,
            4 => FightActionType::Evaded,
            5 => FightActionType::MinionAttack,
            6 => FightActionType::MinionAttackBlocked,
            7 => FightActionType::MinionAttackEvaded,
            25 => FightActionType::MinionCrit,
            200..=250 => FightActionType::SummonSpecial,
            _ => FightActionType::Unknown,
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FighterTyp {
    
    #[default]
    Player,
    
    Monster(u16),
    
    Companion(CompanionClass),
    
    FortressPillager,
    
    FortressWall,
    
    UnderworldMinion,
    
    Pet,
}
