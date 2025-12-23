/**
 * SF Bot Modern UI - Main JavaScript
 */

// ============================================================================
// State
// ============================================================================

const state = {
    running: false,
    paused: false,
    accounts: [],
    characters: [],
    pendingActiveOverrides: {}, // key: `${id}_${name}` -> bool
    refreshInterval: null,
    currentCharacter: null,
    currentCharacterSettings: null,
    currentCharacterSettingsSnapshot: null,
    applySettingsToAll: false,
    accountFilter: '' // Empty string = show all, otherwise filter by account name
};

const REFRESH_INTERVAL = 5000;

// Priority lists (with defaults)
const DEFAULT_EXPEDITION_PRIORITY_LIST = [
    "Mushrooms", "Gold", "Wood", "Stone", "Arcane Splinter",
    "Metal", "Souls", "Pet Egg", "Quicksand Glasses", "Fruit Basket", "Lucky coins"
];

const DEFAULT_DICE_PRIORITY_LIST = [
    "Gold", "HourGlass", "Reroll", "Souls", "Arcane Splinter", "Wood", "Stone"
];

let expeditionPriorityList = [...DEFAULT_EXPEDITION_PRIORITY_LIST];
let dicePriorityList = [...DEFAULT_DICE_PRIORITY_LIST];
let isRedeemingCoupon = false;
let couponStatusInterval = null;
let couponProgressDismissed = false;

// ============================================================================
// Initialization
// ============================================================================

document.addEventListener('DOMContentLoaded', async () => {
    console.log('SF Bot UI initializing...');

    // Check server connection
    const connected = await checkServerConnection();
    if (!connected) {
        showLog('Server nicht erreichbar. Bitte Backend starten.', 'error');
    }

    // Load version
    await loadVersion();

    // Load accounts
    await loadAccounts();

    // Load cached characters (so they appear before bot starts)
    await loadCachedCharacters();

    // Setup event handlers
    setupNavigation();
    setupBotControls();
    setupLoginForm();
    setupModals();
    setupLogModal();
    setupExpeditionStatsModal();
    setupExpeditionSummaryModal();
    setupCouponModal();
    setupSettingsNavigation();
    await resumeCouponStatusPolling();

    // Start refresh interval
    startRefreshInterval();

    // Hide loading overlay
    document.getElementById('loading-overlay').classList.add('hidden');

    console.log('SF Bot UI initialized');
});

// ============================================================================
// Server Connection
// ============================================================================

async function checkServerConnection() {
    try {
        const response = await fetch('/api/version');
        return response.ok;
    } catch (e) {
        console.error('Server not reachable:', e);
        return false;
    }
}

async function loadVersion() {
    try {
        const result = await invoke('get_app_version');
        document.getElementById('version').textContent = `v${result}`;
    } catch (e) {
        console.error('Failed to load version:', e);
    }
}

// ============================================================================
// Navigation
// ============================================================================

function setupNavigation() {
    const navBtns = document.querySelectorAll('.nav-btn');
    const views = document.querySelectorAll('.view');
    const viewTitle = document.getElementById('view-title');

    const viewTitles = {
        'dashboard': 'Dashboard',
        'accounts': 'Accounts',
        'logs': 'Logs'
    };

    navBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const viewId = btn.dataset.view;

            // Update nav buttons
            navBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            // Update views
            views.forEach(v => v.classList.remove('active'));
            document.getElementById(`view-${viewId}`).classList.add('active');

            // Update title
            viewTitle.textContent = viewTitles[viewId] || viewId;

            // Reset account filter when clicking Dashboard
            if (viewId === 'dashboard') {
                setAccountFilter('');
            }
        });
    });
}

// ============================================================================
// Bot Controls
// ============================================================================

function setupBotControls() {
    document.getElementById('btn-start').addEventListener('click', startBot);
    document.getElementById('btn-stop').addEventListener('click', stopBot);
    document.getElementById('btn-pause').addEventListener('click', togglePause);
    document.getElementById('btn-refresh').addEventListener('click', refreshData);
    document.getElementById('btn-shutdown').addEventListener('click', shutdownServer);
    const summaryBtn = document.getElementById('btn-expedition-summary');
    if (summaryBtn) summaryBtn.addEventListener('click', openExpeditionSummary);
    const settingsAllBtn = document.getElementById('btn-settings-all');
    if (settingsAllBtn) settingsAllBtn.addEventListener('click', openAllCharacterSettings);
    const couponBtn = document.getElementById('btn-redeem-coupon');
    if (couponBtn) couponBtn.addEventListener('click', openCouponModal);
}

async function shutdownServer() {
    if (confirm('Server wirklich beenden? Der Bot wird gestoppt und die Anwendung geschlossen.')) {
        try {
            showLog('Server wird beendet...', 'info');
            await window.sfBotApi.shutdownServer();
        } catch (e) {
            // Server ist bereits beendet, daher Fehler erwartet
        }
    }
}

async function startBot() {
    if (state.accounts.length === 0) {
        showLog('Keine Accounts vorhanden. Bitte zuerst Account hinzufuegen.', 'error');
        return;
    }

    try {
        showLog('Bot wird gestartet...', 'info');

        const accountsToStart = state.accounts.map(acc => ({
            accname: acc.accname,
            password: acc.password,
            single: acc.single || false,
            server: acc.server || ''
        }));

        await window.sfBotApi.startBot(accountsToStart);

        state.running = true;
        state.paused = false;

        updateBotUI();
        showLog('Bot gestartet', 'success');

    } catch (e) {
        console.error('Failed to start bot:', e);
        showLog('Bot konnte nicht gestartet werden: ' + e.message, 'error');
    }
}

async function stopBot() {
    try {
        showLog('Bot wird gestoppt...', 'info');

        await window.sfBotApi.stopBot();

        state.running = false;
        state.paused = false;

        updateBotUI();
        showLog('Bot gestoppt', 'success');

    } catch (e) {
        console.error('Failed to stop bot:', e);
        showLog('Fehler beim Stoppen: ' + e.message, 'error');
    }
}

async function togglePause() {
    try {
        if (state.paused) {
            await window.sfBotApi.resumeBot();
            state.paused = false;
            showLog('Bot fortgesetzt', 'info');
        } else {
            await window.sfBotApi.pauseBot();
            state.paused = true;
            showLog('Bot pausiert', 'info');
        }
        updateBotUI();
    } catch (e) {
        console.error('Failed to toggle pause:', e);
    }
}

function updateBotUI() {
    const statusEl = document.getElementById('bot-status');
    const statusText = statusEl.querySelector('.status-text');
    const startBtn = document.getElementById('btn-start');
    const stopBtn = document.getElementById('btn-stop');
    const pauseBtn = document.getElementById('btn-pause');

    // Update status indicator
    statusEl.classList.remove('running', 'paused');
    if (state.running) {
        statusEl.classList.add(state.paused ? 'paused' : 'running');
        statusText.textContent = state.paused ? t('header.paused') : t('header.running');
    } else {
        statusText.textContent = t('header.stopped');
    }

    // Update buttons
    startBtn.disabled = state.running;
    stopBtn.disabled = !state.running;
    pauseBtn.disabled = !state.running;

    // Update pause button text
    const pauseBtnTextEl = pauseBtn.querySelector('svg');
    if (pauseBtnTextEl && pauseBtnTextEl.nextSibling) {
        pauseBtnTextEl.nextSibling.textContent = ' ' + (state.paused ? t('header.resume') : t('header.pause'));
    }

}

// ============================================================================
// Data Refresh
// ============================================================================

function startRefreshInterval() {
    state.refreshInterval = setInterval(refreshBotStatus, REFRESH_INTERVAL);
}

async function refreshData() {
    await refreshBotStatus();
    showLog(t('dashboard.dataRefreshed'), 'info');
}

