#![allow(clippy::module_name_repetitions)]
use std::time::Duration;

use chrono::{DateTime, Local};
use enum_map::{Enum, EnumMap};
use num_derive::FromPrimitive;
use strum::{EnumCount, EnumIter, IntoEnumIterator};

use super::{
    ArrSkip, CCGet, CFPGet, CSTGet, SFError, ServerTime, items::GemType,
};
use crate::{
    PlayerId,
    gamestate::{CGet, EnumMapGet},
};


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fortress {
    
    
    pub buildings: EnumMap<FortressBuildingType, FortressBuilding>,
    
    pub units: EnumMap<FortressUnitType, FortressUnit>,
    
    pub resources: EnumMap<FortressResourceType, FortressResource>,
    
    
    
    
    
    
    
    
    
    pub last_collectable_updated: Option<DateTime<Local>>,

    
    pub building_max_lvl: u8,
    
    
    pub wall_combat_lvl: u16,

    
    pub building_upgrade: FortressAction<FortressBuildingType>,

    
    
    pub upgrades: u16,
    
    pub honor: u32,
    
    pub rank: Option<u32>,

    
    pub gem_search: FortressAction<GemType>,

    
    pub hall_of_knights_level: u16,
    
    
    pub hall_of_knights_upgrade_price: FortressCost,

    
    
    
    
    pub attack_target: Option<PlayerId>,
    
    pub attack_free_reroll: Option<DateTime<Local>>,
    
    pub opponent_reroll_price: u64,
}



#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FortressCost {
    
    pub time: Duration,
    
    pub wood: u64,
    
    pub stone: u64,
    
    pub silver: u64,
}

impl FortressCost {
    pub(crate) fn parse(data: &[i64]) -> Result<FortressCost, SFError> {
        Ok(FortressCost {
            time: Duration::from_secs(data.csiget(0, "fortress time", 0)?),
            
            silver: data.csiget(1, "silver cost", u64::MAX)?,
            wood: data.csiget(2, "wood cost", u64::MAX)?,
            stone: data.csiget(3, "stone cost", u64::MAX)?,
        })
    }
}


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FortressResource {
    
    
    pub current: u64,
    
    
    pub limit: u64,
    
    
    pub limit_next_level: u64,
    
    pub production: FortressProduction,
    
    pub secret_storage: FortressSecretStorage,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FortressSecretStorage {
    pub amount: u64,
    pub limit: u64,
}



#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FortressProduction {
    
    
    
    
    pub last_collectable: u64,
    
    
    pub limit: u64,
    
    
    pub per_hour: u64,
    
    
    pub per_hour_next_lvl: u64,
}


#[derive(Debug, Clone, Copy, EnumCount, EnumIter, PartialEq, Eq, Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum FortressResourceType {
    Wood = 0,
    Stone = 1,
    Experience = 2,
}


#[derive(
    Debug, Clone, Copy, EnumCount, FromPrimitive, PartialEq, Eq, Enum, EnumIter,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum FortressBuildingType {
    Fortress = 0,
    LaborersQuarters = 1,
    WoodcuttersHut = 2,
    Quarry = 3,
    GemMine = 4,
    Academy = 5,
    ArcheryGuild = 6,
    Barracks = 7,
    MagesTower = 8,
    Treasury = 9,
    Smithy = 10,
    Wall = 11,
    FortressGroupBonusUpgrade = 12,
}

impl FortressBuildingType {
    
    
    #[must_use]
    pub fn required_min_fortress_level(&self) -> u16 {
        match self {
            FortressBuildingType::Fortress => 0,
            FortressBuildingType::LaborersQuarters
            | FortressBuildingType::Quarry
            | FortressBuildingType::Smithy
            | FortressBuildingType::WoodcuttersHut => 1,
            FortressBuildingType::Treasury => 2,
            FortressBuildingType::GemMine => 3,
            FortressBuildingType::Barracks | FortressBuildingType::Wall => 4,
            FortressBuildingType::ArcheryGuild => 5,
            FortressBuildingType::Academy => 6,
            FortressBuildingType::MagesTower => 7,
            FortressBuildingType::FortressGroupBonusUpgrade => 0,
        }
    }

    
    #[must_use]
    pub fn unit_produced(self) -> Option<FortressUnitType> {
        match self {
            FortressBuildingType::Barracks => Some(FortressUnitType::Soldier),
            FortressBuildingType::MagesTower => {
                Some(FortressUnitType::Magician)
            }
            FortressBuildingType::ArcheryGuild => {
                Some(FortressUnitType::Archer)
            }
            _ => None,
        }
    }
}


