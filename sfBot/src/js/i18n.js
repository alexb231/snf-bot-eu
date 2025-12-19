// Translations
const translations = {
    en: {
        // Loading
        "loading.connecting": "Connecting to server...",

        // Navigation
        "nav.dashboard": "Dashboard",
        "nav.accounts": "Accounts",
        "nav.logs": "Logs",
        "nav.botSettings": "Bot Settings",

        // Header
        "header.stopped": "Stopped",
        "header.running": "Running",
        "header.paused": "Paused",
        "header.start": "Start",
        "header.pause": "Pause",
        "header.resume": "Resume",
        "header.stop": "Stop",
        "header.shutdown": "Shutdown server",

        // Dashboard Stats
        "stats.accounts": "Accounts",
        "stats.characters": "Characters",
        "stats.active": "Active",
        "stats.uptime": "Uptime",

        // Dashboard
        "dashboard.currentAction": "Current Action",
        "dashboard.botNotStarted": "Bot not started",
        "dashboard.botPaused": "Bot paused",
        "dashboard.waitingForAction": "Waiting for action...",
        "dashboard.dataRefreshed": "Data refreshed",
        "dashboard.characters": "Characters",
        "dashboard.refresh": "Refresh",

        // Table Headers
        "table.active": "Active",
        "table.name": "Name",
        "table.level": "Level",
        "table.server": "Server",
        "table.guild": "Guild",
        "table.gold": "Gold",
        "table.mushrooms": "Mushrooms",
        "table.luckycoins": "Lucky Coins",
        "table.hourglasses": "Hourglasses",
        "table.mount": "Mount",
        "table.beer": "Beer",
        "table.arena": "Arena",
        "table.petfights": "Pet Fights",
        "table.dicerolls": "Dice Rolls",
        "table.thirst": "Thirst",
        "table.currentAction": "Current",
        "table.action": "Action",
        "table.copySettings": "Copy settings",
        "table.stats": "Stats",
        "table.log": "Log",
        "table.settings": "Settings",
        "table.empty": "No characters loaded. Please add an account and start the bot.",
        "charSettings.copyFrom": "Copy settings from:",
        "charSettings.copyPlaceholder": "Select character",

        // Log messages
        "log.settingsSaved": "Settings saved",
        "log.charSettingsSaved": "Settings for {name} saved",
        "log.charsLoadedFromCache": "{count} character(s) loaded from cache",
        "log.saveError": "Error saving",

        // Accounts View
        "accounts.add": "Add Account",
        "accounts.username": "Username",
        "accounts.password": "Password",
        "accounts.singleServer": "Single Server Account (Old)",
        "accounts.serverUrl": "Server URL",
        "accounts.addBtn": "Add Account",
        "accounts.saved": "Saved Accounts",
        "accounts.empty": "No accounts saved",

        // Logs View
        "logs.title": "Bot Logs",
        "logs.clear": "Clear Logs",
        "logs.ready": "Bot ready...",

        // Global Settings Modal
        "globalSettings.title": "Bot Settings",
        "globalSettings.autoStart": "Automatically start all accounts on launch",
        "globalSettings.sleepTime": "Wait time between requests (ms)",
        "globalSettings.min": "Min",
        "globalSettings.max": "Max",
        "globalSettings.doNotRelogSeconds": "Do not relog character after invalid session (seconds)",
        "globalSettings.doNotRelogSecondsShort": "Seconds",
        "globalSettings.save": "Save",
        "globalSettings.cancel": "Cancel",

        // Character Settings Modal
        "charSettings.title": "Character Settings",

        // Settings Sections
        "section.general": "General",
        "section.tavern": "Tavern",
        "section.items": "Items",
        "section.arena": "Arena",
        "section.quarters": "Quarters",
        "section.fortress": "Fortress",
        "section.underworld": "Underworld",
        "section.character": "Character",
        "section.pets": "Pets",
        "section.misc": "Misc",
        "section.dungeons": "Dungeons",
        "section.toilet": "Toilet",

        // General Settings
        "general.title": "General Settings",
        "general.activate": "Activate character",

        // Tavern Settings
        "tavern.title": "Tavern",
        "tavern.expeditions": "Expeditions",
        "tavern.playExpeditions": "Play expeditions",
        "tavern.prioritizeExp": "Prioritize EXP",
        "tavern.prioritizeGold": "Prioritize Gold",
        "tavern.skipWithHourglasses": "Skip expeditions using hourglasses",
        "tavern.expeditionsFrom": "Expeditions from",
        "tavern.rewardPriority": "Expedition Reward Priority",
        "tavern.beer": "Beer",
        "tavern.beerAmount": "Amount of beers to drink",
        "tavern.cityGuard": "City Guard",
        "tavern.playCityGuard": "Play city guard",
        "tavern.from": "From",
        "tavern.to": "To",
        "tavern.guardDuration": "City guard duration (hours)",
        "tavern.diceGame": "Dice Game",
        "tavern.playDiceGame": "Play dice game",
        "tavern.diceSkipHG": "Skip wait time using hourglasses",
        "tavern.dicePriority": "Dice Game Priorities",

        // Items Settings
        "items.title": "Items",
        "items.inventoryManagement": "Inventory Management",
        "items.enableInventory": "Enable inventory management",
        "items.minGold": "Minimum gold to keep",
        "items.minMushrooms": "Minimum mushrooms to keep",
        "items.freeSlots": "Free inventory slots",
        "items.ignoreGemMine": "Ignore min gold during gem search",
        "items.sellOptions": "Sell Options",
        "items.sellCheapest": "Sell cheapest item",
        "items.sellExpensive": "Sell most expensive item",
        "items.dontSellEpics": "Don't sell epics",
        "items.throwCauldron": "Immediately throw into cauldron",
        "items.excludeEpics": "Exclude epics from cauldron",
        "items.enchant": "Enchant Items",
        "items.weapon": "Weapon",
        "items.hat": "Helmet",
        "items.chest": "Chestplate",
        "items.gloves": "Gloves",
        "items.boots": "Boots",
        "items.necklace": "Necklace",
        "items.belt": "Belt",
        "items.ring": "Ring",
        "items.talisman": "Talisman",
        "items.potions": "Potions",
        "items.drink": "Drink",
        "items.sell": "Sell",
        "items.keep": "Keep",
        "items.buy": "Buy",
        "items.winged": "Winged",
        "items.strSmall": "Str Small",
        "items.strMedium": "Str Medium",
        "items.strLarge": "Str Large",
        "items.dexSmall": "Dex Small",
        "items.dexMedium": "Dex Medium",
        "items.dexLarge": "Dex Large",
        "items.intSmall": "Int Small",
        "items.intMedium": "Int Medium",
        "items.intLarge": "Int Large",
        "items.constSmall": "Const Small",
        "items.constMedium": "Const Medium",
        "items.constLarge": "Const Large",
        "items.luckSmall": "Luck Small",
        "items.luckMedium": "Luck Medium",
        "items.luckLarge": "Luck Large",
        "items.buyHourglasses": "Buy hourglasses in shop",
        "items.brewPotions": "Brew potions using fruits (Level 632+)",
        "items.gems": "Gems",
        "items.strength": "Strength",
        "items.dexterity": "Dexterity",
        "items.intelligence": "Intelligence",
        "items.constitution": "Constitution",
        "items.luck": "Luck",
        "items.black": "Black",
        "items.legendary": "Legendary",
        "items.keepGemPercent": "Keep gems if >= % of current",
        "items.equipmentSwap": "Equipment Swap",
        "items.enableSwap": "Enable equipment swap",

        // Arena Settings
        "arena.title": "Arena",
        "arena.enable": "Enable arena",
        "arena.stopAfter10": "Stop after 10 wins",
        "arena.fillScrapbook": "Fill scrapbook after EXP fights",

        // Quarters Settings
        "quarters.title": "Quarters",
        "quarters.guildAttacks": "Guild Attacks",
        "quarters.orderAttack": "Attack preferred guilds",
        "quarters.preferredEnemies": "Preferred enemies (separated by /)",
        "quarters.firstAttackTime": "First attack time",
        "quarters.secondAttackTime": "Second attack time",
        "quarters.guildParticipation": "Guild Participation",
        "quarters.signUpAttacks": "Sign up for guild attacks",
        "quarters.signUpDefense": "Sign up for guild defense",
        "quarters.signUpHydra": "Sign up for guild hydra",
        "quarters.fightPortal": "Fight guild portal",
        "quarters.other": "Other",
        "quarters.collectMail": "Collect mail rewards",
        "quarters.spinWheel": "Spin lucky wheel",
        "quarters.luckyCoins": "Lucky coins",
        "quarters.mushrooms": "Mushrooms",
        "quarters.spinAmount": "Number of spins",
        "quarters.hellevator": "Hellevator",
        "quarters.playHellevator": "Play hellevator",
        "quarters.keepKeycards": "Keep keycards",
        "quarters.claimDaily": "Claim daily rewards",
        "quarters.claimFinal": "Claim final reward",
        "quarters.joinRaid": "Join Hell Attack",
        "quarters.raidFloor": "Hell Attack floor",

        // Fortress Settings
        "fortress.title": "Fortress",
        "fortress.collectResources": "Collect Resources",
        "fortress.collectWood": "Collect wood",
        "fortress.collectStone": "Collect stone",
        "fortress.collectExp": "Collect EXP",
        "fortress.collectFrom": "Collect from",
        "fortress.collectTo": "to",
        "fortress.searchGems": "Search for gems",
        "fortress.attacks": "Attacks",
        "fortress.doAttacks": "Perform fortress attacks",
        "fortress.oneSoldier": "Use 1 soldier",
        "fortress.proposedSoldiers": "Proposed soldiers",
        "fortress.additionalPercent": "Additional % soldiers",
        "fortress.training": "Training",
        "fortress.trainSoldiers": "Train soldiers",
        "fortress.trainMages": "Train mages",
        "fortress.trainArchers": "Train archers",
        "fortress.buildings": "Buildings",
        "fortress.upgradeBuildings": "Upgrade fortress buildings",

        // Underworld Settings
        "underworld.title": "Underworld",
        "underworld.resources": "Resources",
        "underworld.collectSouls": "Collect souls",
        "underworld.collectGold": "Collect gold",
        "underworld.collectThirst": "Collect thirst",
        "underworld.dontCollectFrom": "Don't collect gold from",
        "underworld.dontCollectTo": "to",
        "underworld.upgrades": "Upgrades",
        "underworld.upgradeBuildings": "Upgrade underworld buildings",
        "underworld.upgradeKeeper": "Upgrade keeper",
        "underworld.keepSouls": "Keep souls",
        "underworld.attacks": "Attacks",
        "underworld.performAttacks": "Perform proposed attacks",
        "underworld.attackFavourite": "Attack preferred opponent",
        "underworld.favouriteOpponent": "Preferred opponent",

        // Character Settings
        "character.title": "Character",
        "character.attributes": "Attributes",
        "character.increaseStats": "Increase attributes",
        "character.statDistribution": "Stat Distribution (%)",
        "character.str": "Strength",
        "character.dex": "Dexterity",
        "character.int": "Intelligence",
        "character.const": "Constitution",
        "character.luck": "Luck",
        "character.mount": "Mount",
        "character.enableMount": "Enable mount buying",
        "character.bestMount": "Best affordable",
        "character.griffon": "Griffon/Dragon",
        "character.tiger": "Tiger",
        "character.horse": "Horse",
        "character.cow": "Cow",

        // Pets Settings
        "pets.title": "Pets",
        "pets.fights": "Fights",
        "pets.doFights": "Pet arena fights",
        "pets.doDungeons": "Pet dungeon fights",
        "pets.feeding": "Feeding",
        "pets.doFeed": "Feed pets",
        "pets.cheapest": "Cheapest way",
        "pets.expensive": "Shroomer way",
        "pets.feedPerDay": "Pets per habitat to feed",

        // Misc Settings
        "misc.title": "Misc",
        "misc.calendar": "Calendar",
        "misc.dontCollectBefore": "Don't collect calendar before",
        "misc.collectAll": "Collect all calendars",
        "misc.collectExpOnly": "Collect EXP calendar only",
        "misc.collectMushrooms": "Consider mushroom calendar",
        "misc.rewards": "Rewards",
        "misc.collectDaily": "Collect daily rewards",
        "misc.collectWeekly": "Collect weekly rewards",
        "misc.dailyTasks": "Daily Tasks",
        "misc.gambling": "Gambling",
        "misc.bareHand": "Bare hand attack",
        "misc.defeatWarrior": "Defeat 3x Warrior",
        "misc.defeatScout": "Defeat 3x Scout",
        "misc.defeatMage": "Defeat 3x Mage",
        "misc.defeatAssassin": "Defeat 3x Assassin",
        "misc.defeatBattleMage": "Defeat 3x Battle Mage",
        "misc.defeatBerserker": "Defeat 3x Berserker",
        "misc.defeatDruid": "Defeat 3x Druid",
        "misc.defeatDemonHunter": "Defeat 3x Demon Hunter",
        "misc.defeatBard": "Defeat 3x Bard",
        "misc.defeatNecromancer": "Defeat 3x Necromancer",
        "misc.defeatPaladin": "Defeat 3x Paladin",
        "misc.breakTimes": "Break Times",
        "misc.noActionsFrom": "No actions from",
        "misc.noActionsTo": "to",

        // Dungeons Settings
        "dungeons.title": "Dungeons",
        "dungeons.enable": "Enable dungeons",
        "dungeons.fightPortal": "Fight demon portal",
        "dungeons.bestWinrate": "Dungeon with best win rate",
        "dungeons.skip": "Skip Dungeons",
        "dungeons.skipIdols": "Skip Loop of Idols",
        "dungeons.skipTwister": "Skip Twister",
        "dungeons.skipTower": "Skip Tower",
        "dungeons.skipSandstorm": "Skip Sandstorm",
        "dungeons.arenaManager": "Arena Manager",
        "dungeons.enableManager": "Enable arena manager",
        "dungeons.sacrificePercent": "Sacrifice after % of current runes",
        "dungeons.sacrificeAfterToiletCycle": "Sacrifice after each toilet cycle",

        // Toilet Settings
        "toilet.title": "Toilet",
        "toilet.enable": "Enable arcane toilet",
        "toilet.flushWhenFull": "Flush toilet when full",
        "toilet.priorityHint": "Priority: 1. Epic, 2. Gem, 3. Normal item",
        "toilet.sacrificeEpics": "Sacrifice epics",
        "toilet.excludeWeapons": "Exclude epic weapons",
        "toilet.sacrificeGems": "Sacrifice gems",
        "toilet.sacrificeNormal": "Sacrifice normal items",

        // Log Modal
        "logModal.previous": "Previous",
        "logModal.close": "Close",
        "logModal.next": "Next",

        // Expedition Stats
        "expeditionStats.title": "Expedition Stats",
        "expeditionStats.close": "Close",
        "expeditionStats.loading": "Loading...",
        "expeditionStats.noData": "No expedition stats found",
        "expeditionStats.modeAll": "All",
        "expeditionStats.modeExp": "EXP",
        "expeditionStats.modeGold": "Gold",
        "expeditionStats.character": "Character",
        "expeditionStats.server": "Server",
        "expeditionStats.runs": "Expeditions",
        "expeditionStats.heroismAvg": "Heroism avg",
        "expeditionStats.heroismMax": "Heroism max",
        "expeditionStats.heroismLast": "Heroism last",
        "expeditionStats.keysAvg": "Keys avg",
        "expeditionStats.chestsAvg": "Chests avg",
        "expeditionStats.encounters": "Encounters",
        // Expedition Summary
        "expeditionSummary.button": "Expedition Overview",
        "expeditionSummary.title": "Expedition Overview",
        "expeditionSummary.close": "Close",
        "expeditionSummary.loading": "Loading...",
        "expeditionSummary.noData": "No expedition stats found",
        "expeditionSummary.modeAll": "All",
        "expeditionSummary.modeExp": "EXP",
        "expeditionSummary.modeGold": "Gold",
        "expeditionSummary.expedition": "Expedition",
        "expeditionSummary.runs": "Expeditions",
        "expeditionSummary.heroismAvg": "Heroism avg",
        "expeditionSummary.keysAvg": "Keys avg",
        "expeditionSummary.chestsAvg": "Chests avg"
    },
    de: {
        // Loading
        "loading.connecting": "Verbinde mit Server...",

        // Navigation
        "nav.dashboard": "Dashboard",
        "nav.accounts": "Accounts",
        "nav.logs": "Logs",
        "nav.botSettings": "Bot Einstellungen",

        // Header
        "header.stopped": "Gestoppt",
        "header.running": "Laeuft",
        "header.paused": "Pausiert",
        "header.start": "Start",
        "header.pause": "Pause",
        "header.resume": "Fortsetzen",
        "header.stop": "Stop",
        "header.shutdown": "Server beenden",

        // Dashboard Stats
        "stats.accounts": "Accounts",
        "stats.characters": "Charaktere",
        "stats.active": "Aktiv",
        "stats.uptime": "Laufzeit",

        // Dashboard
        "dashboard.currentAction": "Aktuelle Aktion",
        "dashboard.botNotStarted": "Bot nicht gestartet",
        "dashboard.botPaused": "Bot pausiert",
        "dashboard.waitingForAction": "Warte auf Aktion...",
        "dashboard.dataRefreshed": "Daten aktualisiert",
        "dashboard.characters": "Charaktere",
        "dashboard.refresh": "Aktualisieren",

        // Table Headers
        "table.active": "Aktiv",
        "table.name": "Name",
        "table.level": "Level",
        "table.server": "Server",
        "table.guild": "Gilde",
        "table.gold": "Gold",
        "table.mushrooms": "Pilze",
        "table.luckycoins": "Glücksmarken",
        "table.hourglasses": "Stundengläser",
        "table.mount": "Reittier",
        "table.beer": "Bier",
        "table.arena": "Arena",
        "table.petfights": "Pet-Kaempfe",
        "table.dicerolls": "Wuerfe",
        "table.thirst": "Durst",
        "table.currentAction": "Aktuell",
        "table.action": "Aktion",
        "table.copySettings": "Einstellungen kopieren",
        "table.stats": "Stats",
        "table.log": "Log",
        "table.settings": "Einstellungen",
        "table.empty": "Keine Charaktere geladen. Bitte Account hinzufuegen und Bot starten.",
        "charSettings.copyFrom": "Einstellungen kopieren von:",
        "charSettings.copyPlaceholder": "Charakter auswählen",

        // Log messages
        "log.settingsSaved": "Einstellungen gespeichert",
        "log.charSettingsSaved": "Einstellungen fuer {name} gespeichert",
        "log.charsLoadedFromCache": "{count} Charakter(e) aus Cache geladen",
        "log.saveError": "Fehler beim Speichern",

        // Accounts View
        "accounts.add": "Account hinzufuegen",
        "accounts.username": "Benutzername",
        "accounts.password": "Passwort",
        "accounts.singleServer": "Single Server Account (Alt)",
        "accounts.serverUrl": "Server URL",
        "accounts.addBtn": "Account hinzufuegen",
        "accounts.saved": "Gespeicherte Accounts",
        "accounts.empty": "Keine Accounts gespeichert",

        // Logs View
        "logs.title": "Bot Logs",
        "logs.clear": "Logs loeschen",
        "logs.ready": "Bot bereit...",

        // Global Settings Modal
        "globalSettings.title": "Bot Einstellungen",
        "globalSettings.autoStart": "Automatisch alle Accounts beim Start starten",
        "globalSettings.sleepTime": "Wartezeit zwischen Anfragen (ms)",
        "globalSettings.min": "Min",
        "globalSettings.max": "Max",
        "globalSettings.doNotRelogSeconds": "Charakter nach ungültiger Session nicht neu einloggen (Sekunden)",
        "globalSettings.doNotRelogSecondsShort": "Sekunden",
        "globalSettings.save": "Speichern",
        "globalSettings.cancel": "Abbrechen",

        // Character Settings Modal
        "charSettings.title": "Charakter Einstellungen",

        // Settings Sections
        "section.general": "Allgemein",
        "section.tavern": "Taverne",
        "section.items": "Items",
        "section.arena": "Arena",
        "section.quarters": "Quartier",
        "section.fortress": "Festung",
        "section.underworld": "Unterwelt",
        "section.character": "Charakter",
        "section.pets": "Haustiere",
        "section.misc": "Sonstiges",
        "section.dungeons": "Dungeons",
        "section.toilet": "Toilette",

        // General Settings
        "general.title": "Allgemeine Einstellungen",
        "general.activate": "Charakter aktivieren",

        // Tavern Settings
        "tavern.title": "Taverne",
        "tavern.expeditions": "Expeditionen",
        "tavern.playExpeditions": "Expeditionen spielen",
        "tavern.prioritizeExp": "EXP priorisieren",
        "tavern.prioritizeGold": "Gold priorisieren",
        "tavern.skipWithHourglasses": "Expeditionen mit Sanduhren ueberspringen",
        "tavern.expeditionsFrom": "Expeditionen ab",
        "tavern.rewardPriority": "Expeditions-Belohnungs-Prioritaet",
        "tavern.beer": "Bier",
        "tavern.beerAmount": "Anzahl Biere trinken",
        "tavern.cityGuard": "Stadtwache",
        "tavern.playCityGuard": "Stadtwache spielen",
        "tavern.from": "Von",
        "tavern.to": "Bis",
        "tavern.guardDuration": "Stadtwache Dauer (Stunden)",
        "tavern.diceGame": "Wuerfelspiel",
        "tavern.playDiceGame": "Wuerfelspiel spielen",
        "tavern.diceSkipHG": "Wartezeit mit Sanduhren ueberspringen",
        "tavern.dicePriority": "Wuerfelspiel Prioritaeten",

        // Items Settings
        "items.title": "Items",
        "items.inventoryManagement": "Inventar Management",
        "items.enableInventory": "Inventar Management aktivieren",
        "items.minGold": "Minimum Gold behalten",
        "items.minMushrooms": "Minimum Pilze behalten",
        "items.freeSlots": "Freie Inventarplaetze",
        "items.ignoreGemMine": "Min Gold bei Edelsteinsuche ignorieren",
        "items.sellOptions": "Verkaufsoptionen",
        "items.sellCheapest": "Billigstes Item verkaufen",
        "items.sellExpensive": "Teuerstes Item verkaufen",
        "items.dontSellEpics": "Epics nicht verkaufen",
        "items.throwCauldron": "Sofort in Hexenkessel werfen",
        "items.excludeEpics": "Epics vom Hexenkessel ausschliessen",
        "items.enchant": "Items verzaubern",
        "items.weapon": "Waffe",
        "items.hat": "Helm",
        "items.chest": "Brustplatte",
        "items.gloves": "Handschuhe",
        "items.boots": "Stiefel",
        "items.necklace": "Halskette",
        "items.belt": "Guertel",
        "items.ring": "Ring",
        "items.talisman": "Talisman",
        "items.potions": "Traenke",
        "items.drink": "Trinken",
        "items.sell": "Verkaufen",
        "items.keep": "Behalten",
        "items.buy": "Kaufen",
        "items.winged": "Gefluegelt",
        "items.strSmall": "Str Klein",
        "items.strMedium": "Str Mittel",
        "items.strLarge": "Str Gross",
        "items.dexSmall": "Dex Klein",
        "items.dexMedium": "Dex Mittel",
        "items.dexLarge": "Dex Gross",
        "items.intSmall": "Int Klein",
        "items.intMedium": "Int Mittel",
        "items.intLarge": "Int Gross",
        "items.constSmall": "Konst Klein",
        "items.constMedium": "Konst Mittel",
        "items.constLarge": "Konst Gross",
        "items.luckSmall": "Glueck Klein",
        "items.luckMedium": "Glueck Mittel",
        "items.luckLarge": "Glueck Gross",
        "items.buyHourglasses": "Sanduhren im Shop kaufen",
        "items.brewPotions": "Traenke mit Fruechten brauen (Level 632+)",
        "items.gems": "Edelsteine",
        "items.strength": "Staerke",
        "items.dexterity": "Geschick",
        "items.intelligence": "Intelligenz",
        "items.constitution": "Konstitution",
        "items.luck": "Glueck",
        "items.black": "Schwarz",
        "items.legendary": "Legendaer",
        "items.keepGemPercent": "Edelsteine behalten wenn >= % der aktuellen",
        "items.equipmentSwap": "Ausruestung tauschen",
        "items.enableSwap": "Ausruestung tauschen aktivieren",

        // Arena Settings
        "arena.title": "Arena",
        "arena.enable": "Arena aktivieren",
        "arena.stopAfter10": "Nach 10 Siegen stoppen",
        "arena.fillScrapbook": "Sammelalbum nach EXP-Kaempfen fuellen",

        // Quarters Settings
        "quarters.title": "Quartier",
        "quarters.guildAttacks": "Gilden-Angriffe",
        "quarters.orderAttack": "Angriff auf bevorzugte Gilden",
        "quarters.preferredEnemies": "Bevorzugte Gegner (getrennt durch /)",
        "quarters.firstAttackTime": "Erste Angriffszeit",
        "quarters.secondAttackTime": "Zweite Angriffszeit",
        "quarters.guildParticipation": "Gilden-Teilnahme",
        "quarters.signUpAttacks": "Fuer Gilden-Angriffe anmelden",
        "quarters.signUpDefense": "Fuer Gilden-Verteidigung anmelden",
        "quarters.signUpHydra": "Fuer Gilden-Hydra anmelden",
        "quarters.fightPortal": "Gilden-Portal kaempfen",
        "quarters.other": "Sonstiges",
        "quarters.collectMail": "Post-Belohnungen sammeln",
        "quarters.spinWheel": "Gluecksrad drehen",
        "quarters.luckyCoins": "Gluecksmuenzen",
        "quarters.mushrooms": "Pilze",
        "quarters.spinAmount": "Anzahl Drehungen",
        "quarters.hellevator": "Hellevator",
        "quarters.playHellevator": "Hellevator spielen",
        "quarters.keepKeycards": "Keycards behalten",
        "quarters.claimDaily": "Taegliche Belohnungen abholen",
        "quarters.claimFinal": "Finale Belohnung abholen",
        "quarters.joinRaid": "Hell Attack beitreten",
        "quarters.raidFloor": "Hell Attack Etage",

        // Fortress Settings
        "fortress.title": "Festung",
        "fortress.collectResources": "Ressourcen sammeln",
        "fortress.collectWood": "Holz sammeln",
        "fortress.collectStone": "Stein sammeln",
        "fortress.collectExp": "EXP sammeln",
        "fortress.collectFrom": "Sammeln von",
        "fortress.collectTo": "bis",
        "fortress.searchGems": "Nach Edelsteinen suchen",
        "fortress.attacks": "Angriffe",
        "fortress.doAttacks": "Festungs-Angriffe durchfuehren",
        "fortress.oneSoldier": "1 Soldat nutzen",
        "fortress.proposedSoldiers": "Vorgeschlagene Soldaten",
        "fortress.additionalPercent": "Zusaetzliche % Soldaten",
        "fortress.training": "Training",
        "fortress.trainSoldiers": "Soldaten trainieren",
        "fortress.trainMages": "Magier trainieren",
        "fortress.trainArchers": "Bogenschuetzen trainieren",
        "fortress.buildings": "Gebaeude",
        "fortress.upgradeBuildings": "Festungs-Gebaeude upgraden",

        // Underworld Settings
        "underworld.title": "Unterwelt",
        "underworld.resources": "Ressourcen",
        "underworld.collectSouls": "Seelen sammeln",
        "underworld.collectGold": "Gold sammeln",
        "underworld.collectThirst": "Durst sammeln",
        "underworld.dontCollectFrom": "Gold nicht sammeln von",
        "underworld.dontCollectTo": "bis",
        "underworld.upgrades": "Upgrades",
        "underworld.upgradeBuildings": "Unterwelt-Gebaeude upgraden",
        "underworld.upgradeKeeper": "Keeper upgraden",
        "underworld.keepSouls": "Seelen behalten",
        "underworld.attacks": "Angriffe",
        "underworld.performAttacks": "Vorgeschlagene Angriffe durchfuehren",
        "underworld.attackFavourite": "Bevorzugten Gegner angreifen",
        "underworld.favouriteOpponent": "Bevorzugter Gegner",

        // Character Settings
        "character.title": "Charakter",
        "character.attributes": "Attribute",
        "character.increaseStats": "Attribute erhoehen",
        "character.statDistribution": "Stat Verteilung (%)",
        "character.str": "Staerke",
        "character.dex": "Geschick",
        "character.int": "Intelligenz",
        "character.const": "Konstitution",
        "character.luck": "Glueck",
        "character.mount": "Reittier",
        "character.enableMount": "Reittier kaufen aktivieren",
        "character.bestMount": "Bestes leistbares",
        "character.griffon": "Greif/Drache",
        "character.tiger": "Tiger",
        "character.horse": "Pferd",
        "character.cow": "Kuh",

        // Pets Settings
        "pets.title": "Haustiere",
        "pets.fights": "Kaempfe",
        "pets.doFights": "Pet-Arena Kaempfe",
        "pets.doDungeons": "Pet-Dungeon Kaempfe",
        "pets.feeding": "Fuettern",
        "pets.doFeed": "Pets fuettern",
        "pets.cheapest": "Guenstigster Weg",
        "pets.expensive": "Shroomer Weg",
        "pets.feedPerDay": "Pets pro Habitat fuettern",

        // Misc Settings
        "misc.title": "Sonstiges",
        "misc.calendar": "Kalender",
        "misc.dontCollectBefore": "Kalender nicht sammeln vor",
        "misc.collectAll": "Alle Kalender sammeln",
        "misc.collectExpOnly": "Nur EXP-Kalender sammeln",
        "misc.collectMushrooms": "Pilz-Kalender beruecksichtigen",
        "misc.rewards": "Belohnungen",
        "misc.collectDaily": "Taegliche Belohnungen sammeln",
        "misc.collectWeekly": "Woechentliche Belohnungen sammeln",
        "misc.dailyTasks": "Taegliche Aufgaben",
        "misc.gambling": "Gluecksspiel",
        "misc.bareHand": "Unbewaffneter Angriff",
        "misc.defeatWarrior": "3x Krieger besiegen",
        "misc.defeatScout": "3x Kundschafter besiegen",
        "misc.defeatMage": "3x Magier besiegen",
        "misc.defeatAssassin": "3x Assassine besiegen",
        "misc.defeatBattleMage": "3x Kampfmagier besiegen",
        "misc.defeatBerserker": "3x Berserker besiegen",
        "misc.defeatDruid": "3x Druide besiegen",
        "misc.defeatDemonHunter": "3x Daemonenjaeger besiegen",
        "misc.defeatBard": "3x Barde besiegen",
        "misc.defeatNecromancer": "3x Nekromant besiegen",
        "misc.defeatPaladin": "3x Paladin besiegen",
        "misc.breakTimes": "Pausenzeiten",
        "misc.noActionsFrom": "Keine Aktionen von",
        "misc.noActionsTo": "bis",

        // Dungeons Settings
        "dungeons.title": "Dungeons",
        "dungeons.enable": "Dungeons aktivieren",
        "dungeons.fightPortal": "Daemonen-Portal kaempfen",
        "dungeons.bestWinrate": "Dungeon mit bester Gewinnrate",
        "dungeons.skip": "Dungeons ueberspringen",
        "dungeons.skipIdols": "Loop of Idols ueberspringen",
        "dungeons.skipTwister": "Twister ueberspringen",
        "dungeons.skipTower": "Tower ueberspringen",
        "dungeons.skipSandstorm": "Sandstorm ueberspringen",
        "dungeons.arenaManager": "Arena Manager",
        "dungeons.enableManager": "Arena Manager aktivieren",
        "dungeons.sacrificePercent": "Opfern nach % der aktuellen Runen",
        "dungeons.sacrificeAfterToiletCycle": "Nach jedem Toilettenzyklus opfern",

        // Toilet Settings
        "toilet.title": "Toilette",
        "toilet.enable": "Arkane Toilette aktivieren",
        "toilet.flushWhenFull": "Toilette spuelen wenn voll",
        "toilet.priorityHint": "Prioritaet: 1. Epic, 2. Edelstein, 3. Normales Item",
        "toilet.sacrificeEpics": "Epics opfern",
        "toilet.excludeWeapons": "Epic Waffen ausschliessen",
        "toilet.sacrificeGems": "Edelsteine opfern",
        "toilet.sacrificeNormal": "Normale Items opfern",

        // Log Modal
        "logModal.previous": "Vorheriger",
        "logModal.close": "Schliessen",
        "logModal.next": "Naechster",

        // Expedition Stats
        "expeditionStats.title": "Expeditions-Stats",
        "expeditionStats.close": "Schliessen",
        "expeditionStats.loading": "Lade...",
        "expeditionStats.noData": "Keine Expeditions-Stats gefunden",
        "expeditionStats.modeAll": "Alle",
        "expeditionStats.modeExp": "EXP",
        "expeditionStats.modeGold": "Gold",
        "expeditionStats.character": "Charakter",
        "expeditionStats.server": "Server",
        "expeditionStats.runs": "Expeditionen",
        "expeditionStats.heroismAvg": "Heroism Durchschnitt",
        "expeditionStats.heroismMax": "Heroism Max",
        "expeditionStats.heroismLast": "Heroism Letzte",
        "expeditionStats.keysAvg": "Keys Durchschnitt",
        "expeditionStats.chestsAvg": "Chests Durchschnitt",
        "expeditionStats.encounters": "Begegnungen",
        // Expedition Summary
        "expeditionSummary.button": "Expeditions-Uebersicht",
        "expeditionSummary.title": "Expeditions-Uebersicht",
        "expeditionSummary.close": "Schliessen",
        "expeditionSummary.loading": "Lade...",
        "expeditionSummary.noData": "Keine Expeditions-Stats gefunden",
        "expeditionSummary.modeAll": "Alle",
        "expeditionSummary.modeExp": "EXP",
        "expeditionSummary.modeGold": "Gold",
        "expeditionSummary.expedition": "Expedition",
        "expeditionSummary.runs": "Expeditionen",
        "expeditionSummary.heroismAvg": "Heroism Durchschnitt",
        "expeditionSummary.keysAvg": "Keys Durchschnitt",
        "expeditionSummary.chestsAvg": "Chests Durchschnitt"
    }
};