async function refreshBotStatus() {
    try {
        const status = await window.sfBotApi.getBotStatus();

        state.running = status.running;
        state.paused = status.paused;

        updateBotUI();

        // Update characters from status - merge with cached inactive characters
        if (status.characters && status.characters.length > 0) {
            // Map existing characters (cached or last known) for enrichment/fallbacks
            const cachedMap = new Map(
                state.characters.map(c => [`${c.id}_${c.server}`, c])
            );

            // Active/status characters enriched with cached details if available
            const activeChars = status.characters.map(c => {
                const key = `${c.id}_${c.server}`;
                const cached = cachedMap.get(key);
                return {
                    ...c,
                    cached: false,
                    // Prefer live values; fall back to cache only if missing
                    mount: c.mount || cached?.mount || '-',
                    luckycoins: c.luckycoins ?? cached?.luckycoins ?? 0,
                    hourglasses: c.hourglasses ?? cached?.hourglasses ?? 0,
                    beers: c.beers ?? cached?.beers ?? 0,
                    mushrooms: c.mushrooms ?? cached?.mushrooms ?? 0,
                    gold: c.gold ?? cached?.gold ?? 0,
                    guild: c.guild || cached?.guild || '',
                    petfights: c.petfights ?? cached?.petfights ?? 0,
                    dicerolls: c.dicerolls ?? cached?.dicerolls ?? 0,
                    current_action: c.current_action || cached?.current_action || '-',
                };
            });

            const activeIds = new Set(activeChars.map(c => `${c.id}_${c.server}`));

            // Cached characters not present in active list (regardless of isActive) stay in the UI
            const cachedMissing = Array.from(cachedMap.values()).filter(c => !activeIds.has(`${c.id}_${c.server}`));

            state.characters = [...activeChars, ...cachedMissing];
            // Apply pending overrides for isActive
            state.characters = state.characters.map(c => {
                const key = `${c.id}_${c.name}`;
                if (state.pendingActiveOverrides.hasOwnProperty(key)) {
                    return { ...c, isActive: state.pendingActiveOverrides[key] };
                }
                return c;
            });
            updateAccountSubmenu();
            renderCharactersTable();
        }

        // Update stats
        document.getElementById('stat-accounts').textContent = state.accounts.length;

    } catch (e) {
        console.error('Failed to refresh status:', e);
    }
}

// ============================================================================
// Accounts
// ============================================================================

async function loadAccounts() {
    try {
        state.accounts = await invoke('read_user_conf') || [];
        renderAccountsList();
        document.getElementById('stat-accounts').textContent = state.accounts.length;

        if (state.accounts.length > 0) {
            showLog(`${state.accounts.length} Account(s) geladen`, 'info');
        }
    } catch (e) {
        console.error('Failed to load accounts:', e);
    }
}

/**
 * Load cached characters from the server
 * This allows displaying characters before the bot is started
 */
async function loadCachedCharacters() {
    try {
        const result = await window.sfBotApi.getCachedCharacters();

        if (result.characters && result.characters.length > 0) {
            // Map cached characters to the format expected by the UI
            state.characters = result.characters.map(c => ({
                id: c.id,
                name: c.name,
                lvl: c.lvl,
                alu: c.alu,
                guild: c.guild,
                beers: c.beers,
                mushrooms: c.mushrooms,
                hourglasses: c.hourglasses,
                gold: c.gold,
                luckycoins: c.luckycoins,
                fights: c.fights,
                luckyspins: c.luckyspins,
                petfights: c.petfights,
                dicerolls: c.dicerolls,
                server: c.server,
                isActive: c.isActive,
                mount: c.mount,
                account: c.account,
                current_action: '-',
                cached: true,
                cachedAt: c.cached_at
            }));

            renderCharactersTable();
            document.getElementById('stat-characters').textContent = state.characters.length;
            showLog(t('log.charsLoadedFromCache').replace('{count}', state.characters.length), 'info');
        }
    } catch (e) {
        console.error('Failed to load cached characters:', e);
        // Not an error - just means no cache exists yet
    }
}

function renderAccountsList() {
    const container = document.getElementById('accounts-list');

    if (state.accounts.length === 0) {
        container.innerHTML = '<div class="empty-state">Keine Accounts gespeichert</div>';
        return;
    }

    container.innerHTML = state.accounts.map(acc => `
        <div class="account-item">
            <span class="account-name">${acc.accname}</span>
            <span class="account-type">${acc.single ? 'Single' : 'SSO'}</span>
        </div>
    `).join('');
}

function setupLoginForm() {
    const form = document.getElementById('login-form');
    const singleCheckbox = document.getElementById('input-single');
    const serverGroup = document.getElementById('server-group');

    // Toggle server input
    singleCheckbox.addEventListener('change', () => {
        serverGroup.style.display = singleCheckbox.checked ? 'block' : 'none';
    });

    // Handle form submit
    form.addEventListener('submit', async (e) => {
        e.preventDefault();

        const username = document.getElementById('input-username').value;
        const password = document.getElementById('input-password').value;
        const isSingle = document.getElementById('input-single').checked;
        const server = document.getElementById('input-server').value;

        if (!username || !password) {
            showLog('Bitte Benutzername und Passwort eingeben', 'error');
            return;
        }

        if (isSingle && !server) {
            showLog('Bitte Server-URL eingeben', 'error');
            return;
        }

        try {
            showLog(`Login ${username}...`, 'info');

            // Save to config
            await invoke('save_user_conf', {
                accname: username,
                password: password,
                single: isSingle,
                server: server || ''
            });

            // Reload accounts
            await loadAccounts();

            // Clear form
            form.reset();
            serverGroup.style.display = 'none';

            showLog(`Account ${username} hinzugefuegt`, 'success');

        } catch (e) {
            console.error('Failed to add account:', e);
            showLog('Fehler beim Hinzufuegen: ' + e.message, 'error');
        }
    });
}

// ============================================================================
// Account Submenu
// ============================================================================

function updateAccountSubmenu() {
    const submenu = document.getElementById('account-submenu');
    if (!submenu) return;

    // Get unique account names from characters
    const accounts = [...new Set(state.characters.map(c => c.account).filter(a => a))];

    if (accounts.length === 0) {
        submenu.innerHTML = '';
        return;
    }

    submenu.innerHTML = accounts.map(acc => `
        <button class="account-submenu-btn ${state.accountFilter === acc ? 'active' : ''}" data-account="${acc}">
            ${acc}
        </button>
    `).join('');

    // Add click handlers
    submenu.querySelectorAll('.account-submenu-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const account = e.target.dataset.account;
            setAccountFilter(account);
        });
    });
}

function setAccountFilter(account) {
    state.accountFilter = account;

    // Update active states
    const dashboardBtn = document.querySelector('.nav-btn[data-view="dashboard"]');
    const submenuBtns = document.querySelectorAll('.account-submenu-btn');

    // Dashboard button is active only when no filter
    if (account === '') {
        dashboardBtn.classList.add('active');
    } else {
        dashboardBtn.classList.remove('active');
    }

    // Update submenu button states
    submenuBtns.forEach(btn => {
        if (btn.dataset.account === account) {
            btn.classList.add('active');
        } else {
            btn.classList.remove('active');
        }
    });

    // Re-render table
    renderCharactersTable();
}

// ============================================================================
// Characters Table
// ============================================================================

function renderCharactersTable() {
    const tbody = document.getElementById('character-list');

    // Filter characters by account if filter is set
    let displayCharacters = state.characters;
    if (state.accountFilter) {
        displayCharacters = state.characters.filter(c => c.account === state.accountFilter);
    }

    if (displayCharacters.length === 0) {
        tbody.innerHTML = `
            <tr class="empty-row">
                <td colspan="17">${state.accountFilter ? 'Keine Charaktere fuer diesen Account.' : 'Keine Charaktere geladen. Bitte Account hinzufuegen und Bot starten.'}</td>
            </tr>
        `;
        return;
    }

    // Sort: active first, then by level (desc), then name
    const sortedCharacters = [...displayCharacters].sort((a, b) => {
        const activeA = !!a.isActive;
        const activeB = !!b.isActive;
        if (activeA !== activeB) return activeA ? -1 : 1; // active above inactive

        const lvlA = a.lvl || 0;
        const lvlB = b.lvl || 0;
        if (lvlA !== lvlB) return lvlB - lvlA; // higher level first

        return (a.name || '').localeCompare(b.name || '');
    });

    tbody.innerHTML = sortedCharacters.map(char => {
        // Single accounts have server set, normal accounts don't - show dash for normal accounts
        const serverDisplay = char.server ? char.server : '-';
        return `
        <tr>
            <td><input type="checkbox" class="char-active-toggle" data-char-id="${char.id}" data-char-name="${char.name}" ${char.isActive ? 'checked' : ''}></td>
            <td>${char.name}</td>
            <td>${char.lvl || '-'}</td>
            <td>${serverDisplay}</td>
            <td>${char.guild || '-'}</td>
            <td>${formatNumber(char.gold || 0)}</td>
            <td>${char.mushrooms || 0}</td>
            <td>${char.luckycoins ?? 0}</td>
            <td>${char.hourglasses ?? 0}</td>
            <td>${char.mount || '-'}</td>
            <td>${char.beers || 0}/11</td>
            <td>${char.fights || 0}/10</td>
            <td>${char.petfights ?? 0}</td>
            <td>${char.dicerolls ?? 0}</td>
            <td>${char.alu || 0}</td>
            <td>${char.current_action || '-'}</td>
            <td>
                <button class="char-exp-stats-btn" data-char-id="${char.id}" data-char-name="${char.name}" data-char-server="${char.server || ''}">${t('table.stats')}</button>
                <button class="char-log-btn" data-char-id="${char.id}" data-char-name="${char.name}">${t('table.log')}</button>
                <button class="char-settings-btn" data-char-id="${char.id}" data-char-name="${char.name}">${t('table.settings')}</button>
            </td>
        </tr>
    `}).join('');

    // Update stats
    document.getElementById('stat-characters').textContent = state.characters.length;
    document.getElementById('stat-active').textContent = state.characters.filter(c => c.isActive).length;

    // Add event listeners
    tbody.querySelectorAll('.char-active-toggle').forEach(checkbox => {
        checkbox.addEventListener('change', (e) => {
            const charId = parseInt(e.target.dataset.charId);
            const charName = e.target.dataset.charName;
            toggleCharacterActive(charId, charName, e.target.checked);
        });
    });

    tbody.querySelectorAll('.char-settings-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const charId = parseInt(e.target.dataset.charId);
            const charName = e.target.dataset.charName;
            openCharacterSettings(charId, charName);
        });
    });

    tbody.querySelectorAll('.char-log-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const charId = parseInt(e.target.dataset.charId);
            const charName = e.target.dataset.charName;
            openCharacterLog(charId, charName);
        });
    });

    tbody.querySelectorAll('.char-exp-stats-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const charId = parseInt(e.target.dataset.charId);
            const charName = e.target.dataset.charName;
            const charServer = e.target.dataset.charServer || '';
            openExpeditionStats(charId, charName, charServer);
        });
    });
}

