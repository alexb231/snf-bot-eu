use std::hash::Hash;

use enum_map::EnumMap;
use fastrand::Rng;

use crate::{
    command::AttributeType,
    gamestate::{character::Class, items::*},
    misc::EnumMapGet,
    simulate::{damage::*, upgradeable::UpgradeableFighter, *},
};








#[derive(Debug, Clone)]
pub struct Fighter {
    pub ident: FighterIdent,
    
    
    pub name: std::sync::Arc<str>,
    
    pub class: Class,
    
    pub level: u16,
    
    pub attributes: EnumMap<AttributeType, u32>,
    
    pub max_health: f64,
    
    pub armor: u32,
    
    pub first_weapon: Option<Weapon>,
    
    
    pub second_weapon: Option<Weapon>,
    
    pub has_reaction_enchant: bool,
    
    pub crit_dmg_multi: f64,
    
    pub resistances: EnumMap<Element, i32>,
    
    pub portal_dmg_bonus: f64,
    
    pub is_companion: bool,
    
    pub gladiator_lvl: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FighterIdent(u32);

impl FighterIdent {
    pub fn new() -> Self {
        FighterIdent(fastrand::u32(..))
    }
}

impl Default for FighterIdent {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&Monster> for Fighter {
    fn from(monster: &Monster) -> Fighter {
        let mut weapon = Weapon {
            rune_value: 0,
            rune_type: None,
            damage: DamageRange {
                min: f64::from(monster.min_dmg),
                max: f64::from(monster.max_dmg),
            },
        };
        let mut resistances = EnumMap::default();

        if let Some(runes) = &monster.runes {
            resistances = runes.resistances;
            weapon.rune_value = runes.damage;
            weapon.rune_type = Some(runes.damage_type);
        }

        
        let second_weapon =
            (monster.class == Class::Assassin).then(|| weapon.clone());

        Fighter {
            ident: FighterIdent::new(),
            name: std::sync::Arc::from(monster.name),
            class: monster.class,
            level: monster.level,
            attributes: monster.attributes,
            max_health: monster.hp as f64,
            armor: monster.armor,
            second_weapon,
            first_weapon: Some(weapon),
            has_reaction_enchant: false,
            crit_dmg_multi: 2.0,
            resistances,
            portal_dmg_bonus: 0.0,
            is_companion: false,
            gladiator_lvl: 0,
        }
    }
}

impl From<&UpgradeableFighter> for Fighter {
    fn from(char: &UpgradeableFighter) -> Self {
        use RuneType as RT;

        let attributes = char.attributes();
        let health = char.hit_points(&attributes) as f64;

        let mut resistances = EnumMap::default();
        let mut has_reaction = false;
        let mut extra_crit_dmg = 0.0;
        let mut armor = 0;
        let mut weapon = None;
        let mut offhand = None;

        for (slot, item) in &char.equipment.0 {
            let Some(item) = item else {
                continue;
            };
            armor += item.armor();
            match item.enchantment {
                Some(Enchantment::SwordOfVengeance) => {
                    extra_crit_dmg = 0.05;
                }
                Some(Enchantment::ShadowOfTheCowboy) => {
                    has_reaction = true;
                }
                _ => {}
            }

            if let Some(rune) = item.rune {
                let mut apply = |element| {
                    *resistances.get_mut(element) += i32::from(rune.value);
                };
                match rune.typ {
                    RT::FireResistance => apply(Element::Fire),
                    RT::ColdResistence => apply(Element::Cold),
                    RT::LightningResistance => apply(Element::Lightning),
                    RT::TotalResistence => {
                        for val in &mut resistances.values_mut() {
                            *val += i32::from(rune.value);
                        }
                    }
                    _ => {}
                }
            }

            match item.typ {
                ItemType::Weapon { min_dmg, max_dmg } => {
                    let mut res = Weapon {
                        rune_value: 0,
                        rune_type: None,
                        damage: DamageRange {
                            min: f64::from(min_dmg),
                            max: f64::from(max_dmg),
                        },
                    };
                    if let Some(rune) = item.rune {
                        res.rune_type = match rune.typ {
                            RT::FireDamage => Some(Element::Fire),
                            RT::ColdDamage => Some(Element::Cold),
                            RT::LightningDamage => Some(Element::Lightning),
                            _ => None,
                        };
                        res.rune_value = rune.value.into();
                    }
                    match slot {
                        EquipmentSlot::Weapon => weapon = Some(res),
                        EquipmentSlot::Shield => offhand = Some(res),
                        _ => {}
                    }
                }
                ItemType::Shield { block_chance: _ } => {
                    
                    
                }
                _ => (),
            }
        }

        let crit_multiplier =
            2.0 + extra_crit_dmg + f64::from(char.gladiator) * 0.11;

        Fighter {
            ident: FighterIdent::new(),
            name: char.name.clone(),
            class: char.class,
            level: char.level,
            attributes,
            max_health: health,
            armor,
            first_weapon: weapon,
            second_weapon: offhand,
            has_reaction_enchant: has_reaction,
            crit_dmg_multi: crit_multiplier,
            resistances,
            portal_dmg_bonus: f64::from(char.portal_dmg_bonus),
            is_companion: char.is_companion,
            gladiator_lvl: char.gladiator,
        }
    }
}







#[derive(Debug, Clone)]
pub(crate) struct InBattleFighter {
    
    
    #[allow(unused)]
    pub name: Arc<str>,
    
