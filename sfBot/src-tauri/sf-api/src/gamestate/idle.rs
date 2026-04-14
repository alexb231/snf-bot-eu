#![allow(clippy::module_name_repetitions)]
use chrono::{DateTime, Local};
use enum_map::{Enum, EnumMap};
use num_bigint::BigInt;
use strum::EnumIter;

use super::ServerTime;


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdleGame {
    
    pub current_money: BigInt,
    
    pub current_runes: BigInt,
    
    pub resets: u32,
    
    pub sacrifice_runes: BigInt,
    
    pub merchant_new_goods: DateTime<Local>,
    
    
    
    pub total_sacrificed: BigInt,
    
    _current_money_2: BigInt,
    
    pub buildings: EnumMap<IdleBuildingType, IdleBuilding>,
}


#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdleBuilding {
    
    pub level: u32,
    
    pub earning: BigInt,
    
    
    
    pub cycle_start: Option<DateTime<Local>>,
    
    
    
    pub cycle_end: Option<DateTime<Local>>,
    
    pub golden: bool,
    
    pub upgrade_cost: BigInt,
    
    pub upgrade_cost_10x: BigInt,
    
    pub upgrade_cost_25x: BigInt,
    
    pub upgrade_cost_100x: BigInt,
}

impl IdleGame {
    pub(crate) fn parse_idle_game(
        data: &[BigInt],
        server_time: ServerTime,
    ) -> Option<IdleGame> {
        if data.len() < 118 {
            return None;
        }

        let mut res = IdleGame {
            resets: data.get(2)?.try_into().ok()?,
            merchant_new_goods: server_time.convert_to_local(
                data.get(63)?.try_into().ok()?,
                "trader time",
            )?,
            current_money: data.get(72)?.clone(),
            total_sacrificed: data.get(73)?.clone(),
            _current_money_2: data.get(74)?.clone(),
            sacrifice_runes: data.get(75)?.clone(),
            current_runes: data.get(76)?.clone(),
            buildings: EnumMap::default(),
        };

        
        for (pos, building) in
            res.buildings.as_mut_array().iter_mut().enumerate()
        {
            building.level = data.get(pos + 3)?.try_into().ok()?;
            building.earning.clone_from(data.get(pos + 13)?);
            building.cycle_start = server_time.convert_to_local(
                data.get(pos + 23)?.try_into().ok()?,
                "idle cycle start time",
            );
            building.cycle_end = server_time.convert_to_local(
                data.get(pos + 33)?.try_into().ok()?,
                "idle cycle end time",
            );
            building.golden = data.get(pos + 53)? == &1.into();
            building.upgrade_cost.clone_from(data.get(pos + 78)?);
            building.upgrade_cost_10x.clone_from(data.get(pos + 88)?);
            building.upgrade_cost_25x.clone_from(data.get(pos + 98)?);
            building.upgrade_cost_100x.clone_from(data.get(pos + 108)?);
        }
        Some(res)
    }
}


#[derive(Debug, Clone, Copy, Enum, EnumIter, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum IdleBuildingType {
    Seat = 1,
    PopcornStand,
    ParkingLot,
    Trap,
    Drinks,
    DeadlyTrap,
    VIPSeat,
    Snacks,
    StrayingMonsters,
    Toilet,
}