async function toggleCharacterActive(charId, charName, isActive) {
    try {
        const settings = await invoke('load_character_settings', {
            charactername: charName,
            characterid: charId
        }) || {};

        settings.settingCharacterActive = isActive;

        await invoke('save_character_settings', {
            charactername: charName,
            characterid: charId,
            settings: settings
        });

        // Remember override locally so the next refresh keeps the checkbox state immediately
        const overrideKey = `${charId}_${charName}`;
        state.pendingActiveOverrides[overrideKey] = isActive;

        // Update local list right away for snappier UI
        const idx = state.characters.findIndex(c => c.id === charId && c.name === charName);
        if (idx !== -1) {
            state.characters[idx] = { ...state.characters[idx], isActive };
            renderCharactersTable();
        }

        showLog(`${charName}: ${isActive ? 'Aktiviert' : 'Deaktiviert'}`, 'info');

    } catch (e) {
        console.error('Failed to toggle character active:', e);
    }
}

function formatNumber(num) {
    if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
    if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
    return num;
}

// ============================================================================
// Character Log Modal
// ============================================================================

let currentLogCharIndex = 0;
let currentExpeditionStats = null;
let currentExpeditionStatsFallback = null;
let currentExpeditionStatsMode = 'all';
let currentExpeditionSummary = null;
let currentExpeditionSummaryMode = 'all';

async function openCharacterLog(charId, charName) {
    // Find the index of this character in the sorted list
    const sortedCharacters = [...state.characters].sort((a, b) => a.id - b.id);
    currentLogCharIndex = sortedCharacters.findIndex(c => c.id === charId);

    await loadAndDisplayLog(charName, charId);
    document.getElementById('character-log-modal').classList.add('active');
}

async function loadAndDisplayLog(charName, charId) {
    const logContent = document.getElementById('character-log-content');
    const title = document.getElementById('log-modal-title');

    title.textContent = `Log: ${charName}`;
    logContent.textContent = 'Lade...';

    try {
        const result = await invoke('get_character_log', { name: charName, id: charId });
        if (result.error) {
            logContent.textContent = result.error;
        } else {
            logContent.textContent = result.log || t('logs.empty');
        }
        // Scroll to bottom after content is rendered
        requestAnimationFrame(() => {
            logContent.scrollTop = logContent.scrollHeight;
        });
    } catch (e) {
        logContent.textContent = 'Fehler beim Laden: ' + e.message;
    }
}

function navigateLogCharacter(direction) {
    const sortedCharacters = [...state.characters].sort((a, b) => a.id - b.id);
    if (sortedCharacters.length === 0) return;

    currentLogCharIndex += direction;
    if (currentLogCharIndex < 0) currentLogCharIndex = sortedCharacters.length - 1;
    if (currentLogCharIndex >= sortedCharacters.length) currentLogCharIndex = 0;

    const char = sortedCharacters[currentLogCharIndex];
    loadAndDisplayLog(char.name, char.id);
}

function setupLogModal() {
    document.getElementById('close-char-log').addEventListener('click', () => {
        document.getElementById('character-log-modal').classList.remove('active');
    });

    document.getElementById('log-close').addEventListener('click', () => {
        document.getElementById('character-log-modal').classList.remove('active');
    });

    document.getElementById('log-prev-char').addEventListener('click', () => {
        navigateLogCharacter(-1);
    });

    document.getElementById('log-next-char').addEventListener('click', () => {
        navigateLogCharacter(1);
    });
}

// ============================================================================
// Expedition Stats Modal
// ============================================================================

async function openExpeditionStats(charId, charName, charServer) {
    const modal = document.getElementById('expedition-stats-modal');
    const title = document.getElementById('expedition-stats-title');
    const summary = document.getElementById('expedition-stats-summary');
    const content = document.getElementById('expedition-stats-content');

    title.textContent = `${t('expeditionStats.title')}: ${charName}`;
    summary.innerHTML = '';
    content.textContent = t('expeditionStats.loading');
    content.classList.add('expedition-stats-empty');

    modal.classList.add('active');

    try {
        const result = await invoke('get_character_expedition_stats', {
            name: charName,
            id: charId,
            server: charServer || ''
        });

        currentExpeditionStats = result?.stats || null;
        currentExpeditionStatsFallback = {
            name: charName,
            id: charId,
            server: charServer || ''
        };
        setExpeditionStatsMode(currentExpeditionStatsMode);
    } catch (e) {
        content.textContent = 'Fehler beim Laden: ' + e.message;
    }
}