    pub class: Class,
    
    pub max_health: f64,
    
    
    pub health: f64,
    
    
    pub damage: DamageRange,
    
    
    pub reaction: u8,
    
    pub crit_chance: f64,
    
    pub crit_dmg_multi: f64,
    
    
    
    pub opponent_is_mage: bool,

    
    
    pub class_data: ClassData,
}


#[derive(Debug, Clone)]
pub(crate) enum ClassData {
    Warrior {
        
        block_chance: i32,
    },
    Mage,
    Scout,
    Assassin {
        
        secondary_damage: DamageRange,
    },
    BattleMage {
        
        fireball_dmg: f64,
    },
    Berserker {
        
        
        frenzy_attacks: u32,
    },
    DemonHunter {
        
        revive_count: u32,
    },
    Druid {
        
        is_in_bear_form: bool,
        
        rage_crit_chance: f64,
        
        
        has_just_dodged: bool,
        
        swoop_chance: f64,
        
        
        swoop_dmg_multi: f64,
    },
    Bard {
        
        melody_remaining_rounds: i32,
        
        melody_cooldown_rounds: i32,
        
        
        melody_dmg_multi: f64,
    },
    Necromancer {
        
        damage_multi: f64,
        
        minion: Option<Minion>,
        
        minion_remaining_rounds: i32,
        
        skeleton_revived: i32,
    },
    Paladin {
        
        
        initial_armor_reduction: f64,
        
        stance: Stance,
    },
    PlagueDoctor {
        
        poison_remaining_round: usize,
        
        
        poison_dmg_multis: [f64; 3],
    },
}


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Minion {
    Skeleton,
    Hound,
    Golem,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stance {
    Regular,
    Defensive,
    Offensive,
}

impl Stance {
    pub(crate) fn damage_multiplier(self) -> f64 {
        match self {
            Stance::Regular => 1.0,
            Stance::Defensive => 1.0 / 0.833 * 0.568,
            Stance::Offensive => 1.0 / 0.833 * 1.253,
        }
    }

    pub(crate) fn block_chance(self) -> u8 {
        match self {
            Stance::Regular => 30,
            Stance::Defensive => 50,
            Stance::Offensive => 25,
        }
    }
}

pub(crate) fn calculate_crit_chance(
    main: &Fighter,
    opponent: &Fighter,
    cap: f64,
    crit_bonus: f64,
) -> f64 {
    let luck_factor = f64::from(main.attributes[AttributeType::Luck]) * 5.0;
    let opponent_level_factor = f64::from(opponent.level) * 2.0;
    let crit_chance = luck_factor / opponent_level_factor / 100.0 + crit_bonus;
    crit_chance.min(cap)
}

impl InBattleFighter {
    
    pub fn is_mage(&self) -> bool {
        self.class == Class::Mage
    }

    
    
    pub fn update_opponent(
        &mut self,
        main: &Fighter,
        opponent: &Fighter,
        reduce_gladiator: bool,
    ) {
        self.damage = calculate_damage(main, opponent, false);

        let mut crit_dmg_multi = main.crit_dmg_multi;
        if reduce_gladiator {
            let glad_lvl = main.gladiator_lvl.min(opponent.gladiator_lvl);
            crit_dmg_multi -= f64::from(glad_lvl) * 0.11;
        }
        self.crit_dmg_multi = crit_dmg_multi;
        self.crit_chance = calculate_crit_chance(main, opponent, 0.5, 0.0);

        self.class_data.update_opponent(main, opponent);
        self.opponent_is_mage = opponent.class == Class::Mage;
    }

    
    
    pub fn attack(
        &mut self,
        target: &mut InBattleFighter,
        round: &mut u32,
        rng: &mut Rng,
    ) -> bool {
        match &mut self.class_data {
            ClassData::Assassin { secondary_damage } => {
                let secondary_damage = *secondary_damage;

                
                *round += 1;
                if target.will_take_attack(rng) {
                    let first_weapon_damage =
                        self.calc_basic_hit_damage(*round, rng);
                    if target.take_attack_dmg(first_weapon_damage, round, rng) {
                        return true;
                    }
                }

                
                *round += 1;
                if !target.will_take_attack(rng) {
                    return false;
                }

                let second_weapon_damage = calculate_hit_damage(
                    &secondary_damage,
                    *round,
                    self.crit_chance,
                    self.crit_dmg_multi,
                    rng,
                );

                target.take_attack_dmg(second_weapon_damage, round, rng)
            }
            ClassData::Druid {
                has_just_dodged,
                rage_crit_chance,
                is_in_bear_form,
                swoop_chance,
                swoop_dmg_multi,
            } => {
                if target.is_mage() {
                    return self.attack_generic(target, round, rng);
                }

                if *has_just_dodged {
                    
                    *is_in_bear_form = true;
                    *has_just_dodged = false;

                    *round += 1;

                    if !target.will_take_attack(rng) {
                        return false;
                    }

                    let rage_crit_multi = 6.0 * self.crit_dmg_multi / 2.0;
                    let dmg = calculate_hit_damage(
                        &self.damage,
                        *round,
                        *rage_crit_chance,
                        rage_crit_multi,
                        rng,
                    );
                    return target.take_attack_dmg(dmg, round, rng);
                }

                *is_in_bear_form = false;

                

                let do_swoop_attack = rng.f64() < *swoop_chance;
                if do_swoop_attack {
                    *round += 1;
                    *swoop_chance = (*swoop_chance + 0.05).min(0.5);

                    if target.will_take_attack(rng) {
                        let swoop_dmg_multi = *swoop_dmg_multi;
                        let swoop_dmg = self.calc_basic_hit_damage(*round, rng)
                            * swoop_dmg_multi;

                        if target.take_attack_dmg(swoop_dmg, round, rng) {
                            return true;
                        }
                    }
                }

                self.attack_generic(target, round, rng)
            }
            ClassData::Bard {
                melody_remaining_rounds,
                melody_cooldown_rounds,
                melody_dmg_multi,
            } => {
                if target.is_mage() {
                    return self.attack_generic(target, round, rng);
                }

                if *melody_remaining_rounds <= 0 && *melody_cooldown_rounds <= 0
                {
                    
                    let (length, multi) = match rng.u32(0..4) {
                        0 | 1 => (3, 1.4),
                        2 => (3, 1.2),
                        _ => (4, 1.6),
                    };
                    *melody_remaining_rounds = length;
                    *melody_dmg_multi = multi;
                    *melody_cooldown_rounds = 4;
                } else if *melody_remaining_rounds == 0 {
                    
                    *melody_dmg_multi = 1.0;
                }

                *melody_remaining_rounds -= 1;
                *melody_cooldown_rounds -= 1;

                if !target.will_take_attack(rng) {
                    return false;
                }

                let dmg_multi = *melody_dmg_multi;
                let dmg = self.calc_basic_hit_damage(*round, rng) * dmg_multi;
                target.take_attack_dmg(dmg, round, rng)
            }
            ClassData::Necromancer {
                minion,
                minion_remaining_rounds: minion_rounds,
                ..
            } => {
                if target.is_mage() {
                    return self.attack_generic(target, round, rng);
                }
                *round += 1;

                if minion.is_none() && rng.bool() {
                    
                    let (new_type, new_rounds) = match rng.u8(0..3) {
                        0 => (Minion::Skeleton, 3),
                        1 => (Minion::Hound, 2),
                        _ => (Minion::Golem, 4),
                    };

                    *minion = Some(new_type);
                    *minion_rounds = new_rounds;
                    return self.attack_with_minion(target, round, rng);
                }

                if target.will_take_attack(rng) {
                    
                    let dmg = self.calc_basic_hit_damage(*round, rng);
                    if target.take_attack_dmg(dmg, round, rng) {
                        return true;
                    }
                }

                self.attack_with_minion(target, round, rng)
            }
            ClassData::Paladin { stance, .. } => {
                if target.is_mage() {
                    return self.attack_generic(target, round, rng);
                }

                *round += 1;
                if rng.bool() {
                    
                    *stance = match stance {
                        Stance::Regular => Stance::Defensive,
                        Stance::Defensive => Stance::Offensive,
                        Stance::Offensive => Stance::Regular,
                    };
                }

                if !target.will_take_attack(rng) {
                    return false;
                }

                let dmg_multi = stance.damage_multiplier();
                let dmg = self.calc_basic_hit_damage(*round, rng) * dmg_multi;
                target.take_attack_dmg(dmg, round, rng)
            }
            ClassData::PlagueDoctor {
                poison_remaining_round,
                poison_dmg_multis,
            } => {
                if target.is_mage() {
                    return self.attack_generic(target, round, rng);
                }

                if *poison_remaining_round == 0 && rng.bool() {
                    
                    *round += 1;
                    if !target.will_take_attack(rng) {
                        return false;
                    }

                    *poison_remaining_round = 3;

                    let dmg_multi = poison_dmg_multis[2];
                    let dmg =
                        self.calc_basic_hit_damage(*round, rng) * dmg_multi;
                    return target.take_attack_dmg(dmg, round, rng);
                }

                if *poison_remaining_round > 0 {
                    
                    
                    *round += 1;
                    *poison_remaining_round -= 1;

                    #[allow(clippy::indexing_slicing)]
                    let dmg_multi = poison_dmg_multis[*poison_remaining_round];
                    let dmg =
                        self.calc_basic_hit_damage(*round, rng) * dmg_multi;

                    if target.class == Class::Paladin {
                        
                        target.health -= dmg;
                        if target.health <= 0.0 {
                            return true;
                        }
                    } else if target.take_attack_dmg(dmg, round, rng) {
                        return true;
                    }
                }
                self.attack_generic(target, round, rng)
            }
            ClassData::Mage => {
                
                let dmg = self.calc_basic_hit_damage(*round, rng);
                target.take_attack_dmg(dmg, round, rng)
            }
            _ => self.attack_generic(target, round, rng),
        }
    }

    
    
    fn attack_generic(
        &mut self,
        target: &mut InBattleFighter,
        round: &mut u32,
        rng: &mut Rng,
    ) -> bool {
        *round += 1;

        if !target.will_take_attack(rng) {
            return false;
        }

        let dmg = self.calc_basic_hit_damage(*round, rng);
        target.take_attack_dmg(dmg, round, rng)
    }

    
    pub fn attack_before_fight(
        &mut self,
        target: &mut InBattleFighter,
        round: &mut u32,
        rng: &mut Rng,
    ) -> bool {
        match &mut self.class_data {
            ClassData::BattleMage { fireball_dmg } => {
                *round += 1;
                target.take_attack_dmg(*fireball_dmg, round, rng)
            }
            _ => false,
        }
    }

    
    pub fn will_skips_opponent_round(
        &mut self,
        target: &mut InBattleFighter,
        _round: &mut u32,
        rng: &mut Rng,
    ) -> bool {
        match &mut self.class_data {
            ClassData::Berserker { frenzy_attacks } => {
                if target.class == Class::Mage {
                    return false;
                }

                if *frenzy_attacks < 14 && rng.bool() {
                    *frenzy_attacks += 1;
                    return true;
                }

                *frenzy_attacks = 0;
                false
            }
            _ => false,
        }
    }

    
    
    
    pub fn take_attack_dmg(
        &mut self,
        damage: f64,
        round: &mut u32,
        rng: &mut Rng,
    ) -> bool {
        match &mut self.class_data {
            ClassData::DemonHunter { revive_count } => {
                let health = &mut self.health;
                *health -= damage;
                if *health > 0.0 {
                    return false;
                }
                if self.opponent_is_mage {
                    return true;
                }

                
                let revive_chance = 0.44 - (f64::from(*revive_count) * 0.11);
                if revive_chance <= 0.0 || rng.f64() >= revive_chance {
                    return true;
                }

                *round += 1;
                *revive_count += 1;

                true
            }
            ClassData::Paladin {
                stance,
                initial_armor_reduction,
            } => {
                let current_armor_reduction = match stance {
                    Stance::Regular | Stance::Defensive => 1.0,
                    Stance::Offensive => {
                        1.0 / (1.0 - *initial_armor_reduction)
                            * (1.0 - initial_armor_reduction.min(0.20))
                    }
                };
                let actual_damage = damage * current_armor_reduction;
                let health = &mut self.health;

                if self.opponent_is_mage {
                    *health -= actual_damage;
                    return *health <= 0.0;
                }

                if *stance == Stance::Defensive
                    && rng.u8(1..=100) <= stance.block_chance()
                {
                    let heal_cap = actual_damage * 0.3;
                    *health += (self.max_health - *health).clamp(0.0, heal_cap);
                    return false;
                }

                *health -= actual_damage;
                *health <= 0.0
            }
            _ => {
                let health = &mut self.health;
                *health -= damage;
                *health <= 0.0
            }
        }
    }

    
    pub fn will_take_attack(&mut self, rng: &mut Rng) -> bool {
        match &mut self.class_data {
            ClassData::Warrior { block_chance } => {
                rng.i32(1..=100) > *block_chance
            }
            ClassData::Assassin { .. } | ClassData::Scout => rng.bool(),
            ClassData::Druid {
                is_in_bear_form,
                has_just_dodged,
                ..
            } => {
                if !*is_in_bear_form && rng.u8(1..=100) <= 35 {
                    
                    *has_just_dodged = true;
                    return false;
                }
                true
            }
            ClassData::Necromancer { minion, .. } => {
                if self.opponent_is_mage {
                    return true;
                }
                if *minion != Some(Minion::Golem) {
                    return true;
                }
                rng.u8(1..=100) > 25
            }
            ClassData::Paladin { stance, .. } => {
                *stance == Stance::Defensive
                    || rng.u8(1..=100) > stance.block_chance()
            }
            ClassData::PlagueDoctor {
                poison_remaining_round,
                ..
            } => {
                let chance = match poison_remaining_round {
                    3 => 65,
                    2 => 50,
                    1 => 35,
                    _ => 20,
                };
                rng.u8(1..=100) > chance
            }
            _ => true,
        }
    }

    fn calc_basic_hit_damage(&self, round: u32, rng: &mut Rng) -> f64 {
        calculate_hit_damage(
            &self.damage,
            round,
            self.crit_chance,
            self.crit_dmg_multi,
            rng,
        )
    }

    fn attack_with_minion(
        &mut self,
        target: &mut InBattleFighter,
        round: &mut u32,
        rng: &mut Rng,
    ) -> bool {
        let ClassData::Necromancer {
            minion,
            minion_remaining_rounds,
            skeleton_revived,
            damage_multi,
        } = &mut self.class_data
        else {
            
            return false;
        };

        if minion.is_none() {
            return false;
        }

        *round += 1;

        *minion_remaining_rounds -= 1;

        
        
        if *minion_remaining_rounds == 0
            && *minion == Some(Minion::Skeleton)
            && *skeleton_revived < 1
            && rng.bool()
        {
            *minion_remaining_rounds = 1;
            *skeleton_revived += 1;
        } else if *minion_remaining_rounds == 0 {
            *minion = None;
            *skeleton_revived = 0;
        }

        if !target.will_take_attack(rng) {
            return false;
        }

        let mut crit_chance = self.crit_chance;
        let mut crit_multi = self.crit_dmg_multi;
        if *minion == Some(Minion::Hound) {
            crit_chance = (crit_chance + 0.1).min(0.6);
            crit_multi = 2.5 * (crit_multi / 2.0);
        }

        let mut dmg = calculate_hit_damage(
            &self.damage,
            *round,
            crit_chance,
            crit_multi,
            rng,
        );

        let base_multi = *damage_multi;
        let minion_dmg_multiplier = match minion {
            Some(Minion::Skeleton) => (base_multi + 0.25) / base_multi,
            Some(Minion::Hound) => (base_multi + 1.0) / base_multi,
            Some(Minion::Golem) => 1.0,
            None => 0.0,
        };
        dmg *= minion_dmg_multiplier;

        target.take_attack_dmg(dmg, round, rng)
    }
}

impl InBattleFighter {
    pub(crate) fn new(
        main: &Fighter,
        opponent: &Fighter,
        reduce_gladiator: bool,
    ) -> InBattleFighter {
        let class_data = ClassData::new(main, opponent);

        let mut res = InBattleFighter {
            name: main.name.clone(),
            class: main.class,
            health: main.max_health,
            max_health: main.max_health,
            reaction: u8::from(main.has_reaction_enchant),
            damage: DamageRange::default(),
            crit_chance: 0.0,
            crit_dmg_multi: 0.0,
            opponent_is_mage: false,
            class_data,
        };
        res.update_opponent(main, opponent, reduce_gladiator);
        res
    }
}

impl ClassData {
    pub(crate) fn update_opponent(
        &mut self,
        main: &Fighter,
        opponent: &Fighter,
    ) {
        
        
        match self {
            ClassData::Bard { .. }
            | ClassData::DemonHunter { .. }
            | ClassData::Mage
            | ClassData::Scout
            | ClassData::Warrior { .. } => {}
            ClassData::Assassin { secondary_damage } => {
                let range = calculate_damage(main, opponent, true);
                *secondary_damage = range;
            }
            ClassData::BattleMage { fireball_dmg, .. } => {
                *fireball_dmg = calculate_fire_ball_damage(main, opponent);
            }
            ClassData::Berserker {
                frenzy_attacks: chain_attack_counter,
            } => *chain_attack_counter = 0,
            ClassData::Druid {
                rage_crit_chance,
                swoop_dmg_multi,
                ..
            } => {
                *rage_crit_chance =
                    calculate_crit_chance(main, opponent, 0.75, 0.1);
                *swoop_dmg_multi = calculate_swoop_damage(main, opponent);
            }

            ClassData::Necromancer { damage_multi, .. } => {
                *damage_multi = calculate_damage_multiplier(main, opponent);
            }
            ClassData::Paladin {
                initial_armor_reduction,
                ..
            } => {
                *initial_armor_reduction =
                    calculate_damage_reduction(opponent, main);
            }
            ClassData::PlagueDoctor {
                poison_dmg_multis, ..
            } => {
                let base_dmg_multi =
                    calculate_damage_multiplier(main, opponent);

                let dmg_multiplier = Class::PlagueDoctor.damage_multiplier();
                let class_dmg_multi = base_dmg_multi / dmg_multiplier;

                *poison_dmg_multis = [
                    (base_dmg_multi - 0.9 * class_dmg_multi) / base_dmg_multi,
                    (base_dmg_multi - 0.55 * class_dmg_multi) / base_dmg_multi,
                    (base_dmg_multi - 0.2 * class_dmg_multi) / base_dmg_multi,
                ];
                
            }
        }
    }

    pub(crate) fn new(main: &Fighter, opponent: &Fighter) -> ClassData {
        let mut res = match main.class {
            Class::Warrior if main.is_companion => {
                ClassData::Warrior { block_chance: 0 }
            }
            Class::Warrior => ClassData::Warrior { block_chance: 25 },
            Class::Mage => ClassData::Mage,
            Class::Scout => ClassData::Scout,
            Class::Assassin => ClassData::Assassin {
                secondary_damage: DamageRange::default(),
            },
            Class::BattleMage => ClassData::BattleMage { fireball_dmg: 0.0 },
            Class::Berserker => ClassData::Berserker { frenzy_attacks: 0 },
            Class::DemonHunter => ClassData::DemonHunter { revive_count: 0 },
            Class::Druid => ClassData::Druid {
                rage_crit_chance: 0.0,
                is_in_bear_form: false,
                has_just_dodged: false,
                swoop_chance: 0.15,
                swoop_dmg_multi: 0.0,
            },
            Class::Bard => ClassData::Bard {
                melody_remaining_rounds: -1,
                melody_cooldown_rounds: 0,
                melody_dmg_multi: 1.0,
            },
            Class::Necromancer => ClassData::Necromancer {
                damage_multi: 0.0,
                minion: None,
                minion_remaining_rounds: 0,
                skeleton_revived: 0,
            },
            Class::Paladin => ClassData::Paladin {
                initial_armor_reduction: 0.0,
                stance: Stance::Regular,
            },
            Class::PlagueDoctor => ClassData::PlagueDoctor {
                poison_remaining_round: 0,
                poison_dmg_multis: [0.0, 0.0, 0.0],
            },
        };
        res.update_opponent(main, opponent);
        res
    }
}
