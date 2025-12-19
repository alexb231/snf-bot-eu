use chrono::{Duration, Local, NaiveTime};
use num_bigint::BigInt;
use sf_api::{
    command::{Command, FortunePayment},
    gamestate::{
        dungeons::{DungeonProgress, LightDungeon},
        fortress::{FortressBuildingType, FortressUnitType},
        guild::{BattlesJoined, GuildRank},
        items::{Item, ItemType},
        rewards::Event,
        social::{ClaimableMailType, ClaimableStatus},
        tavern::CurrentAction,
        underworld::{UnderworldBuildingType, UnderworldUnitType},
        unlockables::HellevatorStatus,
        GameState,
    },
};

use crate::{
    bot_runner::write_character_log,
    city_guard::sleep_between_commands,
    dungeon_management::fight_dungeon_with_highest_win_rate,
    fetch_character_setting,
    guild::check_participation,
    pet_management::get_pets_left_for_pet_arena,
    quarter::get_resource_from_setting,
    utils::{check_time_in_range, pretty_print, shitty_print},
    witch_enchantment::should_enchant,
};

/// Helper macro to log skip reason and return true
macro_rules! skip_with_reason {
    ($gs:expr, $func:expr, $reason:expr) => {{
        write_character_log(
            &$gs.character.name,
            $gs.character.player_id,
            &format!("SKIP_REASON: {} - {}", $func, $reason),
        );
        return true;
    }};
}