function renderExpeditionStats(stats, fallback, mode) {
    const summary = document.getElementById('expedition-stats-summary');
    const content = document.getElementById('expedition-stats-content');
    summary.innerHTML = '';
    content.innerHTML = '';

    const expeditions = extractModeExpeditions(stats, mode);
    if (!expeditions || Object.keys(expeditions).length === 0) {
        content.textContent = t('expeditionStats.noData');
        content.classList.add('expedition-stats-empty');
        return;
    }

    content.classList.remove('expedition-stats-empty');

    const expeditionEntries = Object.entries(expeditions);
    expeditionEntries.sort((a, b) => {
        const nameA = formatExpeditionName(a[0]).toLowerCase();
        const nameB = formatExpeditionName(b[0]).toLowerCase();
        return nameA.localeCompare(nameB);
    });

    let totalRuns = 0;
    let totalHeroism = 0;
    let maxHeroism = 0;
    let totalKeys = 0;
    let totalChests = 0;

    expeditionEntries.forEach(([, data]) => {
        const picked = Number(data?.picked || 0);
        const heroismTotal = Number(data?.heroism_total || 0);
        const heroismMax = Number(data?.heroism_max || 0);
        const encounters = data?.encounters || {};
        totalKeys += getEncounterCount(encounters, ['Key', 'Keys']);
        totalChests += getEncounterCount(encounters, ['Suitcase', 'Chests']);
        totalRuns += picked;
        totalHeroism += heroismTotal;
        if (heroismMax > maxHeroism) maxHeroism = heroismMax;
    });

    const overallAvg = totalRuns > 0 ? (totalHeroism / totalRuns) : 0;
    const avgKeys = totalRuns > 0 ? (totalKeys / totalRuns) : 0;
    const avgChests = totalRuns > 0 ? (totalChests / totalRuns) : 0;
    const characterName = stats.character || fallback.name;
    const serverName = stats.server || fallback.server || '-';

    summary.appendChild(createSummaryCard(t('expeditionStats.character'), characterName));
    summary.appendChild(createSummaryCard(t('expeditionStats.server'), serverName));
    summary.appendChild(createSummaryCard(t('expeditionStats.runs'), totalRuns.toString()));
    summary.appendChild(createSummaryCard(t('expeditionStats.heroismAvg'), overallAvg.toFixed(1)));
    summary.appendChild(createSummaryCard(t('expeditionStats.heroismMax'), maxHeroism.toString()));
    summary.appendChild(createSummaryCard(t('expeditionStats.keysAvg'), avgKeys.toFixed(2)));
    summary.appendChild(createSummaryCard(t('expeditionStats.chestsAvg'), avgChests.toFixed(2)));

    const grid = document.createElement('div');
    grid.className = 'expedition-stats-grid';

    expeditionEntries.forEach(([expeditionName, data]) => {
        const card = document.createElement('div');
        card.className = 'expedition-card';

        const header = document.createElement('div');
        header.className = 'expedition-card-header';

        const title = document.createElement('div');
        title.className = 'expedition-name';
        title.textContent = formatExpeditionName(expeditionName);

        const picked = Number(data?.picked || 0);
        const pickedEl = document.createElement('div');
        pickedEl.className = 'expedition-picked';
        pickedEl.textContent = `${picked} ${t('expeditionStats.runs')}`;

        header.appendChild(title);
        header.appendChild(pickedEl);

        const heroismRow = document.createElement('div');
        heroismRow.className = 'expedition-heroism';

        const heroismTotal = Number(data?.heroism_total || 0);
        const heroismAvg = picked > 0 ? (heroismTotal / picked) : 0;
        const heroismMax = Number(data?.heroism_max || 0);
        const heroismLast = Number(data?.heroism_last || 0);

        heroismRow.appendChild(createStatItem(t('expeditionStats.heroismAvg'), heroismAvg.toFixed(1)));
        heroismRow.appendChild(createStatItem(t('expeditionStats.heroismMax'), heroismMax.toString()));
        heroismRow.appendChild(createStatItem(t('expeditionStats.heroismLast'), heroismLast.toString()));

        const lootRow = document.createElement('div');
        lootRow.className = 'expedition-loot';
        const encounters = data?.encounters || {};
        const keysCount = getEncounterCount(encounters, ['Key', 'Keys']);
        const chestsCount = getEncounterCount(encounters, ['Suitcase', 'Chests']);
        const keysAvg = picked > 0 ? (keysCount / picked) : 0;
        const chestsAvg = picked > 0 ? (chestsCount / picked) : 0;
        lootRow.appendChild(createStatItem(t('expeditionStats.keysAvg'), keysAvg.toFixed(2)));
        lootRow.appendChild(createStatItem(t('expeditionStats.chestsAvg'), chestsAvg.toFixed(2)));

        const encountersWrapper = document.createElement('div');
        encountersWrapper.className = 'expedition-encounters';

        const encountersTitle = document.createElement('div');
        encountersTitle.className = 'encounters-title';
        encountersTitle.textContent = t('expeditionStats.encounters');
        encountersWrapper.appendChild(encountersTitle);

        const encounterList = document.createElement('div');
        encounterList.className = 'encounter-list';

        const encounterEntries = Object.entries(encounters);
        encounterEntries.sort((a, b) => (b[1] || 0) - (a[1] || 0));

        if (encounterEntries.length === 0) {
            const empty = document.createElement('div');
            empty.className = 'encounter-empty';
            empty.textContent = '-';
            encounterList.appendChild(empty);
        } else {
            encounterEntries.forEach(([encounterName, count]) => {
                const row = document.createElement('div');
                row.className = 'encounter-row';

                const nameEl = document.createElement('span');
                nameEl.textContent = formatEncounterName(encounterName);

                const countEl = document.createElement('span');
                countEl.textContent = count.toString();

                row.appendChild(nameEl);
                row.appendChild(countEl);
                encounterList.appendChild(row);
            });
        }

        encountersWrapper.appendChild(encounterList);

        card.appendChild(header);
        card.appendChild(heroismRow);
        card.appendChild(lootRow);
        card.appendChild(encountersWrapper);
        grid.appendChild(card);
    });

    content.appendChild(grid);
}

function createSummaryCard(label, value) {
    const card = document.createElement('div');
    card.className = 'expedition-summary-card';

    const labelEl = document.createElement('div');
    labelEl.className = 'expedition-summary-label';
    labelEl.textContent = label;

    const valueEl = document.createElement('div');
    valueEl.className = 'expedition-summary-value';
    valueEl.textContent = value;

    card.appendChild(labelEl);
    card.appendChild(valueEl);
    return card;
}

function createStatItem(label, value) {
    const item = document.createElement('div');
    item.className = 'heroism-item';

    const labelEl = document.createElement('div');
    labelEl.className = 'heroism-label';
    labelEl.textContent = label;

    const valueEl = document.createElement('div');
    valueEl.className = 'heroism-value';
    valueEl.textContent = value;

    item.appendChild(labelEl);
    item.appendChild(valueEl);
    return item;
}

function getEncounterCount(encounters, names) {
    return names.reduce((sum, name) => sum + (Number(encounters?.[name] || 0)), 0);
}

function formatEncounterName(name) {
    if (name === 'Suitcase') return 'Chests';
    if (name === 'Key') return 'Keys';
    if (name === 'Cake') return 'Suckling Pig';
    if (name === 'WinnersPodium') return 'Podium Climber';
    if (name === 'RoyalFrog') return 'Toxic Fountain Cure';
    if (name === 'Dragon') return 'Dragon Taming';
    if (name === 'RevealingCouple') return 'Revealing Lady';
    if (name === 'Balloons') return 'Bewitched Stew';
    if (name === 'BurntCampfire') return 'Extinguished Campfire';
    if (name === 'ToiletPaper') return 'Toilet Paper';   
    if (name === 'BrokenSword') return 'Broken Sword'; 

    return name;
}

function formatExpeditionName(name) {
    return formatEncounterName(name);
}

function extractModeExpeditions(stats, mode) {
    if (!stats) return {};
    if (!mode || mode === 'all') return stats.expeditions || {};
    return stats.modes?.[mode]?.expeditions || {};
}

function setExpeditionStatsMode(mode) {
    currentExpeditionStatsMode = mode || 'all';
    const toggle = document.getElementById('expedition-stats-toggle');
    if (toggle) {
        toggle.querySelectorAll('.toggle-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.mode === currentExpeditionStatsMode);
        });
    }
    if (currentExpeditionStats) {
        renderExpeditionStats(currentExpeditionStats, currentExpeditionStatsFallback, currentExpeditionStatsMode);
    }
}

function setupExpeditionStatsModal() {
    document.getElementById('close-expedition-stats').addEventListener('click', () => {
        document.getElementById('expedition-stats-modal').classList.remove('active');
    });

    document.getElementById('expedition-stats-close').addEventListener('click', () => {
        document.getElementById('expedition-stats-modal').classList.remove('active');
    });

    const toggle = document.getElementById('expedition-stats-toggle');
    if (toggle) {
        toggle.querySelectorAll('.toggle-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                setExpeditionStatsMode(btn.dataset.mode);
            });
        });
    }
}

// ============================================================================
// Expedition Summary Modal
// ============================================================================

async function openExpeditionSummary() {
    const modal = document.getElementById('expedition-summary-modal');
    const content = document.getElementById('expedition-summary-content');

    content.textContent = t('expeditionSummary.loading');
    content.classList.add('expedition-summary-empty');
    modal.classList.add('active');

    try {
        const result = await invoke('get_expedition_summary');
        currentExpeditionSummary = result || null;
        setExpeditionSummaryMode(currentExpeditionSummaryMode);
    } catch (e) {
        content.textContent = 'Fehler beim Laden: ' + e.message;
    }
}

function renderExpeditionSummary(summaryData, mode) {
    const content = document.getElementById('expedition-summary-content');
    content.innerHTML = '';

    const expeditions = extractSummaryMode(summaryData, mode);
    const entries = Object.entries(expeditions || {});
    if (entries.length === 0) {
        content.textContent = t('expeditionSummary.noData');
        content.classList.add('expedition-summary-empty');
        return;
    }
    content.classList.remove('expedition-summary-empty');

    entries.sort((a, b) => {
        const nameA = formatExpeditionName(a[0]).toLowerCase();
        const nameB = formatExpeditionName(b[0]).toLowerCase();
        return nameA.localeCompare(nameB);
    });

    const table = document.createElement('table');
    table.className = 'expedition-summary-table';

    table.innerHTML = `
        <thead>
            <tr>
                <th>${t('expeditionSummary.expedition')}</th>
                <th>${t('expeditionSummary.runs')}</th>
                <th>${t('expeditionSummary.heroismAvg')}</th>
                <th>${t('expeditionSummary.keysAvg')}</th>
                <th>${t('expeditionSummary.chestsAvg')}</th>
            </tr>
        </thead>
        <tbody></tbody>
    `;

    const tbody = table.querySelector('tbody');
    entries.forEach(([name, data]) => {
        const picked = Number(data?.picked || 0);
        const heroismTotal = Number(data?.heroism_total || 0);
        const keys = Number(data?.keys || 0);
        const chests = Number(data?.chests || 0);

        const heroismAvg = picked > 0 ? (heroismTotal / picked) : 0;
        const keysAvg = picked > 0 ? (keys / picked) : 0;
        const chestsAvg = picked > 0 ? (chests / picked) : 0;

        const row = document.createElement('tr');
        row.innerHTML = `
            <td>${formatExpeditionName(name)}</td>
            <td>${picked}</td>
            <td>${heroismAvg.toFixed(1)}</td>
            <td>${keysAvg.toFixed(2)}</td>
            <td>${chestsAvg.toFixed(2)}</td>
        `;
        tbody.appendChild(row);
    });

    content.appendChild(table);
}

