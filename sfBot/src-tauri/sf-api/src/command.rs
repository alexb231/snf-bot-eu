#![allow(deprecated)]
use enum_map::Enum;
use log::warn;
use num_derive::FromPrimitive;
use strum::EnumIter;

use crate::{
    PlayerId,
    gamestate::{
        ShopPosition,
        character::*,
        dungeons::{CompanionClass, Dungeon},
        fortress::*,
        guild::{Emblem, GuildSkill},
        idle::IdleBuildingType,
        items::*,
        legendary_dungeon::{
            DoorType, DungeonEffectType, GemOfFateType,
            LegendaryDungeonEventTheme, RPSChoice,
        },
        social::Relationship,
        underworld::*,
        unlockables::*,
    },
};


#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Command {
    
    
    
    Custom {
        
        cmd_name: String,
        
        
        arguments: Vec<String>,
    },
    
    
    
    
    #[deprecated = "Use the login method instead"]
    Login {
        
        username: String,
        
        pw_hash: String,
        
        
        
        login_count: u32,
    },
    
    
    
    
    #[cfg(feature = "sso")]
    #[deprecated = "Use a login method instead"]
    SSOLogin {
        
        uuid: String,
        
        character_id: String,
        
        bearer_token: String,
    },
    
    
    
    #[deprecated = "Use the register method instead"]
    Register {
        
        username: String,
        
        password: String,
        
        gender: Gender,
        
        race: Race,
        
        class: Class,
    },
    
    
    
    Update,
    
    
    
    
    
    
    
    
    
    
    
    HallOfFamePage {
        
        
        
        page: usize,
    },
    
    
    HallOfFameFortressPage {
        
        
        
        page: usize,
    },
    
    
    
    ViewPlayer {
        
        ident: String,
    },
    
    BuyBeer,
    
    StartQuest {
        
        quest_pos: usize,
        
        
        overwrite_inv: bool,
    },
    
    CancelQuest,
    
    
    FinishQuest {
        
        
        skip: Option<TimeSkip>,
    },
    
    StartWork {
        
        hours: u8,
    },
    
    CancelWork,
    
    FinishWork,
    
    CheckNameAvailable {
        
        name: String,
    },
    
    BuyMount {
        
        mount: Mount,
    },
    
    
    IncreaseAttribute {
        
        attribute: AttributeType,
        
        increase_to: u32,
    },
    
    RemovePotion {
        
        pos: usize,
    },
    
    CheckArena,
    
    
    
    Fight {
        
        name: String,
        
        
        
        
        use_mushroom: bool,
    },
    
    CollectCalendar,
    
    CollectAdventsCalendar,
    
    
    ViewGuild {
        
        guild_ident: String,
    },
    
    GuildFound {
        
        name: String,
    },
    
    GuildInvitePlayer {
        
        name: String,
    },
    
    GuildKickPlayer {
        
        name: String,
    },
    
    GuildSetLeader {
        
        name: String,
    },
    
    GuildToggleOfficer {
        
        name: String,
    },
    
    GuildLoadMushrooms,
    
    
    GuildIncreaseSkill {
        
        skill: GuildSkill,
        
        current: u16,
    },
    
    GuildJoinAttack,
    
    GuildJoinDefense,
    
    GuildAttack {
        
        guild: String,
    },
    
    GuildRaid,
    
    GuildPortalBattle,
    
    GuildGetFightableTargets,
    
    ToiletFlush,
    
    ToiletOpen,
    
    ToiletDrop {
        
        
        
        item_pos: PlayerItemPosition,
    },
    
    BuyShop {
        
        
        shop_pos: ShopPosition,
        
        
        
        new_pos: PlayerItemPosition,
        
        
        
        item_ident: ItemCommandIdent,
    },
    
    
    
    BuyShopHourglas {
        
        shop_type: ShopType,
        
        shop_pos: usize,
    },
    
    
    SellShop {
        
        
        
        item_pos: PlayerItemPosition,
        
        
        
        item_ident: ItemCommandIdent,
    },
    
    PlayerItemMove {
        
        
        
        from: PlayerItemPosition,
        
        
        
        to: PlayerItemPosition,
        
        
        
        item_ident: ItemCommandIdent,
    },
    
    
    
    ItemMove {
        
        from: ItemPosition,
        
        to: ItemPosition,
        
        
        
        item_ident: ItemCommandIdent,
    },
    
    UsePotion {
        
        from: ItemPosition,
        
        
        
        item_ident: ItemCommandIdent,
    },
    
    MessageOpen {
        
        pos: i32,
    },
    
    MessageDelete {
        
        
        pos: i32,
    },
    
    ViewScrapbook,
    
    
    ViewPet {
        
        pet_id: u16,
    },
    
    
    UnlockFeature {
        
        unlockable: Unlockable,
    },
    
    FightPortal,
    
    
    
    
    
    
    
    UpdateDungeons,
    
    
    FightDungeon {
        
        
        
        dungeon: Dungeon,
        
        
        
        use_mushroom: bool,
    },
    
    FightTower {
        
        current_level: u8,
        
        
        
        use_mush: bool,
    },
    
    FightPetOpponent {
        
        habitat: HabitatType,
        
        opponent_id: PlayerId,
    },
    
    FightPetDungeon {
        
        
        
        use_mush: bool,
        
        habitat: HabitatType,
        
        
        enemy_pos: u32,
        
        
        
        player_pet_id: u32,
    },
    
    
    BrewPotion {
        fruit_type: HabitatType,
    },
    
    
    GuildSetInfo {
        
        description: String,
        
        emblem: Emblem,
    },
    
    
    
    GambleSilver {
        
        amount: u64,
    },
    
    
    
    GambleMushrooms {
        
        amount: u64,
    },
    
    SendMessage {
        
        to: String,
        
        msg: String,
    },
    
    
    
    
    
    
    
    
    SetDescription {
        
        description: String,
    },
    
    WitchDropCauldron {
        
        
        
        item_pos: PlayerItemPosition,
    },
    
    Blacksmith {
        
        
        
        item_pos: PlayerItemPosition,
        
        action: BlacksmithAction,
        
        
        
        item_ident: ItemCommandIdent,
    },
    
    GuildSendChat {
        
        message: String,
    },
    
    
    WitchEnchant {
        
        enchantment: EnchantmentIdent,
    },
    
    
    WitchEnchantCompanion {
        
        enchantment: EnchantmentIdent,
        
        companion: CompanionClass,
    },
    
    
    
    
    UpdateLureSuggestion,
    
    
    
    ViewLureSuggestion {
        
        suggestion: LureSuggestion,
    },
    
    
    SpinWheelOfFortune {
        
        payment: FortunePayment,
    },
    
    CollectEventTaskReward {
        
        pos: usize,
    },
    
    CollectDailyQuestReward {
        
        pos: usize,
    },
    
    
    
    Equip {
        
        
        from_pos: PlayerItemPosition,
        
        to_slot: EquipmentSlot,
        
        
        
        item_ident: ItemCommandIdent,
    },
    
    EquipCompanion {
        
        
        from_pos: PlayerItemPosition,
        
        to_slot: EquipmentSlot,
        
        
        
        item_ident: ItemCommandIdent,
        
        to_companion: CompanionClass,
    },
    
    FortressGather {
        
        resource: FortressResourceType,
    },
    
    FortressChangeEnemy {
        
        
        msg_id: i64,
    },
    
    
    
    FortressGatherSecretStorage {
        
        stone: u64,
        
        wood: u64,
    },
    
    FortressBuild {
        
        f_type: FortressBuildingType,
    },
    
    
    FortressBuildCancel {
        
        f_type: FortressBuildingType,
    },
    
    
    
    
    
    FortressBuildFinish {
        f_type: FortressBuildingType,
        mushrooms: u32,
    },
    
    FortressBuildUnit {
        unit: FortressUnitType,
        count: u32,
    },
    
    FortressGemStoneSearch,
    
    FortressGemStoneSearchCancel,
    
    
    
    FortressGemStoneSearchFinish {
        mushrooms: u32,
    },
    
    
    FortressAttack {
        soldiers: u32,
    },
    
    FortressNewEnemy {
        use_mushroom: bool,
    },
    
    FortressSetCAEnemy {
        msg_id: u32,
    },
    
    FortressUpgradeHallOfKnights,
    
    FortressUpgradeUnit {
        
        unit: FortressUnitType,
    },
    
    Whisper {
        player_name: String,
        message: String,
    },
    
    UnderworldCollect {
        resource: UnderworldResourceType,
    },
    
    UnderworldUnitUpgrade {
        unit: UnderworldUnitType,
    },
    
    UnderworldUpgradeStart {
        building: UnderworldBuildingType,
        mushrooms: u32,
    },
    
    UnderworldUpgradeCancel {
        building: UnderworldUnitType,
    },
    
    
    UnderworldUpgradeFinish {
        building: UnderworldBuildingType,
        mushrooms: u32,
    },
    
    UnderworldAttack {
        player_id: PlayerId,
    },
    
    
    RollDice {
        payment: RollDicePrice,
        dices: [DiceType; 5],
    },
    
    PetFeed {
        pet_id: u32,
        fruit_idx: u32,
    },
    
    GuildPetBattle {
        use_mushroom: bool,
    },
    
    IdleUpgrade {
        typ: IdleBuildingType,
        amount: IdleUpgradeAmount,
    },
    
    IdleSacrifice,
    
    
    UpgradeSkill {
        attribute: AttributeType,
        next_attribute: u32,
    },
    
    RefreshShop {
        shop: ShopType,
    },
    
    HallOfFameGroupPage {
        page: u32,
    },
    
    HallOfFameUnderworldPage {
        page: u32,
    },
    HallOfFamePetsPage {
        page: u32,
    },
    
    SwapMannequin,
    
    UpdateFlag {
        flag: Option<Flag>,
    },
    
    BlockGuildInvites {
        block_invites: bool,
    },
    
    ShowTips {
        show_tips: bool,
    },
    
    
    ChangePassword {
        username: String,
        old: String,
        new: String,
    },
    
    ChangeMailAddress {
        old_mail: String,
        new_mail: String,
        password: String,
        username: String,
    },
    
    
    
    
    
    SetLanguage {
        language: String,
    },
    
    SetPlayerRelation {
        player_id: PlayerId,
        relation: Relationship,
    },
    
    
    SetPortraitFrame {
        portrait_id: i64,
    },
    
    SwapRunes {
        from: ItemPlace,
        from_pos: usize,
        to: ItemPlace,
        to_pos: usize,
    },
    
    
    
    
    ChangeItemLook {
        inv: ItemPlace,
        pos: usize,
        raw_model_id: u16,
    },
    
    ExpeditionPickEncounter {
        
        pos: usize,
    },
    
    
    
    
    ExpeditionContinue,
    
    
    ExpeditionPickReward {
        
        pos: usize,
    },
    
    ExpeditionStart {
        
        pos: usize,
    },
    
    LegendaryDungeonEnter {
        theme: LegendaryDungeonEventTheme,
    },
    
    LegendaryDungeonBuyCurse {
        effect: DungeonEffectType,
        keys: u32,
    },
    
    LegendaryDungeonBuyBlessing {
        effect: DungeonEffectType,
        keys: u32,
    },
    
    
    LegendaryDungeonEncounterInteract,
    
    
    LegendaryDungeonEncounterEscape,
    
    
    LegendaryDungeonEncounterLeave,
    LegendaryDungeonMerchantNewGoods,
    
    LegendaryDungeonRoomLeave,
    
    
    LegendaryDungeonPlayRPC {
        choice: RPSChoice,
    },
    LegendaryDungeonTakeItem {
        
        
        item_idx: usize,
        
        inventory_to: PlayerItemPosition,
        
        
        
        item_ident: ItemCommandIdent,
    },
    
    
    
    LegendaryDungeonRoomInteract,
    
    
    
    
    
    LegendaryDungeonForcedContinue,
    
    
    LegendaryDungeonMonsterCollectKey,
    
    LegendaryDungeonPickDoor {
        
        pos: usize,
        
        typ: DoorType,
    },
    LegendaryDungeonPickGem {
        gem_type: GemOfFateType,
    },
    LegendaryDungeonInteract {
        val: usize,
    },
    
    
    ExpeditionSkipWait {
        
        typ: TimeSkip,
    },
    
    
    
    
    
    
    SetQuestsInsteadOfExpeditions {
        
        value: ExpeditionSetting,
    },
    HellevatorEnter,
    HellevatorViewGuildRanking,
    HellevatorFight {
        use_mushroom: bool,
    },
    HellevatorBuy {
        position: usize,
        typ: HellevatorTreatType,
        price: u32,
        use_mushroom: bool,
    },
    HellevatorRefreshShop,
    HellevatorJoinHellAttack {
        use_mushroom: bool,
        plain: usize,
    },
    HellevatorClaimDaily,
    HellevatorClaimDailyYesterday,
    HellevatorClaimFinal,
    HellevatorPreviewRewards,
    HallOfFameHellevatorPage {
        page: usize,
    },
    ClaimablePreview {
        msg_id: i64,
    },
    ClaimableClaim {
        msg_id: i64,
    },
    
    BuyGoldFrame,
    LegendaryDungeonMonsterFight,
    LegendaryDungeonMonsterEscape,
}