#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FortressUnit {
    
    pub level: u16,

    
    pub count: u16,
    
    pub in_training: u16,
    
    pub limit: u16,
    
    pub training: FortressAction<()>,

    
    pub upgrade_cost: FortressCost,
    
    pub upgrade_next_lvl: u64,
}



#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FortressAction<T> {
    
    
    pub start: Option<DateTime<Local>>,
    
    pub finish: Option<DateTime<Local>>,
    
    pub cost: FortressCost,
    
    
    pub target: Option<T>,
}

impl<T> Default for FortressAction<T> {
    fn default() -> Self {
        Self {
            start: None,
            finish: None,
            cost: FortressCost::default(),
            target: None,
        }
    }
}


#[derive(Debug, Clone, Copy, EnumCount, PartialEq, Eq, Enum, EnumIter)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum FortressUnitType {
    Soldier = 0,
    Magician = 1,
    Archer = 2,
}

impl FortressUnitType {
    
    #[must_use]
    pub fn training_building(&self) -> FortressBuildingType {
        match self {
            FortressUnitType::Archer => FortressBuildingType::ArcheryGuild,
            FortressUnitType::Magician => FortressBuildingType::MagesTower,
            FortressUnitType::Soldier => FortressBuildingType::Barracks,
        }
    }
}



#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FortressBuilding {
    
    
    pub level: u16,
    
    pub upgrade_cost: FortressCost,
}

impl Fortress {
    
    
    
    #[must_use]
    pub fn in_use(&self, building_type: FortressBuildingType) -> bool {
        
        if let Some(unit_type) = building_type.unit_produced()
            && let Some(finish) = self.units.get(unit_type).training.finish
            && finish > Local::now()
        {
            return true;
        }

        
        if building_type == FortressBuildingType::GemMine
            && self.gem_search.finish.is_some()
        {
            return true;
        }
        false
    }

    
    #[must_use]
    pub fn can_build(
        &self,
        building_type: FortressBuildingType,
        available_silver: u64,
    ) -> bool {
        let building_info = self.buildings.get(building_type);
        let fortress_level =
            self.buildings.get(FortressBuildingType::Fortress).level;
        let smithy_required_buildings = [
            FortressBuildingType::ArcheryGuild,
            FortressBuildingType::Barracks,
            FortressBuildingType::MagesTower,
            FortressBuildingType::Wall,
        ];

        
        
        
        if self.in_use(building_type) {
            return false;
        }

        
        let can_smithy_be_built = smithy_required_buildings
            .map(|required_building| {
                self.buildings.get(required_building).level
            })
            .iter()
            .all(|level| *level > 0);

        if matches!(building_type, FortressBuildingType::Smithy)
            && !can_smithy_be_built
        {
            
            false
        } else if !matches!(building_type, FortressBuildingType::Fortress)
            && building_info.level == fortress_level
        {
            
            
            false
        } else {
            let upgrade_cost = building_info.upgrade_cost;

            
            building_type.required_min_fortress_level() <= fortress_level
            
            && self.building_upgrade.target.is_none()
            
            && upgrade_cost.stone <= self.resources.get(FortressResourceType::Stone).current
            && upgrade_cost.wood <= self.resources.get(FortressResourceType::Wood).current
            && upgrade_cost.silver <= available_silver
        }
    }

    pub(crate) fn update_resources(
        &mut self,
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        for (idx, (typ, resource)) in self.resources.iter_mut().enumerate() {
            resource.production.last_collectable =
                data.csiget(idx, "ft resource last collectable", 0)?;
            resource.production.limit =
                data.csiget(3 + idx, "ft resource production limit", 0)?;
            resource.production.per_hour =
                data.csiget(8 + idx, "ft resource per hour", 0)?;

            if typ != FortressResourceType::Experience {
                resource.limit =
                    data.csiget(6 + idx, "ft resource limit", 0)?;
                resource.limit_next_level =
                    data.csiget(12 + idx, "ft resource per hour", 0)?;
                resource.secret_storage.limit =
                    data.csiget(14 + idx, "ft secret storage limit", 0)?;
            }
        }
        self.last_collectable_updated =
            data.cstget(11, "ft resource update", server_time)?;
        Ok(())
    }