function setupExpeditionSummaryModal() {
    document.getElementById('close-expedition-summary').addEventListener('click', () => {
        document.getElementById('expedition-summary-modal').classList.remove('active');
    });

    document.getElementById('expedition-summary-close').addEventListener('click', () => {
        document.getElementById('expedition-summary-modal').classList.remove('active');
    });

    const toggle = document.getElementById('expedition-summary-toggle');
    if (toggle) {
        toggle.querySelectorAll('.toggle-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                setExpeditionSummaryMode(btn.dataset.mode);
            });
        });
    }
}

function extractSummaryMode(summaryData, mode) {
    if (!summaryData) return {};
    if (!mode || mode === 'all') return summaryData.expeditions || {};
    return summaryData.modes?.[mode]?.expeditions || {};
}

function setExpeditionSummaryMode(mode) {
    currentExpeditionSummaryMode = mode || 'all';
    const toggle = document.getElementById('expedition-summary-toggle');
    if (toggle) {
        toggle.querySelectorAll('.toggle-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.mode === currentExpeditionSummaryMode);
        });
    }
    if (currentExpeditionSummary) {
        renderExpeditionSummary(currentExpeditionSummary, currentExpeditionSummaryMode);
    }
}

// ============================================================================
// Coupon Modal
// ============================================================================

function setupCouponModal() {
    const modal = document.getElementById('coupon-modal');
    if (!modal) return;

    const closeBtn = document.getElementById('close-coupon');
    const cancelBtn = document.getElementById('coupon-cancel');
    const redeemBtn = document.getElementById('coupon-redeem');
    const input = document.getElementById('coupon-code');
    const progressClose = document.getElementById('coupon-progress-close');

    if (closeBtn) closeBtn.addEventListener('click', closeCouponModal);
    if (cancelBtn) cancelBtn.addEventListener('click', closeCouponModal);
    if (redeemBtn) redeemBtn.addEventListener('click', redeemCouponForAll);
    if (input) {
        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                redeemCouponForAll();
            }
        });
    }
    if (progressClose) {
        progressClose.addEventListener('click', () => {
            couponProgressDismissed = true;
            setCouponProgressVisible(false);
        });
    }
}

function openCouponModal() {
    if (!isRedeemingCoupon) {
        resetCouponModal();
    }
    const modal = document.getElementById('coupon-modal');
    modal.classList.add('active');
    const input = document.getElementById('coupon-code');
    if (input) input.focus();
}

function closeCouponModal() {
    const modal = document.getElementById('coupon-modal');
    modal.classList.remove('active');
}

function resetCouponModal() {
    const input = document.getElementById('coupon-code');
    if (input) input.value = '';
    setCouponResult('', '');
}

function setCouponResult(message, status) {
    const resultEl = document.getElementById('coupon-result');
    if (!resultEl) return;
    resultEl.classList.remove('success', 'error', 'hidden');
    if (!message) {
        resultEl.textContent = '';
        resultEl.classList.add('hidden');
        return;
    }
    resultEl.textContent = message;
    if (status === 'success') {
        resultEl.classList.add('success');
    } else if (status === 'error') {
        resultEl.classList.add('error');
    }
}

function formatCouponSummary(result) {
    const total = result?.results?.length || 0;
    const applied = typeof result?.applied === 'number' ? result.applied : 0;
    const failed = typeof result?.failed === 'number' ? result.failed : Math.max(0, total - applied);

    let summary = t('coupon.summary')
        .replace('{applied}', applied.toString())
        .replace('{total}', total.toString())
        .replace('{failed}', failed.toString());

    const failures = (result?.results || []).filter(r => !r.success);
    if (failures.length > 0) {
        const rateLimited = failures.filter(r => (r.message || '').toLowerCase().includes('rate limit'));
        const otherFailures = failures.filter(r => !(r.message || '').toLowerCase().includes('rate limit'));
        if (rateLimited.length > 0) {
            summary = `${summary}\n${t('coupon.rateLimitSummary').replace('{count}', rateLimited.length.toString())}`;
        }
        if (otherFailures.length > 0) {
            const maxLines = 8;
            const lines = otherFailures.slice(0, maxLines).map(r => `${r.name}: ${r.message}`);
            summary = `${summary}\n${lines.join('\n')}`;
            if (otherFailures.length > maxLines) {
                summary = `${summary}\n${t('coupon.moreFailures').replace('{count}', (otherFailures.length - maxLines).toString())}`;
            }
        }
    }
    return summary;
}

function startCouponStatusPolling() {
    if (couponStatusInterval) {
        clearInterval(couponStatusInterval);
    }
    couponStatusInterval = setInterval(() => {
        pollCouponStatus(true);
    }, 5000);
    pollCouponStatus(true);
}

function stopCouponStatusPolling() {
    if (couponStatusInterval) {
        clearInterval(couponStatusInterval);
        couponStatusInterval = null;
    }
}

function setCouponProgressVisible(visible) {
    const panel = document.getElementById('coupon-progress');
    if (!panel) return;
    const shouldShow = visible && !couponProgressDismissed;
    panel.classList.toggle('hidden', !shouldShow);
}

async function pollCouponStatus(showToastOnFinish) {
    try {
        const status = await invoke('get_coupon_status');
        if (!status) return;

        if (status.running) {
            isRedeemingCoupon = true;
            setCouponProgressVisible(true);
            return;
        }

        if (!status.summary && !status.error) {
            return;
        }

        stopCouponStatusPolling();
        isRedeemingCoupon = false;
        couponProgressDismissed = false;
        setCouponProgressVisible(false);

        if (status.summary) {
            const summary = formatCouponSummary(status.summary);
            const hasFailures = (status.summary.failed || 0) > 0;
            const toastLevel = hasFailures ? 'warning' : 'success';
            showLog(summary.split('\n')[0], hasFailures ? 'warning' : 'success');
            if (showToastOnFinish) {
                if (hasFailures) {
                    const toastMessage = t('coupon.toastDone')
                        .replace('{applied}', (status.summary.applied || 0).toString())
                        .replace('{total}', (status.summary.results?.length || 0).toString());
                    showToast(toastMessage, toastLevel);
                } else {
                    showToast(t('coupon.toastSuccess'), toastLevel);
                }
            }
            return;
        }

        if (status.error) {
            const message = `${t('coupon.error')} ${status.error}`;
            showLog(message, 'error');
            if (showToastOnFinish) {
                showToast(message, 'error');
            }
            setCouponProgressVisible(false);
        }
    } catch (e) {
        console.error('Failed to poll coupon status:', e);
    }
}

async function resumeCouponStatusPolling() {
    try {
        const status = await invoke('get_coupon_status');
        if (!status) return;

        if (status.running) {
            isRedeemingCoupon = true;
            couponProgressDismissed = false;
            setCouponProgressVisible(true);
            startCouponStatusPolling();
            return;
        }

        if (status.summary) {
            const summary = formatCouponSummary(status.summary);
            const hasFailures = (status.summary.failed || 0) > 0;
            showLog(summary.split('\n')[0], hasFailures ? 'warning' : 'success');
            return;
        }

        if (status.error) {
            showLog(`${t('coupon.error')} ${status.error}`, 'error');
        }
    } catch (e) {
        console.error('Failed to resume coupon status polling:', e);
    }
}