#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExpeditionSetting {
    
    
    
    
    PreferExpeditions,
    
    
    
    #[default]
    PreferQuests,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlacksmithAction {
    Dismantle = 201,
    SocketUpgrade = 202,
    SocketUpgradeWithMushrooms = 212,
    GemExtract = 203,
    GemExtractWithMushrooms = 213,
    Upgrade = 204,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FortunePayment {
    LuckyCoins = 0,
    Mushrooms,
    FreeTurn,
}


#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RollDicePrice {
    Free = 0,
    Mushrooms,
    Hourglass,
}


#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum DiceType {
    
    
    
    ReRoll,
    Silver,
    Stone,
    Wood,
    Souls,
    Arcane,
    Hourglass,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiceReward {
    
    pub win_typ: DiceType,
    
    pub amount: u32,
}


#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Enum, FromPrimitive, Hash, EnumIter,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum AttributeType {
    Strength = 1,
    Dexterity = 2,
    Intelligence = 3,
    Constitution = 4,
    Luck = 5,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, EnumIter, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum ShopType {
    #[default]
    Weapon = 3,
    Magic = 4,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum TimeSkip {
    Mushroom = 1,
    Glass = 2,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum IdleUpgradeAmount {
    
    Max = -1,
    
    One = 1,
    
    Ten = 10,
    
    TwentyFive = 25,
    
    Hundred = 100,
}

impl Command {
    
    
    #[allow(deprecated, clippy::useless_format)]
    #[cfg(feature = "session")]
    pub(crate) fn request_string(
        &self,
    ) -> Result<String, crate::error::SFError> {
        const APP_VERSION: &str = "295000000000";
        use crate::{
            error::SFError,
            gamestate::dungeons::{LightDungeon, ShadowDungeon},
            misc::{HASH_CONST, sha1_hash, to_sf_string},
        };

        Ok(match self {
            Command::Custom {
                cmd_name,
                arguments: values,
            } => {
                format!("{cmd_name}:{}", values.join("/"))
            }
            Command::Login {
                username,
                pw_hash,
                login_count,
            } => {
                let full_hash = sha1_hash(&format!("{pw_hash}{login_count}"));
                format!(
                    "AccountLogin:{username}/{full_hash}/{login_count}/\
                     unity3d_webglplayer//{APP_VERSION}///0/"
                )
            }
            #[cfg(feature = "sso")]
            Command::SSOLogin {
                uuid, character_id, ..
            } => format!(
                "SFAccountCharLogin:{uuid}/{character_id}/unity3d_webglplayer/\
                 /{APP_VERSION}"
            ),
            Command::Register {
                username,
                password,
                gender,
                race,
                class,
            } => {
                
                format!(
                    "AccountCreate:{username}/{password}/{username}@playa.sso/\
                     {}/{}/{}/8,203,201,6,199,3,1,2,1/0//en",
                    *gender as usize + 1,
                    *race as usize,
                    *class as usize + 1
                )
            }
            Command::Update => "Poll:".to_string(),
            Command::HallOfFamePage { page } => {
                let per_page = 51;
                let pos = 26 + (per_page * page);
                format!("PlayerGetHallOfFame:{pos}//25/25")
            }
            Command::HallOfFameFortressPage { page } => {
                let per_page = 51;
                let pos = 26 + (per_page * page);
                format!("FortressGetHallOfFame:{pos}//25/25")
            }
            Command::HallOfFameGroupPage { page } => {
                let per_page = 51;
                let pos = 26 + (per_page * page);
                format!("GroupGetHallOfFame:{pos}//25/25")
            }
            Command::HallOfFameUnderworldPage { page } => {
                let per_page = 51;
                let pos = 26 + (per_page * page);
                format!("UnderworldGetHallOfFame:{pos}//25/25")
            }
            Command::HallOfFamePetsPage { page } => {
                let per_page = 51;
                let pos = 26 + (per_page * page);
                format!("PetsGetHallOfFame:{pos}//25/25")
            }
            Command::ViewPlayer { ident } => format!("PlayerLookAt:{ident}"),
            Command::BuyBeer => format!("PlayerBeerBuy:"),
            Command::StartQuest {
                quest_pos,
                overwrite_inv,
            } => {
                format!(
                    "PlayerAdventureStart:{}/{}",
                    quest_pos + 1,
                    u8::from(*overwrite_inv)
                )
            }
            Command::CancelQuest => format!("PlayerAdventureStop:"),
            Command::FinishQuest { skip } => {
                format!(
                    "PlayerAdventureFinished:{}",
                    skip.map_or(0, |a| a as u8)
                )
            }
            Command::StartWork { hours } => format!("PlayerWorkStart:{hours}"),
            Command::CancelWork => format!("PlayerWorkStop:"),
            Command::FinishWork => format!("PlayerWorkFinished:"),
            Command::CheckNameAvailable { name } => {
                format!("AccountCheck:{name}")
            }
            Command::BuyMount { mount } => {
                format!("PlayerMountBuy:{}", *mount as usize)
            }
            Command::IncreaseAttribute {
                attribute,
                increase_to,
            } => format!(
                "PlayerAttributIncrease:{}/{increase_to}",
                *attribute as u8
            ),
            Command::RemovePotion { pos } => {
                format!("PlayerPotionKill:{}", pos + 1)
            }
            Command::CheckArena => format!("PlayerArenaEnemy:"),
            Command::Fight { name, use_mushroom } => {
                format!("PlayerArenaFight:{name}/{}", u8::from(*use_mushroom))
            }
            Command::CollectCalendar => format!("PlayerOpenCalender:"),
            Command::UpgradeSkill {
                attribute,
                next_attribute,
            } => format!(
                "PlayerAttributIncrease:{}/{next_attribute}",
                *attribute as i64
            ),
            Command::RefreshShop { shop } => {
                format!("PlayerNewWares:{}", *shop as usize - 2)
            }
            Command::ViewGuild { guild_ident } => {
                format!("GroupLookAt:{guild_ident}")
            }
            Command::GuildFound { name } => format!("GroupFound:{name}"),
            Command::GuildInvitePlayer { name } => {
                format!("GroupInviteMember:{name}")
            }
            Command::GuildKickPlayer { name } => {
                format!("GroupRemoveMember:{name}")
            }
            Command::GuildSetLeader { name } => {
                format!("GroupSetLeader:{name}")
            }
            Command::GuildToggleOfficer { name } => {
                format!("GroupSetOfficer:{name}")
            }
            Command::GuildLoadMushrooms => {
                format!("GroupIncreaseBuilding:0")
            }
            Command::GuildIncreaseSkill { skill, current } => {
                format!("GroupSkillIncrease:{}/{current}", *skill as usize)
            }
            Command::GuildJoinAttack => format!("GroupReadyAttack:"),
            Command::GuildJoinDefense => format!("GroupReadyDefense:"),
            Command::GuildAttack { guild } => {
                format!("GroupAttackDeclare:{guild}")
            }
            Command::GuildRaid => format!("GroupRaidDeclare:"),
            Command::ToiletFlush => format!("PlayerToilettFlush:"),
            Command::ToiletOpen => format!("PlayerToilettOpenWithKey:"),
            Command::FightTower {
                current_level: progress,
                use_mush,
            } => {
                format!("PlayerTowerBattle:{progress}/{}", u8::from(*use_mush))
            }
            Command::ToiletDrop { item_pos } => {
                format!("PlayerToilettLoad:{item_pos}")
            }
            Command::GuildPortalBattle => format!("GroupPortalBattle:"),
            Command::GuildGetFightableTargets => {
                format!("GroupFightableTargets:")
            }
            Command::FightPortal => format!("PlayerPortalBattle:"),
            Command::MessageOpen { pos: index } => {
                format!("PlayerMessageView:{}", *index + 1)
            }
            Command::MessageDelete { pos: index } => format!(
                "PlayerMessageDelete:{}",
                match index {
                    -1 => -1,
                    x => *x + 1,
                }
            ),
            Command::ViewScrapbook => format!("PlayerPollScrapbook:"),
            Command::ViewPet { pet_id: pet_index } => {
                format!("PetsGetStats:{pet_index}")
            }
            Command::BuyShop {
                shop_pos,
                new_pos,
                item_ident,
            } => format!("PlayerItemMove:{shop_pos}/{new_pos}/{item_ident}"),
            Command::BuyShopHourglas {
                shop_type,
                shop_pos,
            } => format!(
                "PlayerItemMove:{}/{}/1/0",
                *shop_type as usize,
                *shop_pos + 1
            ),
            Command::SellShop {
                item_pos,
                item_ident,
            } => {
                let mut rng = fastrand::Rng::new();
                let shop = if rng.bool() {
                    ShopType::Magic
                } else {
                    ShopType::Weapon
                };
                let shop_pos = rng.u32(0..6);
                format!(
                    "PlayerItemMove:{item_pos}/{}/{}/{item_ident}",
                    shop as usize,
                    shop_pos + 1,
                )
            }
            Command::PlayerItemMove {
                from,
                to,
                item_ident,
            } => format!("PlayerItemMove:{from}/{to}/{item_ident}"),
            Command::ItemMove {
                from,
                to,
                item_ident,
            } => format!("PlayerItemMove:{from}/{to}/{item_ident}"),
            Command::UsePotion { from, item_ident } => {
                format!("PlayerItemMove:{from}/1/0/{item_ident}")
            }
            Command::UnlockFeature { unlockable } => format!(
                "UnlockFeature:{}/{}",
                unlockable.main_ident, unlockable.sub_ident
            ),
            Command::GuildSetInfo {
                description,
                emblem,
            } => format!(
                "GroupSetDescription:{}§{}",
                emblem.server_encode(),
                to_sf_string(description)
            ),
            Command::SetDescription { description } => {
                format!("PlayerSetDescription:{}", &to_sf_string(description))
            }
            Command::GuildSendChat { message } => {
                format!("GroupChat:{}", &to_sf_string(message))
            }
            Command::GambleSilver { amount } => {
                format!("PlayerGambleGold:{amount}")
            }
            Command::GambleMushrooms { amount } => {
                format!("PlayerGambleCoins:{amount}")
            }
            Command::SendMessage { to, msg } => {
                format!("PlayerMessageSend:{to}/{}", to_sf_string(msg))
            }
            Command::WitchDropCauldron { item_pos } => {
                format!("PlayerWitchSpendItem:{item_pos}")
            }
            Command::Blacksmith {
                item_pos,
                action,
                item_ident,
            } => format!(
                "PlayerItemMove:{item_pos}/{}/-1/{item_ident}",
                *action as usize
            ),
            Command::WitchEnchant { enchantment } => {
                format!("PlayerWitchEnchantItem:{}/1", enchantment.0)
            }
            Command::WitchEnchantCompanion {
                enchantment,
                companion,
            } => {
                format!(
                    "PlayerWitchEnchantItem:{}/{}",
                    enchantment.0,
                    *companion as u8 + 101,
                )
            }
            Command::UpdateLureSuggestion => {
                format!("PlayerGetHallOfFame:-4//0/0")
            }
            Command::SpinWheelOfFortune {
                payment: fortune_payment,
            } => {
                format!("WheelOfFortune:{}", *fortune_payment as usize)
            }
            Command::FortressGather { resource } => {
                format!("FortressGather:{}", *resource as usize + 1)
            }
            Command::FortressGatherSecretStorage { stone, wood } => {
                format!("FortressGatherTreasure:{wood}/{stone}")
            }
            Command::Equip {
                from_pos,
                to_slot,
                item_ident,
            } => format!(
                "PlayerItemMove:{from_pos}/1/{}/{item_ident}",
                *to_slot as usize
            ),
            Command::EquipCompanion {
                from_pos,
                to_companion,
                item_ident,
                to_slot,
            } => format!(
                "PlayerItemMove:{from_pos}/{}/{}/{item_ident}",
                *to_companion as u8 + 101,
                *to_slot as usize
            ),
            Command::FortressBuild { f_type } => {
                format!("FortressBuildStart:{}/0", *f_type as usize + 1)
            }
            Command::FortressBuildCancel { f_type } => {
                format!("FortressBuildStop:{}", *f_type as usize + 1)
            }
            Command::FortressBuildFinish { f_type, mushrooms } => format!(
                "FortressBuildFinished:{}/{mushrooms}",
                *f_type as usize + 1
            ),
            Command::FortressBuildUnit { unit, count } => {
                format!("FortressBuildUnitStart:{}/{count}", *unit as usize + 1)
            }
            Command::FortressGemStoneSearch => {
                format!("FortressGemstoneStart:")
            }
            Command::FortressGemStoneSearchCancel => {
                format!("FortressGemStoneStop:")
            }
            Command::FortressGemStoneSearchFinish { mushrooms } => {
                format!("FortressGemstoneFinished:{mushrooms}")
            }
            Command::FortressAttack { soldiers } => {
                format!("FortressAttack:{soldiers}")
            }
            Command::FortressNewEnemy { use_mushroom: pay } => {
                format!("FortressEnemy:{}", usize::from(*pay))
            }
            Command::FortressSetCAEnemy { msg_id } => {
                format!("FortressEnemy:0/{}", *msg_id)
            }
            Command::FortressUpgradeHallOfKnights => {
                format!("FortressGroupBonusUpgrade:")
            }
            Command::FortressUpgradeUnit { unit } => {
                format!("FortressUpgrade:{}", *unit as u8 + 1)
            }
            Command::Whisper {
                player_name: player,
                message,
            } => format!(
                "PlayerMessageWhisper:{}/{}",
                player,
                to_sf_string(message)
            ),
            Command::UnderworldCollect { resource } => {
                format!("UnderworldGather:{}", *resource as usize + 1)
            }
            Command::UnderworldUnitUpgrade { unit: unit_t } => {
                format!("UnderworldUpgradeUnit:{}", *unit_t as usize + 1)
            }
            Command::UnderworldUpgradeStart {
                building,
                mushrooms,
            } => format!(
                "UnderworldBuildStart:{}/{mushrooms}",
                *building as usize + 1
            ),
            Command::UnderworldUpgradeCancel { building } => {
                format!("UnderworldBuildStop:{}", *building as usize + 1)
            }
            Command::UnderworldUpgradeFinish {
                building,
                mushrooms,
            } => {
                format!(
                    "UnderworldBuildFinished:{}/{mushrooms}",
                    *building as usize + 1
                )
            }
            Command::UnderworldAttack { player_id } => {
                format!("UnderworldAttack:{player_id}")
            }
            Command::RollDice { payment, dices } => {
                let mut dices = dices.iter().fold(String::new(), |mut a, b| {
                    if !a.is_empty() {
                        a.push('/');
                    }
                    a.push((*b as u8 + b'0') as char);
                    a
                });

                if dices.is_empty() {
                    
                    dices = "0/0/0/0/0".to_string();
                }
                format!("RollDice:{}/{}", *payment as usize, dices)
            }
            Command::PetFeed { pet_id, fruit_idx } => {
                format!("PlayerPetFeed:{pet_id}/{fruit_idx}")
            }
            Command::GuildPetBattle { use_mushroom } => {
                format!("GroupPetBattle:{}", usize::from(*use_mushroom))
            }
            Command::IdleUpgrade { typ: kind, amount } => {
                format!("IdleIncrease:{}/{}", *kind as usize, *amount as i32)
            }
            Command::IdleSacrifice => format!("IdlePrestige:0"),
            Command::SwapMannequin => format!("PlayerDummySwap:301/1"),
            Command::UpdateFlag { flag } => format!(
                "PlayerSetFlag:{}",
                flag.map(Flag::code).unwrap_or_default()
            ),
            Command::BlockGuildInvites { block_invites } => {
                format!("PlayerSetNoGroupInvite:{}", u8::from(*block_invites))
            }
            Command::ShowTips { show_tips } => {
                #[allow(clippy::unreadable_literal)]
                {
                    format!(
                        "PlayerTutorialStatus:{}",
                        if *show_tips { 0 } else { 0xFFFFFFF }
                    )
                }
            }
            Command::ChangePassword { username, old, new } => {
                let old = sha1_hash(&format!("{old}{HASH_CONST}"));
                let new = sha1_hash(&format!("{new}{HASH_CONST}"));
                format!("AccountPasswordChange:{username}/{old}/106/{new}/")
            }
            Command::ChangeMailAddress {
                old_mail,
                new_mail,
                password,
                username,
            } => {
                let pass = sha1_hash(&format!("{password}{HASH_CONST}"));
                format!(
                    "AccountMailChange:{old_mail}/{new_mail}/{username}/\
                     {pass}/106"
                )
            }
            Command::SetLanguage { language } => {
                format!("AccountSetLanguage:{language}")
            }
            Command::SetPlayerRelation {
                player_id,
                relation,
            } => {
                format!("PlayerFriendSet:{player_id}/{}", *relation as i32)
            }
            Command::SetPortraitFrame { portrait_id } => {
                format!("PlayerSetActiveFrame:{portrait_id}")
            }
            Command::CollectDailyQuestReward { pos } => {
                format!("DailyTaskClaim:1/{}", pos + 1)
            }
            Command::CollectEventTaskReward { pos } => {
                format!("DailyTaskClaim:2/{}", pos + 1)
            }
            Command::SwapRunes {
                from,
                from_pos,
                to,
                to_pos,
            } => {
                format!(
                    "PlayerSmithSwapRunes:{}/{}/{}/{}",
                    *from as usize,
                    *from_pos + 1,
                    *to as usize,
                    *to_pos + 1
                )
            }
            Command::ChangeItemLook {
                inv,
                pos,
                raw_model_id: model_id,
            } => {
                format!(
                    "ItemChangePicture:{}/{}/{}",
                    *inv as usize,
                    pos + 1,
                    model_id
                )
            }
            Command::ExpeditionPickEncounter { pos } => {
                format!("ExpeditionProceed:{}", pos + 1)
            }
            Command::ExpeditionContinue => format!("ExpeditionProceed:1"),
            Command::ExpeditionPickReward { pos } => {
                format!("ExpeditionProceed:{}", pos + 1)
            }
            Command::ExpeditionStart { pos } => {
                format!("ExpeditionStart:{}", pos + 1)
            }
            Command::LegendaryDungeonEnter { theme } => {
                format!("IADungeonStart:{}/0", *theme as usize)
            }
            Command::LegendaryDungeonBuyBlessing { effect, keys } => {
                format!("IADungeonMerchantBuy:{}/{}", *effect as i32, *keys)
            }
            Command::LegendaryDungeonBuyCurse { effect, keys } => {
                format!(
                    "IADungeonDebuffMerchantBuy:{}/{}",
                    *effect as i32, *keys
                )
            }
            Command::LegendaryDungeonMonsterCollectKey => {
                "IADungeonInteract:60".into()
            }
            Command::LegendaryDungeonMerchantNewGoods => {
                "IADungeonInteract:50".into()
            }
            Command::LegendaryDungeonInteract { val } => {
                
                
                
                
                
                
                
                format!("IADungeonInteract:{val}")
            }
            Command::LegendaryDungeonMonsterFight => {
                format!("IADungeonInteract:20")
            }
            Command::LegendaryDungeonMonsterEscape => {
                format!("IADungeonInteract:21")
            }
            Command::LegendaryDungeonEncounterInteract => {
                format!("IADungeonInteract:40")
            }
            Command::LegendaryDungeonEncounterEscape => {
                format!("IADungeonInteract:41")
            }
            Command::LegendaryDungeonEncounterLeave => {
                format!("IADungeonInteract:42")
            }
            Command::LegendaryDungeonRoomInteract => {
                format!("IADungeonInteract:50")
            }
            Command::LegendaryDungeonRoomLeave => {
                format!("IADungeonInteract:51")
            }
            Command::LegendaryDungeonForcedContinue => {
                format!("IADungeonInteract:70")
            }
            Command::LegendaryDungeonPickDoor { pos, typ } => {
                let mut id = pos + 1;
                if matches!(
                    typ,
                    DoorType::LockedDoor
                        | DoorType::DoubleLockedDoor
                        | DoorType::EpicDoor
                ) {
                    id += 4;
                }
                format!("IADungeonInteract:{id}")
            }
            Command::LegendaryDungeonPlayRPC { choice } => {
                format!("IADungeonInteract:{}", *choice as i32)
            }
            Command::LegendaryDungeonPickGem { gem_type } => {
                format!("IADungeonSelectSoulStone:{}", *gem_type as u32)
            }
            Command::LegendaryDungeonTakeItem {
                item_idx,
                inventory_to,
                item_ident,
            } => {
                format!(
                    "PlayerItemMove:401/{}/{inventory_to}/{item_ident}",
                    item_idx + 1
                )
            }
            Command::FightDungeon {
                dungeon,
                use_mushroom,
            } => match dungeon {
                Dungeon::Light(name) => {
                    if *name == LightDungeon::Tower {
                        return Err(SFError::InvalidRequest(
                            "The tower must be fought with the FightTower \
                             command",
                        ));
                    }
                    format!(
                        "PlayerDungeonBattle:{}/{}",
                        *name as usize + 1,
                        u8::from(*use_mushroom)
                    )
                }
                Dungeon::Shadow(name) => {
                    if *name == ShadowDungeon::Twister {
                        format!(
                            "PlayerDungeonBattle:{}/{}",
                            LightDungeon::Tower as u32 + 1,
                            u8::from(*use_mushroom)
                        )
                    } else {
                        format!(
                            "PlayerShadowBattle:{}/{}",
                            *name as u32 + 1,
                            u8::from(*use_mushroom)
                        )
                    }
                }
            },
            Command::FightPetOpponent {
                opponent_id,
                habitat: element,
            } => {
                format!("PetsPvPFight:0/{opponent_id}/{}", *element as u32 + 1)
            }
            Command::BrewPotion { fruit_type } => {
                format!("PlayerWitchBrewPotion:{}", *fruit_type as u8)
            }
            Command::FightPetDungeon {
                use_mush,
                habitat: element,
                enemy_pos,
                player_pet_id,
            } => {
                format!(
                    "PetsDungeonFight:{}/{}/{enemy_pos}/{player_pet_id}",
                    u8::from(*use_mush),
                    *element as u8 + 1,
                )
            }
            Command::ExpeditionSkipWait { typ } => {
                format!("ExpeditionTimeSkip:{}", *typ as u8)
            }
            Command::SetQuestsInsteadOfExpeditions { value } => {
                let value = match value {
                    ExpeditionSetting::PreferExpeditions => 'a',
                    ExpeditionSetting::PreferQuests => 'b',
                };
                format!("UserSettingsUpdate:5/{value}")
            }
            Command::HellevatorEnter => format!("GroupTournamentJoin:"),
            Command::HellevatorViewGuildRanking => {
                format!("GroupTournamentRankingOwnGroup")
            }
            Command::HellevatorFight { use_mushroom } => {
                format!("GroupTournamentBattle:{}", u8::from(*use_mushroom))
            }
            Command::HellevatorBuy {
                position,
                typ,
                price,
                use_mushroom,
            } => {
                format!(
                    "GroupTournamentMerchantBuy:{position}/{}/{price}/{}",
                    *typ as u32,
                    if *use_mushroom { 2 } else { 1 }
                )
            }
            Command::HellevatorRefreshShop => {
                format!("GroupTournamentMerchantReroll:")
            }
            Command::HallOfFameHellevatorPage { page } => {
                let per_page = 51;
                let pos = 26 + (per_page * page);
                format!("GroupTournamentRankingAllGroups:{pos}//25/25")
            }
            Command::HellevatorJoinHellAttack {
                use_mushroom,
                plain: pos,
            } => {
                format!(
                    "GroupTournamentRaidParticipant:{}/{}",
                    u8::from(*use_mushroom),
                    *pos + 1
                )
            }
            Command::HellevatorClaimDaily => {
                format!("GroupTournamentClaimDaily:")
            }
            Command::HellevatorClaimDailyYesterday => {
                format!("GroupTournamentClaimDailyYesterday:")
            }
            Command::HellevatorPreviewRewards => {
                format!("GroupTournamentPreview:")
            }
            Command::HellevatorClaimFinal => format!("GroupTournamentClaim:"),
            Command::ClaimablePreview { msg_id } => {
                format!("PendingRewardView:{msg_id}")
            }
            Command::ClaimableClaim { msg_id } => {
                format!("PendingRewardClaim:{msg_id}")
            }
            Command::BuyGoldFrame => {
                format!("PlayerGoldFrameBuy:")
            }
            Command::UpdateDungeons => format!("PlayerDungeonOpen:"),
            Command::CollectAdventsCalendar => {
                format!("AdventsCalendarClaimReward:")
            }
            Command::ViewLureSuggestion { suggestion } => {
                format!("PlayerGetHallOfFame:{}//0/0", suggestion.0)
            }
            Command::FortressChangeEnemy { msg_id } => {
                format!("FortressEnemy:0/{msg_id}")
            }
        })
    }
}

macro_rules! generate_flag_enum {
    ($($variant:ident => $code:expr),*) => {
        
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[allow(missing_docs)]
        pub enum Flag {
            $(
                $variant,
            )*
        }

        impl Flag {
            #[allow(unused)]
            pub(crate) fn code(self) -> &'static str {
                match self {
                    $(
                        Flag::$variant => $code,
                    )*
                }
            }

            pub(crate) fn parse(value: &str) -> Option<Self> {
                if value.is_empty() {
                    return None;
                }

                
                match value {
                    $(
                        $code => Some(Flag::$variant),
                    )*

                    _ => {
                        warn!("Invalid flag value: {value}");
                        None
                    }
                }
            }
        }
    };
}



generate_flag_enum! {
    Argentina => "ar",
    Australia => "au",
    Austria => "at",
    Belgium => "be",
    Bolivia => "bo",
    Brazil => "br",
    Bulgaria => "bg",
    Canada => "ca",
    Chile => "cl",
    China => "cn",
    Colombia => "co",
    CostaRica => "cr",
    Czechia => "cz",
    Denmark => "dk",
    DominicanRepublic => "do",
    Ecuador => "ec",
    ElSalvador =>"sv",
    Finland => "fi",
    France => "fr",
    Germany => "de",
    GreatBritain => "gb",
    Greece => "gr",
    Honduras => "hn",
    Hungary => "hu",
    India => "in",
    Italy => "it",
    Japan => "jp",
    Lithuania => "lt",
    Mexico => "mx",
    Netherlands => "nl",
    Panama => "pa",
    Paraguay => "py",
    Peru => "pe",
    Philippines => "ph",
    Poland => "pl",
    Portugal => "pt",
    Romania => "ro",
    Russia => "ru",
    SaudiArabia => "sa",
    Slovakia => "sk",
    SouthKorea => "kr",
    Spain => "es",
    Sweden => "se",
    Switzerland => "ch",
    Thailand => "th",
    Turkey => "tr",
    Ukraine => "ua",
    UnitedArabEmirates => "ae",
    UnitedStates => "us",
    Uruguay => "uy",
    Venezuela => "ve",
    Vietnam => "vn"
}