pub fn skipFunction(gs: &GameState, funcToExecute: &str) -> bool
{
    let charLevel = &gs.character.level;
    let charName = &gs.character.name;
    let currentAction = &gs.tavern.current_action;
    let curren_mount = &gs.character.mount;

    let miscDontPerformActionsFrom: String = fetch_character_setting(&gs, "miscDontPerformActionsFrom").unwrap_or("00:00".to_string());
    let miscDontPerformActionsTo: String = fetch_character_setting(&gs, "miscDontPerformActionsTo").unwrap_or("00:01".to_string());

    if (check_time_in_range(miscDontPerformActionsFrom.clone(), miscDontPerformActionsTo.clone()))
    {
        std::thread::sleep(std::time::Duration::from_millis(500));
        skip_with_reason!(gs, funcToExecute, format!("time in no-action range ({} - {})", miscDontPerformActionsFrom, miscDontPerformActionsTo));
    }

    match funcToExecute
    {
        "cmd_play_expeditions_gold" =>
        {
            let are_expeditions_enabled: bool = fetch_character_setting(&gs, "tavernPlayExpeditions").unwrap_or(false);
            let play_exp_expeditions: String = fetch_character_setting(&gs, "tavernPlayExpExpedition").unwrap_or("".to_string());
            if (!are_expeditions_enabled)
            {
                skip_with_reason!(gs, funcToExecute, "tavernPlayExpeditions=false");
            }

            if (curren_mount.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "no mount equipped");
            }

            if play_exp_expeditions == "tavernPlayExpExpeditionExp"
            {
                skip_with_reason!(gs, funcToExecute, "expedition type set to EXP, not GOLD");
            }

            let play_expedtion_from_str: String = fetch_character_setting(&gs, "tavernPlayExpeditionFrom").unwrap_or("00:00".to_string());
            let play_expedtion_from_time = NaiveTime::parse_from_str(&play_expedtion_from_str, "%H:%M").unwrap();
            let is_in_range = Local::now().time() > play_expedtion_from_time;
            if (!is_in_range)
            {
                skip_with_reason!(gs, funcToExecute, format!("current time before tavernPlayExpeditionFrom ({})", play_expedtion_from_str));
            }

            let beers_to_drink: i32 = std::cmp::min(fetch_character_setting(&gs, "tavernDrinkBeerAmount").unwrap_or(0), 12).max(0);
            let thirst_left = gs.tavern.thirst_for_adventure_sec;
            let no_thirst_left = thirst_left == 0;
            let max_beers = gs.tavern.beer_max;
            let beers_drunk = gs.tavern.beer_drunk;
            let target_beers = std::cmp::min(beers_to_drink as u8, max_beers);
            let beers_needed = target_beers.saturating_sub(beers_drunk);
            let not_enough_mushrooms_for_beers = gs.character.mushrooms < beers_needed as u32;
            let no_beer_left = beers_drunk >= target_beers;
            let nothing_left_todo = no_thirst_left && (no_beer_left || not_enough_mushrooms_for_beers);
            if nothing_left_todo && gs.tavern.current_action != CurrentAction::Expedition {
                skip_with_reason!(gs, funcToExecute, format!("nothing to do: thirst={}, beers={}/{}, max_beers_setting={}, shrooms={}, action={:?}",
                    thirst_left, beers_drunk, max_beers, beers_to_drink, gs.character.mushrooms, gs.tavern.current_action));
            }
            return false;
        }
        "cmd_play_expeditions_exp" =>
        {
            let are_expeditions_enabled: bool = fetch_character_setting(&gs, "tavernPlayExpeditions").unwrap_or(false);
            let play_gold_expeditions: String = fetch_character_setting(&gs, "tavernPlayExpExpedition").unwrap_or("".to_string());
            if (!are_expeditions_enabled)
            {
                skip_with_reason!(gs, funcToExecute, "tavernPlayExpeditions=false");
            }
            if (curren_mount.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "no mount equipped");
            }

            if (play_gold_expeditions == "tavernPlayExpExpeditionGold")
            {
                skip_with_reason!(gs, funcToExecute, "expedition type set to GOLD, not EXP");
            }

            let play_expedtion_from_str: String = fetch_character_setting(&gs, "tavernPlayExpeditionFrom").unwrap_or("00:00".to_string());
            let play_expedtion_from_time = NaiveTime::parse_from_str(&play_expedtion_from_str, "%H:%M").unwrap();
            let is_in_range = Local::now().time() > play_expedtion_from_time;
            if (!is_in_range)
            {
                skip_with_reason!(gs, funcToExecute, format!("current time before tavernPlayExpeditionFrom ({})", play_expedtion_from_str));
            }

            let beers_to_drink: i32 = std::cmp::min(fetch_character_setting(&gs, "tavernDrinkBeerAmount").unwrap_or(0), 12).max(0);
            let thirst_left = gs.tavern.thirst_for_adventure_sec;
            let no_thirst_left = thirst_left == 0;
            let max_beers = gs.tavern.beer_max;
            let beers_drunk = gs.tavern.beer_drunk;
            let target_beers = std::cmp::min(beers_to_drink as u8, max_beers);
            let beers_needed = target_beers.saturating_sub(beers_drunk);
            let not_enough_mushrooms_for_beers = gs.character.mushrooms < beers_needed as u32;
            let no_beer_left = beers_drunk >= target_beers;
            let nothing_left_todo = no_thirst_left && (no_beer_left || not_enough_mushrooms_for_beers);
            if nothing_left_todo && gs.tavern.current_action != CurrentAction::Expedition {
                skip_with_reason!(gs, funcToExecute, format!("nothing to do: thirst={}, beers={}/{}, max_beers_setting={}, shrooms={}, action={:?}",
                    thirst_left, beers_drunk, max_beers, beers_to_drink, gs.character.mushrooms, gs.tavern.current_action));
            }
            return false;
        }
        "cmd_play_idle_game" =>
        {
            let enable_arena_manager: bool = fetch_character_setting(&gs, "arenaManagerActive").unwrap_or(false);
            if (!enable_arena_manager)
            {
                skip_with_reason!(gs, funcToExecute, "arenaManagerActive=false");
            }

            if (charLevel < &105)
            {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 105 required", charLevel));
            }
            if gs.idle_game.is_none()
            {
                skip_with_reason!(gs, funcToExecute, "idle_game not unlocked");
            }
            let idle_game = &gs.idle_game.as_ref().unwrap();
            let current_runes = idle_game.current_runes.clone();
            let base = BigInt::from(10);
            let exponent = 151;
            let ingame_max_rune_limit = base.pow(exponent);
            if current_runes >= ingame_max_rune_limit
            {
                skip_with_reason!(gs, funcToExecute, "max rune limit reached");
            }

            let buildings = &idle_game.buildings;
            for building in buildings.values()
            {
                if idle_game.current_money > building.upgrade_cost
                {
                    return false;
                }
            }
            skip_with_reason!(gs, funcToExecute, "no building upgrades affordable");
        }
        "cmd_fight_pet_arena" =>
        {
            let enable_pet_arena: bool = fetch_character_setting(&gs, "petsDoFights").unwrap_or(false);
            if (!enable_pet_arena)
            {
                skip_with_reason!(gs, funcToExecute, "petsDoFights=false");
            }
            if (charLevel < &65)
            {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 65 required", charLevel));
            }
            let pet_fights = get_pets_left_for_pet_arena(&gs).len();
            if (pet_fights == 0)
            {
                skip_with_reason!(gs, funcToExecute, "no pet fights left");
            }
            return false;
        }
        "cmd_collect_gifts_from_mail" =>
        {
            let enable_mail_claiming: bool = fetch_character_setting(&gs, "quartersCollectMailRewards").unwrap_or(false);
            if (!enable_mail_claiming)
            {
                skip_with_reason!(gs, funcToExecute, "quartersCollectMailRewards=false");
            }

            for claimable in &gs.mail.claimables
            {
                if claimable.status != ClaimableStatus::Claimed && (claimable.typ == ClaimableMailType::TwitchDrop || claimable.typ == ClaimableMailType::Coupon)
                {
                    return false;
                }
            }
            skip_with_reason!(gs, funcToExecute, "no unclaimed TwitchDrop/Coupon in mail");
        }
        "cmd_city_guard" =>
        {
            let enable_city_guard: bool = fetch_character_setting(&gs, "tavernPlayCityGuard").unwrap_or(false);
            if !enable_city_guard {
                skip_with_reason!(gs, funcToExecute, "tavernPlayCityGuard=false");
            }
            return false;
        }
        "cmd_perform_underworld_atk_suggested_enemy" =>
        {
            let underworld = gs.underworld.clone();
            if underworld.is_none()
            {
                skip_with_reason!(gs, funcToExecute, "underworld not unlocked");
            }
            let underworld_buildings = underworld.clone().unwrap().buildings;
            let lures_today = underworld.unwrap().lured_today;
            let gate_level = underworld_buildings[UnderworldBuildingType::Gate].level;
            let keeper_level = underworld_buildings[UnderworldBuildingType::Keeper].level;
            let max_lures = if gate_level >= 5 { 5 } else { gate_level };
            if keeper_level < 1 {
                skip_with_reason!(gs, funcToExecute, "keeper not built");
            }
            if lures_today >= max_lures as u16 {
                skip_with_reason!(gs, funcToExecute, format!("max lures reached ({}/{})", lures_today, max_lures));
            }
            return false;
        }
        "cmd_fight_pet_dungeon" =>
        {
            let enable_pet_dungeon: bool = fetch_character_setting(&gs, "petsDoDungeons").unwrap_or(false);
            if (!enable_pet_dungeon)
            {
                skip_with_reason!(gs, funcToExecute, "petsDoDungeons=false");
            }
            let pets = gs.pets.clone();
            if (pets.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "pets not unlocked");
            }
            if (pets.clone().unwrap().next_free_exploration > Some(Local::now()))
            {
                skip_with_reason!(gs, funcToExecute, "exploration still running");
            }
            if (pets.unwrap().any_habitat_unfinished())
            {
                return false;
            }
            skip_with_reason!(gs, funcToExecute, "no unfinished habitat");
        }
        "cmd_fight_hydra" =>
        {
            let enable_hydra_fight: bool = fetch_character_setting(&gs, "quartersSignUpHydra").unwrap_or(false);
            if (!enable_hydra_fight)
            {
                skip_with_reason!(gs, funcToExecute, "quartersSignUpHydra=false");
            }

            let guild = gs.guild.clone();
            if (guild.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "no guild");
            }

            let members = guild.clone().unwrap().members;
            let member_count = members.len();
            let hydra = guild.clone().unwrap().hydra;
            let mut leader_level_ok: bool = false;

            for member in members.iter()
            {
                if (member.guild_rank == GuildRank::Leader && member.level >= 150)
                {
                    leader_level_ok = true;
                }
            }
            if (!leader_level_ok)
            {
                skip_with_reason!(gs, funcToExecute, "guild leader level < 150");
            }
            if member_count < 10 {
                skip_with_reason!(gs, funcToExecute, format!("guild too small ({}/10 members)", member_count));
            }
            if guild.clone().unwrap().pet_id == 0 {
                skip_with_reason!(gs, funcToExecute, "no guild pet");
            }
            if hydra.remaining_fights <= 0 {
                skip_with_reason!(gs, funcToExecute, "no hydra fights remaining");
            }
            return false;
        }
        "cmd_fight_demon_portal" =>
        {
            let enable_demon_portal: bool = fetch_character_setting(&gs, "dungeonFightDemonPortal").unwrap_or(false);
            if (!enable_demon_portal)
            {
                skip_with_reason!(gs, funcToExecute, "dungeonFightDemonPortal=false");
            }
            if (gs.character.level < 99)
            {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 99 required", gs.character.level));
            }
            if (gs.dungeons.portal.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "portal not unlocked");
            }
            let portal = gs.dungeons.portal.clone().unwrap();
            if (portal.finished >= 50)
            {
                skip_with_reason!(gs, funcToExecute, "portal finished (50/50)");
            }
            if !portal.can_fight {
                skip_with_reason!(gs, funcToExecute, "cannot fight portal now");
            }
            return false;
        }
        "cmd_fight_guild_portal" =>
        {
            let enable_guild_portal: bool = fetch_character_setting(&gs, "quarterFightDungeonPortal").unwrap_or(false);
            if (!enable_guild_portal)
            {
                skip_with_reason!(gs, funcToExecute, "quarterFightDungeonPortal=false");
            }
            let name = gs.character.name.clone();
            let guild = &gs.guild;
            if (guild.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "no guild");
            }
            if (charLevel < &99)
            {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 99 required", charLevel));
            }

            let defeat_count = &guild.clone().unwrap().portal.defeated_count;
            if (*defeat_count == 50)
            {
                skip_with_reason!(gs, funcToExecute, "guild portal finished (50/50)");
            }

            let guild_members = &guild.clone().unwrap().members;
            for x in guild_members.iter()
            {
                if (x.name == name)
                {
                    if let Some(last_fought_datee) = x.portal_fought
                    {
                        if !(last_fought_datee.date_naive() < Local::now().date_naive()) {
                            skip_with_reason!(gs, funcToExecute, "already fought guild portal today");
                        }
                        return false;
                    }
                }
            }
            return false;
        }
        "cmd_fight_dungeon_with_lowest_level" =>
        {
            let enable_dungeon = fetch_character_setting(&gs, "dungeonCheckbox").unwrap_or(false);
            if (!enable_dungeon)
            {
                skip_with_reason!(gs, funcToExecute, "dungeonCheckbox=false");
            }
            if let Some(next_free_fight) = gs.dungeons.next_free_fight
            {
                let extra_delay = chrono::Duration::minutes(5);
                let earliest_fight_time = next_free_fight + extra_delay;
                let is_fight_free = Local::now() >= earliest_fight_time;
                if !is_fight_free {
                    skip_with_reason!(gs, funcToExecute, format!("dungeon on cooldown until {}", earliest_fight_time.format("%H:%M:%S")));
                }
                if gs.character.inventory.count_free_slots() == 0 {
                    skip_with_reason!(gs, funcToExecute, "inventory full");
                }
                return false;
            }
            skip_with_reason!(gs, funcToExecute, "no next_free_fight time available");
        }
        "cmd_arena_fight" =>
        {
            let enable_arena: bool = fetch_character_setting(&gs, "arenaCheckbox").unwrap_or(false);
            let stop_after_ten_won_fights: bool = fetch_character_setting(&gs, "arenaStopWhenDone").unwrap_or(false);
            let max_fights_for_exp = 10;
            if (!enable_arena)
            {
                skip_with_reason!(gs, funcToExecute, "arenaCheckbox=false");
            }

            let arena = &gs.arena.clone();

            if stop_after_ten_won_fights && arena.fights_for_xp == max_fights_for_exp
            {
                skip_with_reason!(gs, funcToExecute, "10 daily arena fights completed (arenaStopWhenDone=true)");
            }

            let current_time = Local::now();
            let current_time_minus_3 = current_time - Duration::minutes(3);

            if let Some(next_free_fight) = arena.next_free_fight
            {
                if next_free_fight >= current_time_minus_3
                {
                    skip_with_reason!(gs, funcToExecute, format!("arena on cooldown until {}", next_free_fight.format("%H:%M:%S")));
                }
            }
            return false;
        }
        "cmd_enchant_items" =>
        {
            let witch = &gs.witch;
            if gs.character.level < 66 {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 66 required", gs.character.level));
            }
            if witch.is_none()
            {
                skip_with_reason!(gs, funcToExecute, "witch not unlocked");
            }

            let equipment = &gs.character.equipment.0;
            for (slot, item) in equipment.iter()
            {
                if (should_enchant(&gs, slot))
                {
                    if let Some(item) = item
                    {
                        if item.enchantment.is_some()
                        {
                            continue;
                        }
                        else
                        {
                            if let Some(optional_witch) = witch
                            {
                                if item.enchantment.is_none()
                                {
                                    if let Some(slot_enchantment) = slot.enchantment()
                                    {
                                        let enchantment_indent = optional_witch.enchantments[slot_enchantment];
                                        if let Some(enchantment_indent_unwrapped) = enchantment_indent
                                        {
                                            if (gs.character.silver > optional_witch.enchantment_price)
                                            {
                                                return false;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            skip_with_reason!(gs, funcToExecute, "no items need enchanting or not enough silver");
        }
        "cmd_spin_lucky_wheel" =>
        {
            let enable_lucky_wheel: bool = fetch_character_setting(&gs, "quartersSpinLuckyWheel").unwrap_or(false);
            if (!enable_lucky_wheel)
            {
                skip_with_reason!(gs, funcToExecute, "quartersSpinLuckyWheel=false");
            }
            let mut wheel_spins: i32 = fetch_character_setting(&gs, "quartersSpinLuckyWithResourcesAmount").unwrap_or(1);
            let resource_to_spin_with: String = fetch_character_setting(&gs, "quartersSpinLuckyWithResources").unwrap_or("".to_string());
            let events = &gs.specials.events.active;
            let spin_cost_lucky_coins = 10;
            let max_spins = if events.contains(&Event::LuckyDay) { 40 } else { 20 };
            if (wheel_spins > max_spins)
            {
                wheel_spins = max_spins;
            }
            if (gs.specials.wheel.clone().spins_today < 1)
            {
                return false;
            }

            if (&gs.specials.wheel.lucky_coins < &spin_cost_lucky_coins && get_resource_from_setting(&*resource_to_spin_with) == FortunePayment::LuckyCoins)
            {
                skip_with_reason!(gs, funcToExecute, format!("not enough lucky coins ({}<{})", gs.specials.wheel.lucky_coins, spin_cost_lucky_coins));
            }
            if (&gs.character.mushrooms == &0 && get_resource_from_setting(&*resource_to_spin_with) == FortunePayment::Mushrooms)
            {
                skip_with_reason!(gs, funcToExecute, "no mushrooms for wheel spin");
            }

            if (gs.specials.wheel.clone().spins_today < wheel_spins as u8)
            {
                return false;
            }
            skip_with_reason!(gs, funcToExecute, format!("already spun today ({}/{})", gs.specials.wheel.spins_today, wheel_spins));
        }
        "cmd_attack_fortress" =>
        {
            let enable_fortress_attacks: bool = fetch_character_setting(&gs, "fortressDoAttacks").unwrap_or(false);
            if (!enable_fortress_attacks)
            {
                skip_with_reason!(gs, funcToExecute, "fortressDoAttacks=false");
            }

            let fortress_option = &gs.fortress;
            if (fortress_option.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "fortress not unlocked");
            }
            let fortress_unwrapped = fortress_option.clone().unwrap();
            if (fortress_unwrapped.buildings[FortressBuildingType::Barracks].level == 0)
            {
                skip_with_reason!(gs, funcToExecute, "barracks not built");
            }
            let available_soldiers = fortress_unwrapped.units[FortressUnitType::Soldier].count;
            if available_soldiers == 0
            {
                skip_with_reason!(gs, funcToExecute, "no soldiers available");
            }
            return false;
        }
        "cmd_accept_unlockables" =>
        {
            let unlocks = &gs.pending_unlocks.clone();
            if unlocks.len() == 0 {
                skip_with_reason!(gs, funcToExecute, "no pending unlocks");
            }
            return false;
        }
        "cmd_collect_fortress_resources" =>
        {
            let fortress_collect_wood: bool = fetch_character_setting(&gs, "collectWood").unwrap_or(false);
            let fortress_collect_stone: bool = fetch_character_setting(&gs, "collectStone").unwrap_or(false);
            let fortress_collect_exp: bool = fetch_character_setting(&gs, "collectExp").unwrap_or(false);
            let collect_resources_from: String = fetch_character_setting(&gs, "fortressCollectTimeFrom").unwrap_or("00:00".to_string());
            let collect_resources_to: String = fetch_character_setting(&gs, "fortressCollectTimeTo").unwrap_or("00:00".to_string());
            let is_in_range = check_time_in_range(collect_resources_from.clone(), collect_resources_to.clone());
            if (!is_in_range)
            {
                skip_with_reason!(gs, funcToExecute, format!("not in time range ({} - {})", collect_resources_from, collect_resources_to));
            }

            if fortress_collect_wood || fortress_collect_stone || fortress_collect_exp
            {
                return false;
            }
            skip_with_reason!(gs, funcToExecute, "no resource collection enabled (collectWood/Stone/Exp all false)");
        }
        "cmd_manage_inventory" =>
        {
            let enable_inventory_management: bool = fetch_character_setting(&gs, "itemsCheckbox").unwrap_or(false);
            if !enable_inventory_management {
                skip_with_reason!(gs, funcToExecute, "itemsCheckbox=false");
            }
            return false;
        }
        "cmd_start_searching_for_gem" =>
        {
            let enable_gem_search: bool = fetch_character_setting(&gs, "fortessSearchForGems").unwrap_or(false);
            let gold_to_keep: i64 = fetch_character_setting(&gs, "itemsInventoryMinGoldSaved").unwrap_or(0) * 100;
            let ignore_min_gold: bool = fetch_character_setting(&gs, "itemsInventoryMinGoldSavedIgnoreGemMine").unwrap_or(false);
            let fortress_option = &gs.fortress;
            if (fortress_option.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "fortress not unlocked");
            }

            let fortress_unwrapped = fortress_option.clone().unwrap();
            let fortress_gem_search_cost = fortress_unwrapped.gem_search.cost.silver;
            if (gold_to_keep > fortress_gem_search_cost as i64 && !ignore_min_gold)
            {
                skip_with_reason!(gs, funcToExecute, format!("gold_to_keep ({}) > gem_search_cost ({})", gold_to_keep, fortress_gem_search_cost));
            }

            if !enable_gem_search {
                skip_with_reason!(gs, funcToExecute, "fortessSearchForGems=false");
            }
            return false;
        }
        "cmd_train_fortress_units" =>
        {
            let train_soldiers: bool = fetch_character_setting(&gs, "fortessTrainSoldiers").unwrap_or(false);
            let train_archers: bool = fetch_character_setting(&gs, "fortessTrainArchers").unwrap_or(false);
            let train_mages: bool = fetch_character_setting(&gs, "fortessTrainMages").unwrap_or(false);

            if (train_mages)
            {
                return false;
            }
            if (train_soldiers)
            {
                return false;
            }
            if (train_archers)
            {
                return false;
            }
            skip_with_reason!(gs, funcToExecute, "no unit training enabled (fortessTrainSoldiers/Archers/Mages all false)");
        }
        "cmd_collect_underworld_resources" =>
        {
            let enable_soul_collection: bool = fetch_character_setting(&gs, "underworldCollectSouls").unwrap_or(false);
            let enable_gold_collection: bool = fetch_character_setting(&gs, "underworldCollectGold").unwrap_or(false);
            let enable_thirst_collection: bool = fetch_character_setting(&gs, "underworldCollectThirst").unwrap_or(false);

            if (enable_gold_collection)
            {
                return false;
            }
            if (enable_thirst_collection)
            {
                return false;
            }
            if (enable_soul_collection)
            {
                return false;
            }
            skip_with_reason!(gs, funcToExecute, "no underworld collection enabled (underworldCollectSouls/Gold/Thirst all false)");
        }
        "cmd_build_underworld_perfect_order" =>
        {
            let enable_underworld_upgrades: bool = fetch_character_setting(&gs, "underworldUpgradeBuildings").unwrap_or(false);
            if !enable_underworld_upgrades {
                skip_with_reason!(gs, funcToExecute, "underworldUpgradeBuildings=false");
            }
            return false;
        }

        "cmd_check_and_swap_equipment" =>
        {
            skip_with_reason!(gs, funcToExecute, "feature disabled (hardcoded)");
            // remove once its fixed
            let enable_equip_swapping: bool = fetch_character_setting(&gs, "itemsEnableEquipmentSwap").unwrap_or(false);
            if !enable_equip_swapping {
                skip_with_reason!(gs, funcToExecute, "itemsEnableEquipmentSwap=false");
            }
            return false;
        }
        "cmd_buy_mount" =>
        {
            let enable_mount_buying: bool = fetch_character_setting(&gs, "enableBuyingMount").unwrap_or(false);
            if (gs.character.mount.is_some())
            {
                skip_with_reason!(gs, funcToExecute, "already has mount");
            }

            if !enable_mount_buying {
                skip_with_reason!(gs, funcToExecute, "enableBuyingMount=false");
            }
            return false;
        }
        "cmd_play_dice" =>
        {
            let enable_dice_game: bool = fetch_character_setting(&gs, "tavernPlayDiceGame").unwrap_or(false);
            let pets = gs.pets.clone();
            if (pets.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "pets not unlocked");
            }
            let tower = gs.dungeons.light[LightDungeon::Tower];
            match tower
            {
                DungeonProgress::Locked =>
                {
                    skip_with_reason!(gs, funcToExecute, "tower not unlocked");
                }
                _ =>
                {}
            }
            let remaining = &gs.tavern.dice_game.clone().remaining;
            if (*remaining == 0)
            {
                skip_with_reason!(gs, funcToExecute, "no dice rolls remaining");
            }

            if !enable_dice_game {
                skip_with_reason!(gs, funcToExecute, "tavernPlayDiceGame=false");
            }
            return false;
        }

        "cmd_build_fortress_our_order" =>
        {
            let enable_building_fortress: bool = fetch_character_setting(&gs, "fortessUpgradeOurOrder").unwrap_or(false);
            if !enable_building_fortress {
                skip_with_reason!(gs, funcToExecute, "fortessUpgradeOurOrder=false");
            }
            return false;
        }
        "cmd_sign_up_for_guild_attack_and_defense" =>
        {
            if (gs.guild.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "no guild");
            }
            return false;
        }
        "cmd_upgrade_skill_points" =>
        {
            let enable_skill_buying: bool = fetch_character_setting(&gs, "characterIncreaseStatAttributes").unwrap_or(false);
            if !enable_skill_buying {
                skip_with_reason!(gs, funcToExecute, "characterIncreaseStatAttributes=false");
            }
            return false;
        }
        "cmd_brew_potions_using_fruits" =>
        {
            let enable_potion_brewing: bool = fetch_character_setting(&gs, "itemsBrewPotionsUsingFruits").unwrap_or(false);
            if (charLevel < &632)
            {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 632 required", charLevel));
            }
            if !enable_potion_brewing {
                skip_with_reason!(gs, funcToExecute, "itemsBrewPotionsUsingFruits=false");
            }
            return false;
        }
        "cmd_level_up_uw_keeper" =>
        {
            let enable_keeper_upgrade: bool = fetch_character_setting(&gs, "underworldUpgradeKeeper").unwrap_or(false);
            let souls_keep: u64 = fetch_character_setting(&gs, "underworldUpgradeKeeperSoulsToKeep").unwrap_or(0);
            if gs.underworld.is_none()
            {
                skip_with_reason!(gs, funcToExecute, "underworld not unlocked");
            }
            let underworld = match &gs.underworld
            {
                None => {
                    skip_with_reason!(gs, funcToExecute, "underworld not unlocked");
                },
                Some(uw) => uw,
            };

            let current_soul_count = underworld.souls_current;
            let char_silver = gs.character.silver;

            let keeper = &underworld.units[UnderworldUnitType::Keeper];
            if (keeper.level <= 0)
            {
                skip_with_reason!(gs, funcToExecute, "keeper not built");
            }

            if souls_keep > current_soul_count
            {
                skip_with_reason!(gs, funcToExecute, format!("souls_keep ({}) > current souls ({})", souls_keep, current_soul_count));
            }
            let souls_we_can_spend = current_soul_count - souls_keep;

            if keeper.upgrade_cost.souls > souls_we_can_spend {
                skip_with_reason!(gs, funcToExecute, format!("not enough souls for upgrade ({} needed, {} available)", keeper.upgrade_cost.souls, souls_we_can_spend));
            }
            if keeper.upgrade_cost.silver > char_silver {
                skip_with_reason!(gs, funcToExecute, format!("not enough silver for upgrade ({} needed, {} available)", keeper.upgrade_cost.silver, char_silver));
            }

            if !enable_keeper_upgrade {
                skip_with_reason!(gs, funcToExecute, "underworldUpgradeKeeper=false");
            }
            return false;
        }
        "cmd_feed_all_pets" =>
        {
            let do_feed_pets: bool = fetch_character_setting(&gs, "petsDoFeed").unwrap_or(false);
            if (charLevel < &75)
            {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 75 required", charLevel));
            }

            if (gs.pets.is_none())
            {
                skip_with_reason!(gs, funcToExecute, "pets not unlocked");
            }

            if let Some(unwrapped_pets) = &gs.pets
            {
                let habitats = &unwrapped_pets.habitats;
                for (hab_type, habitat) in habitats
                {
                    let pets_in_habitat = &habitat.pets;
                    for i in pets_in_habitat.iter()
                    {
                        if i.level < unwrapped_pets.max_pet_level
                        {
                            if !do_feed_pets {
                                skip_with_reason!(gs, funcToExecute, "petsDoFeed=false");
                            }
                            return false;
                        }
                    }
                }
            }
            skip_with_reason!(gs, funcToExecute, "all pets at max level");
        }
        "cmd_play_hellevator" =>
        {
            let do_play_helle: bool = fetch_character_setting(&gs, "quartersDoPlayHellevator").unwrap_or(false);
            if (gs.character.level < 10)
            {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 10 required", gs.character.level));
            }
            if gs.guild.is_none()
            {
                skip_with_reason!(gs, funcToExecute, "no guild");
            }

            match gs.hellevator.status()
            {
                HellevatorStatus::RewardClaimable =>
                {}
                HellevatorStatus::NotEntered =>
                {}
                HellevatorStatus::NotAvailable =>
                {
                    skip_with_reason!(gs, funcToExecute, "hellevator not available");
                }
                _ =>
                {}
            };

            if !do_play_helle {
                skip_with_reason!(gs, funcToExecute, "quartersDoPlayHellevator=false");
            }
            return false;
        }
        "cmd_fill_scrapbook" =>
        {
            let max_items = 1682;
            let enable_arena: bool = fetch_character_setting(&gs, "arenaCheckbox").unwrap_or(false);
            let stop_after_ten_won_fights: bool = fetch_character_setting(&gs, "arenaStopWhenDone").unwrap_or(false);
            let fill_scrapbook: bool = fetch_character_setting(&gs, "arenaFillScrapbook").unwrap_or(false);
            if (gs.character.level < 10)
            {
                skip_with_reason!(gs, funcToExecute, format!("level {} < 10 required", gs.character.level));
            }
            let max_fights_for_exp = 10;
            if (!enable_arena)
            {
                skip_with_reason!(gs, funcToExecute, "arenaCheckbox=false");
            }
            let arena = &gs.arena.clone();
            if stop_after_ten_won_fights && arena.fights_for_xp == max_fights_for_exp
            {
                skip_with_reason!(gs, funcToExecute, "10 daily arena fights completed (arenaStopWhenDone=true)");
            }
            if let Some(s) = gs.character.scrapbook.as_ref()
            {
                if (s.items.len() == max_items)
                {
                    skip_with_reason!(gs, funcToExecute, "scrapbook complete");
                }
            }
            if !fill_scrapbook {
                skip_with_reason!(gs, funcToExecute, "arenaFillScrapbook=false");
            }
            return false;
        }

        _ =>
        {}
    }
    false
}