async function redeemCouponForAll() {
    const input = document.getElementById('coupon-code');
    const redeemBtn = document.getElementById('coupon-redeem');
    const code = input ? input.value.trim() : '';

    if (isRedeemingCoupon) {
        showToast(t('coupon.running'), 'warning');
        return;
    }

    if (!code) {
        setCouponResult(t('coupon.empty'), 'error');
        return;
    }

    isRedeemingCoupon = true;
    couponProgressDismissed = false;
    setCouponProgressVisible(true);
    if (redeemBtn) redeemBtn.disabled = true;
    setCouponResult(t('coupon.redeeming'), '');

    try {
        const result = await invoke('redeem_coupon_all', { code });
        if (result?.status === 'running') {
            showToast(t('coupon.running'), 'warning');
        } else {
            showToast(t('coupon.started'), 'success');
        }
        closeCouponModal();
        startCouponStatusPolling();
    } catch (e) {
        const message = `${t('coupon.error')} ${e.message || e}`;
        setCouponResult(message, 'error');
        showLog(message, 'error');
        showToast(message, 'error');
        closeCouponModal();
        isRedeemingCoupon = false;
        couponProgressDismissed = false;
        setCouponProgressVisible(false);
    } finally {
        if (redeemBtn) redeemBtn.disabled = false;
    }
}

// ============================================================================
// Modals
// ============================================================================

let modalsSetup = false;

function setupModals() {
    // Prevent multiple setup calls
    if (modalsSetup) {
        console.warn('setupModals already called, skipping');
        return;
    }
    modalsSetup = true;

    // Global settings modal
    document.getElementById('btn-bot-settings').addEventListener('click', () => {
        document.getElementById('global-settings-modal').classList.add('active');
        loadGlobalSettings();
    });

    document.getElementById('close-global-settings').addEventListener('click', () => {
        document.getElementById('global-settings-modal').classList.remove('active');
    });

    document.getElementById('cancel-global-settings').addEventListener('click', () => {
        document.getElementById('global-settings-modal').classList.remove('active');
    });

    document.getElementById('save-global-settings').addEventListener('click', saveGlobalSettings);

    // Character settings modal
    document.getElementById('close-char-settings').addEventListener('click', () => {
        document.getElementById('character-settings-modal').classList.remove('active');
        setApplyAllMode(false);
    });

    document.getElementById('cancel-char-settings').addEventListener('click', () => {
        document.getElementById('character-settings-modal').classList.remove('active');
        setApplyAllMode(false);
    });

    // Use onclick instead of addEventListener to ensure only one handler
    document.getElementById('save-char-settings').onclick = saveCharacterSettings;

    // Copy settings when source is selected
    const copySelect = document.getElementById('copy-settings-select');
    if (copySelect) copySelect.onchange = copySettingsFromSelect;

    // Close modals on backdrop click
    document.querySelectorAll('.modal').forEach(modal => {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                modal.classList.remove('active');
            }
        });
    });
}

// ============================================================================
// Global Settings
// ============================================================================

async function loadGlobalSettings() {
    try {
        const settings = await invoke('get_global_settings') || {};

        document.getElementById('global-auto-start').checked = settings.globalLaunchOnStart || false;
        document.getElementById('global-sleep-min').value = settings.globalSleepTimesMin || 50;
        document.getElementById('global-sleep-max').value = settings.globalSleepTimesMax || 100;
        document.getElementById('global-dont-relog-seconds').value = settings.doNotRelogCharacterSeconds || 3;

    } catch (e) {
        console.error('Failed to load global settings:', e);
    }
}

async function saveGlobalSettings() {
    try {
        const settings = {
            globalLaunchOnStart: document.getElementById('global-auto-start').checked,
            globalSleepTimesMin: parseInt(document.getElementById('global-sleep-min').value) || 50,
            globalSleepTimesMax: parseInt(document.getElementById('global-sleep-max').value) || 100,
            doNotRelogCharacterSeconds: parseInt(document.getElementById('global-dont-relog-seconds').value) || 3
        };

        await invoke('save_global_settings', { settings });

        document.getElementById('global-settings-modal').classList.remove('active');
        showLog(t('log.settingsSaved'), 'success');

    } catch (e) {
        console.error('Failed to save global settings:', e);
        showLog(t('log.saveError'), 'error');
    }
}

// ============================================================================
// Character Settings
// ============================================================================

function setupSettingsNavigation() {
    const navBtns = document.querySelectorAll('.settings-nav-btn');
    const sections = document.querySelectorAll('.settings-section');

    navBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const sectionId = btn.dataset.section;

            navBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            sections.forEach(s => s.classList.remove('active'));
            document.getElementById(`section-${sectionId}`).classList.add('active');
        });
    });

    // Initialize priority lists
    renderPriorityList('expedition-priority-list', expeditionPriorityList);
    renderPriorityList('dice-priority-list', dicePriorityList);
}

async function openCharacterSettings(charId, charName, options = {}) {
    setApplyAllMode(false);
    state.currentCharacter = { id: charId, name: charName };

    document.getElementById('settings-char-name').textContent = charName;
    document.getElementById('settings-char-id').textContent = `ID: ${charId}`;
    populateCopySettingsOptions(charId, charName);

    // Reset to first section
    document.querySelectorAll('.settings-nav-btn').forEach((btn, i) => {
        btn.classList.toggle('active', i === 0);
    });
    document.querySelectorAll('.settings-section').forEach((section, i) => {
        section.classList.toggle('active', i === 0);
    });

    if (options.useDefaults) {
        state.currentCharacterSettings = {};
        resetCharacterSettingsToDefaults();
        state.currentCharacterSettingsSnapshot = collectCharacterSettings();
    } else {
        // Load settings
        try {
            console.log('Loading settings for:', charName, charId);
            const settings = await invoke('load_character_settings', {
                charactername: charName,
                characterid: charId
            }) || {};

            console.log('Loaded settings:', settings);
            state.currentCharacterSettings = settings;
            populateCharacterSettings(settings);
            state.currentCharacterSettingsSnapshot = collectCharacterSettings();

        } catch (e) {
            console.error('Failed to load character settings:', e);
            state.currentCharacterSettings = {};
            state.currentCharacterSettingsSnapshot = collectCharacterSettings();
        }
    }

    document.getElementById('character-settings-modal').classList.add('active');
}

function setApplyAllMode(enabled) {
    state.applySettingsToAll = enabled;
    const badge = document.getElementById('settings-apply-all');
    const nameEl = document.getElementById('settings-char-name');
    const idEl = document.getElementById('settings-char-id');
    if (badge) {
        badge.style.display = enabled ? 'inline' : 'none';
    }
    if (nameEl) {
        nameEl.style.display = enabled ? 'none' : '';
    }
    if (idEl) {
        idEl.style.display = enabled ? 'none' : '';
    }
}

function resetCharacterSettingsToDefaults() {
    const container = document.getElementById('settings-content');
    if (!container) return;

    container.querySelectorAll('input[type="checkbox"]').forEach(el => {
        el.checked = el.defaultChecked;
    });

    container.querySelectorAll('input[type="number"], input[type="time"], input[type="text"]').forEach(el => {
        el.value = el.defaultValue || '';
    });

    container.querySelectorAll('input[type="radio"]').forEach(el => {
        el.checked = el.defaultChecked;
    });

    expeditionPriorityList = [...DEFAULT_EXPEDITION_PRIORITY_LIST];
    dicePriorityList = [...DEFAULT_DICE_PRIORITY_LIST];
    renderPriorityList('expedition-priority-list', expeditionPriorityList);
    renderPriorityList('dice-priority-list', dicePriorityList);
}

function setSettingsOverlayVisible(visible) {
    const overlay = document.getElementById('character-settings-overlay');
    if (overlay) {
        overlay.classList.toggle('hidden', !visible);
    }
}

function valuesEqual(a, b) {
    if (a === b) return true;
    if (a === null || b === null || a === undefined || b === undefined) return false;
    if (Array.isArray(a) || Array.isArray(b)) {
        return JSON.stringify(a) === JSON.stringify(b);
    }
    if (typeof a === 'object' || typeof b === 'object') {
        return JSON.stringify(a) === JSON.stringify(b);
    }
    return false;
}

function collectChangedSettings(current, baseline) {
    const changed = {};
    Object.keys(current).forEach(key => {
        const baseVal = baseline ? baseline[key] : undefined;
        if (!valuesEqual(current[key], baseVal)) {
            changed[key] = current[key];
        }
    });
    return changed;
}

async function openAllCharacterSettings() {
    if (!state.characters || state.characters.length === 0) {
        showLog(t('charSettings.noCharacters'), 'error');
        return;
    }

    const baseCharacter = state.characters[0];
    await openCharacterSettings(baseCharacter.id, baseCharacter.name, { useDefaults: true });
    setApplyAllMode(true);
}