    pub(crate) fn update_unit_prices(
        &mut self,
        data: &[i64],
    ) -> Result<(), SFError> {
        for (i, typ) in FortressUnitType::iter().enumerate() {
            self.units.get_mut(typ).training.cost =
                FortressCost::parse(data.skip(i * 4, "unit prices")?)?;
        }
        Ok(())
    }

    pub(crate) fn update_unit_upgrade_info(
        &mut self,
        data: &[i64],
    ) -> Result<(), SFError> {
        for (i, typ) in FortressUnitType::iter().enumerate() {
            self.units.get_mut(typ).upgrade_next_lvl =
                data.csiget(i * 3, "unit next lvl", 0)?;
            self.units.get_mut(typ).upgrade_cost.wood =
                data.csiget(1 + i * 3, "wood price next unit lvl", 0)?;
            self.units.get_mut(typ).upgrade_cost.stone =
                data.csiget(2 + i * 3, "stone price next unit lvl", 0)?;
        }
        Ok(())
    }

    pub(crate) fn update_levels(
        &mut self,
        data: &[i64],
    ) -> Result<(), SFError> {
        self.units.get_mut(FortressUnitType::Soldier).level =
            data.csiget(1, "soldier level", 0)?;
        self.units.get_mut(FortressUnitType::Magician).level =
            data.csiget(2, "magician level", 0)?;
        self.units.get_mut(FortressUnitType::Archer).level =
            data.csiget(3, "archer level", 0)?;
        Ok(())
    }

    pub(crate) fn update_prices(
        &mut self,
        data: &[i64],
    ) -> Result<(), SFError> {
        for (i, typ) in FortressBuildingType::iter().enumerate() {
            self.buildings.get_mut(typ).upgrade_cost =
                FortressCost::parse(data.skip(i * 4, "fortress unit prices")?)?;
        }
        self.gem_search.cost =
            FortressCost::parse(data.skip(48, "gem_search_cost")?)?;
        Ok(())
    }

    pub(crate) fn update_units(
        &mut self,
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        for (idx, unit) in self.units.values_mut().enumerate() {
            unit.count = data.csiget(idx, "ft unit count", 0)?;
            unit.in_training = data.csiget(3 + idx, "ft unit in que", 0)?;
            unit.training.start =
                data.cstget(6 + idx, "ft training start", server_time)?;
            unit.training.finish =
                data.cstget(9 + idx, "ft training end", server_time)?;
            
            
            
        }
        Ok(())
    }

    pub(crate) fn update(
        &mut self,
        data: &[i64],
        server_time: ServerTime,
    ) -> Result<(), SFError> {
        
        for (idx, (_, building)) in self.buildings.iter_mut().enumerate() {
            building.level = data.csiget(idx, "building lvl", 0)?;
        }
        let upgrade = &mut self.building_upgrade;
        upgrade.target =
            data.cfpget(12, "fortress building upgrade", |x| x - 1)?;
        upgrade.finish =
            data.cstget(13, "fortress upgrade end", server_time)?;
        upgrade.start =
            data.cstget(14, "fortress upgrade begin", server_time)?;

        self.upgrades = data.csiget(15, "fortress lvl", 0)?;
        self.honor = data.csiget(16, "fortress honor", 0)?;
        let fortress_rank: i64 = data.csiget(17, "fortress rank", 0)?;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        if fortress_rank > 0 {
            self.rank = Some(fortress_rank as u32);
        } else {
            self.rank = None;
        }
        self.attack_free_reroll =
            data.cstget(18, "fortress attack reroll", server_time)?;
        self.attack_target = data.cwiget(19, "fortress enemy")?;

        
        

        self.gem_search.target =
            GemType::parse(data.cget(22, "gem target")?, 0);
        self.gem_search.finish =
            data.cstget(23, "gem search end", server_time)?;
        self.gem_search.start =
            data.cstget(24, "gem search start", server_time)?;
        self.hall_of_knights_level =
            data.csiget(25, "hall of knights level", 0)?;

        

        if data.len() > 27 {
            log::warn!("fortress update has new values: {data:?}");
        }

        Ok(())
    }
}