let currentLanguage = 'en';

// Get translation
function t(key) {
    return translations[currentLanguage][key] || translations['en'][key] || key;
}

// Set language and update UI
function setLanguage(lang) {
    currentLanguage = lang;
    document.documentElement.lang = lang;
    updateUILanguage();
    saveLanguagePreference(lang);
}

// Update all UI elements with data-i18n attributes
function updateUILanguage() {
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        if (el.tagName === 'INPUT' && el.type === 'text') {
            el.placeholder = t(key);
        } else {
            el.textContent = t(key);
        }
    });

    // Update language switcher active state
    document.querySelectorAll('.lang-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.lang === currentLanguage);
    });
}

// Save language preference to global settings
async function saveLanguagePreference(lang) {
    try {
        const response = await fetch('/api/settings');
        const data = await response.json();
        data.globalLanguage = lang;
        await fetch('/api/settings', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ settings: data })
        });
    } catch (e) {
        console.log('Could not save language preference');
    }
}

// Load language preference from global settings
async function loadLanguagePreference() {
    try {
        const response = await fetch('/api/settings');
        const data = await response.json();
        if (data.globalLanguage) {
            currentLanguage = data.globalLanguage;
        }
    } catch (e) {
        console.log('Could not load language preference, using default');
    }
    updateUILanguage();
}

// Initialize language on load
document.addEventListener('DOMContentLoaded', () => {
    loadLanguagePreference();
});