function populateCopySettingsOptions(currentId, currentName) {
    const select = document.getElementById('copy-settings-select');
    if (!select) return;
    select.innerHTML = `<option value="">${t('charSettings.copyPlaceholder')}</option>`;

    state.characters
        .filter(c => !(c.id === currentId && c.name === currentName))
        .forEach(c => {
            const opt = document.createElement('option');
            opt.value = `${c.id}::${c.name}`;
            opt.textContent = `${c.name} (ID: ${c.id})`;
            select.appendChild(opt);
        });
}

function populateCharacterSettings(settings) {
    // This maps all the setting IDs to their values
    const checkboxes = [
        'settingCharacterActive',
        'tavernPlayExpeditions', 'tavernSkipWithHourglasses', 'tavernPlayCityGuard',
        'tavernPlayDiceGame', 'tavernDiceGameSkipUsingHG',
        'itemsCheckbox', 'itemsInventoryMinGoldSavedIgnoreGemMine', 'itemsDoNotSellEpics',
        'itemsImmediatelyThrowIntoCauldron', 'itemsImmediatelyThrowIntoCauldronExceptEpics',
        'witchEnchantItemWeapon', 'witchEnchantItemHat', 'witchEnchantItemChest',
        'witchEnchantItemGloves', 'witchEnchantItemBoots', 'witchEnchantItemNecklace',
        'witchEnchantItemBelt', 'witchEnchantItemRing', 'witchEnchantItemTalisman',
        'itemsPotionsWingedBuy', 'itemsPotionsStrSmallBuy', 'itemsPotionsStrMediumBuy', 'itemsPotionsStrLargeBuy',
        'itemsPotionsDexSmallBuy', 'itemsPotionsDexMediumBuy', 'itemsPotionsDexLargeBuy',
        'itemsPotionsIntSmallBuy', 'itemsPotionsIntMediumBuy', 'itemsPotionsIntLargeBuy',
        'itemsPotionsConstSmallBuy', 'itemsPotionsConstMediumBuy', 'itemsPotionsConstLargeBuy',
        'itemsPotionsLuckSmallBuy', 'itemsPotionsLuckMediumBuy', 'itemsPotionsLuckLargeBuy',
        'itemsMagicShopBuyHourglasses', 'itemsBrewPotionsUsingFruits', 'itemsEnableEquipmentSwap', 'itemsEquipBeforeSelling',
        'arenaCheckbox', 'arenaStopWhenDone', 'arenaFillScrapbook',
        'quartersOrderAtk', 'quartersSignUpGuildAtks', 'quartersSignUpGuildDef', 'quartersSignUpHydra',
        'quarterFightDungeonPortal', 'quartersCollectMailRewards', 'quartersSpinLuckyWheel',
        'quartersDoPlayHellevator', 'quartersHellevatorClaimReward', 'quartersHellevatorClaimRewardFinal', 'quartersHellevatorJoinRaid',
        'collectWood', 'collectStone', 'collectExp', 'fortessSearchForGems', 'fortressDoAttacks',
        'fortessTrainSoldiers', 'fortessTrainMages', 'fortessTrainArchers', 'fortessUpgradeOurOrder',
        'underworldCollectSouls', 'underworldCollectGold', 'underworldCollectThirst',
        'underworldUpgradeBuildings', 'underworldUpgradeKeeper', 'underworldPerformAttacks', 'underWorldAttackFavouriteOpponent',
        'characterIncreaseStatAttributes', 'enableBuyingMount',
        'petsDoFights', 'petsDoDungeons', 'petsDoFeed',
        'miscCollectCalendar', 'miscCollectCalendarExpOnly', 'miscCollectCalendarMushroomsCalendar',
        'miscCollectDailyRewards', 'miscCollectWeeklyRewards', 'miscCollectFreeMushroom',
        'miscPerformDailyGambling', 'miscPerformDailyBareHand',
        'miscPerformDailyFightWarrior', 'miscPerformDailyFightScout', 'miscPerformDailyFightMage',
        'miscPerformDailyFightAssassin', 'miscPerformDailyFightBattleMage', 'miscPerformDailyFightBerserker',
        'miscPerformDailyFightDruid', 'miscPerformDailyFightDemonHunter', 'miscPerformDailyFightBard',
        'miscPerformDailyFightNecromancer', 'miscPerformDailyFightPaladin',
        'dungeonCheckbox', 'dungeonFightDemonPortal', 'dungeonFightLowestLevel',
        'dungeonSkipIdols', 'dungeonSkipTwister', 'dungeonSkipTower', 'dungeonSkipSandstorm',
        'arenaManagerActive', 'arenaManagerSacrificeAfterToiletCycle',
        'toiletEnableToilet', 'toiletFlushWhenFull', 'toiletSacrificeEpics', 'toiletExcludeEpicWeapons',
        'toiletSacrificeGems', 'toiletSacrificeNormalItems', "miscDonothing"
    ];

    checkboxes.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.checked = settings[id] || false;
    });

    // Number inputs
    const numbers = [
        'tavernDrinkBeerAmount', 'tavernCityGuardTimeToPlay',
        'itemsInventoryMinGoldSaved', 'itemsInventoryMinMushroomsSaved', 'itemsInventorySlotsToBeLeft', 'itemsKeepGemPercent', 'itemsEquipSwapMinBoostPercent',
        'quartersSpinLuckyWithResourcesAmount', 'quartersHellevatorKeyCardsKeep', 'quartersHellevatorJoinRaidFloor',
        'fortressAdditionalSoldierPercent',
        'underworldUpgradeKeeperSoulsToKeep',
        'characterStatDistributionStr', 'characterStatDistributionDex', 'characterStatDistributionInt',
        'characterStatDistributionConst', 'characterStatDistributionLuck',
        'petsToFeedPerDay', 'arenaManagerSacrificeAfterPercent'
    ];

    numbers.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.value = settings[id] ?? '';
    });

    // Time inputs
    const times = [
        'tavernPlayExpeditionFrom', 'tavernPlayCityGuardFrom', 'tavernPlayCityGuardTo',
        'quartersOrderAtkFavouriteEnemiesTimeFirst', 'quartersOrderAtkFavouriteEnemiesTimeSecond',
        'fortressCollectTimeFrom', 'fortressCollectTimeTo',
        'underworldDontCollectGoldFrom', 'underworldDontCollectGoldTo',
        'miscDontCollectCalendarBefore', 'miscDontPerformActionsFrom', 'miscDontPerformActionsTo'
    ];

    times.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.value = settings[id] || '';
    });

    // Text inputs
    const texts = [
        'quartersOrderAtkFavouriteEnemies', 'underworldFavouriteOpponents'
    ];

    texts.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.value = settings[id] || '';
    });

    // Radio buttons
    const radios = [
        { name: 'tavernPlayExpExpedition', value: settings.tavernPlayExpExpedition },
        { name: 'itemsInventoryFullSellOption', value: settings.itemsInventoryFullSellOption },
        { name: 'itemsPotionsWinged', value: settings.itemsPotionsWinged },
        { name: 'itemsPotionsStrSmall', value: settings.itemsPotionsStrSmall },
        { name: 'itemsPotionsStrMedium', value: settings.itemsPotionsStrMedium },
        { name: 'itemsPotionsStrLarge', value: settings.itemsPotionsStrLarge },
        { name: 'itemsPotionsDexSmall', value: settings.itemsPotionsDexSmall },
        { name: 'itemsPotionsDexMedium', value: settings.itemsPotionsDexMedium },
        { name: 'itemsPotionsDexLarge', value: settings.itemsPotionsDexLarge },
        { name: 'itemsPotionsIntSmall', value: settings.itemsPotionsIntSmall },
        { name: 'itemsPotionsIntMedium', value: settings.itemsPotionsIntMedium },
        { name: 'itemsPotionsIntLarge', value: settings.itemsPotionsIntLarge },
        { name: 'itemsPotionsConstSmall', value: settings.itemsPotionsConstSmall },
        { name: 'itemsPotionsConstMedium', value: settings.itemsPotionsConstMedium },
        { name: 'itemsPotionsConstLarge', value: settings.itemsPotionsConstLarge },
        { name: 'itemsPotionsLuckSmall', value: settings.itemsPotionsLuckSmall },
        { name: 'itemsPotionsLuckMedium', value: settings.itemsPotionsLuckMedium },
        { name: 'itemsPotionsLuckLarge', value: settings.itemsPotionsLuckLarge },
        { name: 'itemsGemStrength', value: settings.itemsGemStrength },
        { name: 'itemsGemDex', value: settings.itemsGemDex },
        { name: 'itemsGemInt', value: settings.itemsGemInt },
        { name: 'itemsGemConst', value: settings.itemsGemConst },
        { name: 'itemsGemLuck', value: settings.itemsGemLuck },
        { name: 'itemsGemBlack', value: settings.itemsGemBlack },
        { name: 'itemsGemLegendary', value: settings.itemsGemLegendary },
        { name: 'quartersSpinLuckyWithResources', value: settings.quartersSpinLuckyWithResources },
        { name: 'fortressAttackMode', value: settings.fortressAttackMode },
        { name: 'characterMount', value: settings.characterMount },
        { name: 'petsFeedMode', value: settings.petsFeedMode }
    ];

    radios.forEach(({ name, value }) => {
        if (value) {
            const el = document.querySelector(`input[name="${name}"][value="${value}"]`);
            if (el) el.checked = true;
        }
    });

    // Priority lists
    if (settings.expeditionRewardPrioList) {
        expeditionPriorityList = settings.expeditionRewardPrioList;
        renderPriorityList('expedition-priority-list', expeditionPriorityList);
    }

    if (settings.tavernDiceGameRewardOrder) {
        dicePriorityList = settings.tavernDiceGameRewardOrder;
        renderPriorityList('dice-priority-list', dicePriorityList);
    }
}

function collectCharacterSettings() {
    const settings = {};

    // Checkboxes
    document.querySelectorAll('#settings-content input[type="checkbox"]').forEach(el => {
        settings[el.id] = el.checked;
    });

    // Numbers
    document.querySelectorAll('#settings-content input[type="number"]').forEach(el => {
        if (el.value) settings[el.id] = parseInt(el.value);
    });

    // Times
    document.querySelectorAll('#settings-content input[type="time"]').forEach(el => {
        if (el.value) settings[el.id] = el.value;
    });

    // Texts
    document.querySelectorAll('#settings-content input[type="text"]').forEach(el => {
        if (el.value) settings[el.id] = el.value;
    });

    // Radios
    document.querySelectorAll('#settings-content input[type="radio"]:checked').forEach(el => {
        settings[el.name] = el.value;
    });

    // Priority lists
    settings.expeditionRewardPrioList = expeditionPriorityList;
    settings.tavernDiceGameRewardOrder = dicePriorityList;

    return settings;
}

let isSavingCharacterSettings = false;

async function saveCharacterSettings() {
    // Prevent multiple simultaneous saves
    if (isSavingCharacterSettings) {
        return;
    }

    if (!state.currentCharacter) {
        console.error('No current character selected');
        showLog('Kein Charakter ausgewaehlt', 'error');
        return;
    }

    isSavingCharacterSettings = true;

    // Disable save button during save
    const saveBtn = document.getElementById('save-char-settings');
    if (saveBtn) saveBtn.disabled = true;

    try {
        const settings = collectCharacterSettings();
        let saveResult;
        let applyAllSettings = null;
        if (state.applySettingsToAll) {
            const baseline = state.currentCharacterSettingsSnapshot || {};
            const changedSettings = collectChangedSettings(settings, baseline);
            if (Object.keys(changedSettings).length === 0) {
                showLog(t('charSettings.noChanges'), 'info');
                document.getElementById('character-settings-modal').classList.remove('active');
                setApplyAllMode(false);
                return;
            }
            applyAllSettings = changedSettings;
            setSettingsOverlayVisible(true);
            saveResult = await invoke('save_all_character_settings', { settings: changedSettings });
        } else {
            // Save and get the response which includes the saved settings
            saveResult = await invoke('save_character_settings', {
                charactername: state.currentCharacter.name,
                characterid: state.currentCharacter.id,
                settings: settings
            });
        }

        // Verify using the response from save (no separate load needed)
        if (!state.applySettingsToAll) {
            const savedSettings = saveResult?.settings || {};
            // Check a few key values against what was returned
            if (settings.settingCharacterActive !== savedSettings.settingCharacterActive) {
                console.error('MISMATCH: settingCharacterActive - sent:', settings.settingCharacterActive, 'received:', savedSettings.settingCharacterActive);
            }
        }

        if (state.applySettingsToAll) {
            const appliedSettings = applyAllSettings || {};
            if (appliedSettings.settingCharacterActive !== undefined) {
                state.characters = state.characters.map(c => ({
                    ...c,
                    isActive: appliedSettings.settingCharacterActive
                }));
                renderCharactersTable();
            }
            document.getElementById('character-settings-modal').classList.remove('active');
            showLog(t('charSettings.savedAll'), 'success');
        } else {
            // Update isActive in state.characters to sync table display
            const charIndex = state.characters.findIndex(c =>
                c.name === state.currentCharacter.name && c.id === state.currentCharacter.id
            );
            if (charIndex !== -1) {
                state.characters[charIndex].isActive = settings.settingCharacterActive;
                renderCharactersTable();
            }

            document.getElementById('character-settings-modal').classList.remove('active');
            showLog(t('log.charSettingsSaved').replace('{name}', state.currentCharacter.name), 'success');
        }
        setApplyAllMode(false);

    } catch (e) {
        console.error('Failed to save character settings:', e);
        showLog(t('log.saveError') + ': ' + e.message, 'error');
    } finally {
        isSavingCharacterSettings = false;
        setSettingsOverlayVisible(false);
        // Re-enable save button
        if (saveBtn) saveBtn.disabled = false;
    }
}

// Copy settings from another character into the current target
async function copySettingsFromSelect() {
    if (!state.currentCharacter) return;
    const select = document.getElementById('copy-settings-select');
    if (!select || !select.value) return;

    const [sourceIdStr, sourceName] = select.value.split('::');
    const sourceId = parseInt(sourceIdStr, 10);
    if (!sourceId || !sourceName) return;

    try {
        const sourceSettings = await invoke('load_character_settings', {
            charactername: sourceName,
            characterid: sourceId
        }) || {};

        // Populate form with source settings
        state.currentCharacterSettings = sourceSettings;
        populateCharacterSettings(sourceSettings);
        showLog(`Einstellungen von ${sourceName} geladen. Speichern zum Übernehmen.`, 'info');
    } catch (e) {
        console.error('Failed to copy settings:', e);
        showLog('Kopieren der Einstellungen fehlgeschlagen', 'error');
    }
}

// ============================================================================
// Priority Lists
// ============================================================================

function renderPriorityList(containerId, list) {
    const container = document.getElementById(containerId);
    if (!container) return;

    container.innerHTML = list.map((item, index) => `
        <div class="priority-item" data-index="${index}">
            <span>${item}</span>
            <div class="priority-buttons">
                <button class="priority-btn" onclick="movePriority('${containerId}', ${index}, -1)">&#8593;</button>
                <button class="priority-btn" onclick="movePriority('${containerId}', ${index}, 1)">&#8595;</button>
            </div>
        </div>
    `).join('');
}

window.movePriority = function(containerId, index, direction) {
    const list = containerId === 'expedition-priority-list' ? expeditionPriorityList : dicePriorityList;
    const newIndex = index + direction;

    if (newIndex < 0 || newIndex >= list.length) return;

    const temp = list[index];
    list[index] = list[newIndex];
    list[newIndex] = temp;

    renderPriorityList(containerId, list);
};

// ============================================================================
// Logging
// ============================================================================

function showLog(message, level = 'info') {
    console.log(`[${level.toUpperCase()}] ${message}`);

    const container = document.getElementById('log-container');
    if (!container) return;

    const entry = document.createElement('div');
    entry.className = `log-entry ${level}`;
    entry.innerHTML = `
        <span class="log-time">${new Date().toLocaleTimeString()}</span>
        <span class="log-message">${message}</span>
    `;

    container.insertBefore(entry, container.firstChild);

    // Keep only last 100 entries
    while (container.children.length > 100) {
        container.removeChild(container.lastChild);
    }
}

// Toast helper
function showToast(message, level = 'info') {
    const container = document.getElementById('toast-container');
    if (!container) return;

    const toast = document.createElement('div');
    toast.className = `toast ${level}`;
    toast.textContent = message;
    container.appendChild(toast);

    setTimeout(() => {
        toast.classList.add('show');
    }, 10);

    const removeToast = () => {
        toast.classList.remove('show');
        setTimeout(() => {
            toast.remove();
        }, 250);
    };

    toast.addEventListener('click', removeToast);
    setTimeout(removeToast, 5000);
}

// Clear logs button
document.addEventListener('DOMContentLoaded', () => {
    const clearBtn = document.getElementById('btn-clear-logs');
    if (clearBtn) {
        clearBtn.addEventListener('click', () => {
            const container = document.getElementById('log-container');
            container.innerHTML = '';
            showLog('Logs geloescht', 'info');
        });
    }
});

// ============================================================================
// Expose for debugging
// ============================================================================

window.sfBotState = state;
