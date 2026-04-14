







let running = false;
let paused = false;
let characters = [];
let accounts = [];
let refreshInterval = null;
const REFRESH_INTERVAL_MS = 10000; 



document.addEventListener('DOMContentLoaded', async () => {
    console.log('SF Bot Frontend starting...');

    await checkServerConnection();

    await loadVersion();

    await checkAuthorization();

    await loadAccounts();

    await loadCachedCharacters();

    startAutoRefresh();

    setupEventHandlers();

    console.log('SF Bot Frontend initialized');
});


async function checkServerConnection() {
    const statusEl = document.getElementById('connection-status');

    try {
        const response = await fetch('/api/version');
        if (response.ok) {
            if (statusEl) {
                statusEl.textContent = 'Connected';
                statusEl.className = 'status-connected';
            }
            return true;
        }
    } catch (e) {
        console.error('Server not reachable:', e);
    }

    if (statusEl) {
        statusEl.textContent = 'Disconnected';
        statusEl.className = 'status-disconnected';
    }

    showLog('Server nicht erreichbar. Bitte starten Sie das Backend.', 'error');
    return false;
}

async function loadVersion() {
    try {
        const result = await invoke('get_app_version');
        const versionEl = document.getElementById('version') || document.getElementById('botVersion');
        if (versionEl) {
            versionEl.textContent = `v${result}`;
        }
    } catch (e) {
        console.error('Failed to load version:', e);
    }
}

async function checkAuthorization() {
    const overlay = document.getElementById('auth-overlay') || document.getElementById('ui-blocker');

    try {
        const allowed = await invoke('perform_check_whether_user_is_allowed_to_start_bot_impl');

        if (overlay) {
            overlay.style.display = 'none';
        }

        if (!allowed) {
            showLog('Autorisierung fehlgeschlagen', 'error');
            return false;
        }

        return true;
    } catch (e) {
        console.error('Auth check failed:', e);
        if (overlay) {
            overlay.style.display = 'none';
        }
        return true;
    }
}


async function loadAccounts() {
    try {
        accounts = await invoke('read_user_conf') || [];
        renderAccountsList();
        showLog(`${accounts.length} Account(s) geladen`, 'info');

        if (accounts.length > 0) {
            showLog('Klicke "Start Bot" um alle Accounts zu starten', 'info');
        }
    } catch (e) {
        console.error('Failed to load accounts:', e);
        showLog('Konnte Accounts nicht laden: ' + e.message, 'error');
    }
}





async function loadCachedCharacters() {
    try {
        const result = await window.sfBotApi.getCachedCharacters();

        if (result.characters && result.characters.length > 0) {
            characters = result.characters.map(c => ({
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
                cached: true,  
                cachedAt: c.cached_at
            }));

            renderCharacterTable();
            showLog(`${characters.length} Character(e) aus Cache geladen`, 'info');
        } else {
            showLog('Keine gecachten Characters gefunden', 'info');
        }
    } catch (e) {
        console.error('Failed to load cached characters:', e);
        
    }
}

function renderAccountsList() {
    const listEl = document.getElementById('accounts-list');
    if (!listEl) return;

    listEl.innerHTML = '';

    if (accounts.length === 0) {
        listEl.innerHTML = '<div class="empty-state">Keine Accounts gespeichert</div>';
        return;
    }

    accounts.forEach(account => {
        const div = document.createElement('div');
        div.className = 'account-item';
        div.innerHTML = `
            <span class="account-name">${account.accname}</span>
            <span class="account-type">${account.single ? 'Single' : 'SSO'}</span>
        `;
        listEl.appendChild(div);
    });
}


function renderCharacterTable() {
    const tbody = document.getElementById('character-list');
    if (!tbody) return;

    tbody.innerHTML = '';

    if (characters.length === 0) {
        tbody.innerHTML = '<tr><td colspan="10" class="empty-state">Keine Charaktere geladen. Bitte einloggen.</td></tr>';
        return;
    }

    characters.forEach(char => {
        const row = document.createElement('tr');
        row.id = `char-row-${char.id}`;
        row.className = char.isActive ? 'active' : 'inactive';

        row.innerHTML = `
            <td><input type="checkbox" ${char.isActive ? 'checked' : ''} data-char-id="${char.id}" class="char-active-toggle"></td>
            <td>${char.name}</td>
            <td>${char.lvl}</td>
            <td>${char.server || '-'}</td>
            <td>${formatNumber(char.gold)}</td>
            <td>${char.mushrooms}</td>
            <td>${char.beers}/11</td>
            <td>${char.fights}/10</td>
            <td>${char.alu}</td>
            <td>
                <span class="status-badge ${char.isActive ? 'active' : 'inactive'}">
                    ${char.isActive ? 'Aktiv' : 'Inaktiv'}
                </span>
            </td>
        `;

        tbody.appendChild(row);
    });

    updateLastRefresh();
}

function formatNumber(num) {
    if (num >= 1000000) {
        return (num / 1000000).toFixed(1) + 'M';
    } else if (num >= 1000) {
        return (num / 1000).toFixed(1) + 'K';
    }
    return num;
}

function updateLastRefresh() {
    const el = document.getElementById('last-update');
    if (el) {
        el.textContent = `Letztes Update: ${new Date().toLocaleTimeString()}`;
    }
}


function startAutoRefresh() {
    if (refreshInterval) {
        clearInterval(refreshInterval);
    }

    refreshInterval = setInterval(async () => {
        if (!running) return;

        try {
            await refreshBotStatus();
        } catch (e) {
            console.error('Refresh failed:', e);
        }
    }, REFRESH_INTERVAL_MS);
}

async function refreshBotStatus() {
    try {
        const status = await window.sfBotApi.getBotStatus();

        running = status.running;
        paused = status.paused;

        updateBotStatusUI();

        if (status.current_character) {
            const cc = status.current_character;
            showCurrentAction(`${cc.name}: ${cc.current_action}`);
        } else if (!running) {
            showCurrentAction('Bot gestoppt');
        }

    } catch (e) {
        console.error('Failed to refresh status:', e);
        running = false;
        paused = false;
        updateBotStatusUI();
    }
}

function showCurrentAction(text) {
    const el = document.getElementById('current-action') || document.getElementById('botState');
    if (el) {
        el.textContent = text;
    }
}

function updateBotStatusUI() {
    const statusEl = document.getElementById('bot-status');
    const startBtn = document.getElementById('btn-start') || document.getElementById('launchBtn');
    const stopBtn = document.getElementById('btn-stop');
    const pauseBtn = document.getElementById('btn-pause') || document.getElementById('pauseBotBtn');

    if (statusEl) {
        const indicator = statusEl.querySelector('.status-indicator') || document.createElement('span');
        const text = statusEl.querySelector('span:last-child') || statusEl;

        if (running) {
            indicator.className = 'status-indicator running';
            text.textContent = paused ? 'Pausiert' : 'Läuft';
        } else {
            indicator.className = 'status-indicator stopped';
            text.textContent = 'Gestoppt';
        }
    }

    if (startBtn) {
        startBtn.disabled = running;
    }
    if (stopBtn) {
        stopBtn.disabled = !running;
    }
    if (pauseBtn) {
        pauseBtn.disabled = !running;
        pauseBtn.textContent = paused ? 'Resume' : 'Pause';
    }
}


async function startBot() {
    try {
        if (accounts.length === 0) {
            showLog('Keine Accounts konfiguriert. Bitte zuerst einloggen.', 'error');
            return;
        }

        showLog('Bot wird gestartet...', 'info');

        const accountsToStart = accounts.map(acc => ({
            accname: acc.accname,
            password: acc.password,
            single: acc.single || false,
            server: acc.server || ''
        }));

        await window.sfBotApi.startBot(accountsToStart);

        running = true;
        paused = false;
        updateBotStatusUI();
        showLog('Bot gestartet - Login läuft im Hintergrund', 'success');

        startAutoRefresh();

    } catch (e) {
        console.error('Failed to start bot:', e);
        showLog('Bot konnte nicht gestartet werden: ' + e.message, 'error');
    }
}

async function stopBot() {
    try {
        showLog('Bot wird gestoppt...', 'info');

        await window.sfBotApi.stopBot();

        running = false;
        paused = false;
        updateBotStatusUI();
        showLog('Bot gestoppt', 'success');

    } catch (e) {
        console.error('Failed to stop bot:', e);
        showLog('Bot konnte nicht gestoppt werden: ' + e.message, 'error');
    }
}

async function togglePause() {
    try {
        if (paused) {
            await window.sfBotApi.resumeBot();
            paused = false;
            showLog('Bot fortgesetzt', 'info');
        } else {
            await window.sfBotApi.pauseBot();
            paused = true;
            showLog('Bot pausiert', 'info');
        }
        updateBotStatusUI();
    } catch (e) {
        console.error('Failed to toggle pause:', e);
    }
}


async function handleLogin(event) {
    event.preventDefault();

    const username = document.getElementById('username')?.value ||
                     document.getElementById('loginAccountName')?.value;
    const password = document.getElementById('password')?.value ||
                     document.getElementById('loginPassword')?.value;
    const isSingle = document.getElementById('single-server')?.checked ||
                     document.getElementById('loginIsSingleAccount')?.checked;
    const server = document.getElementById('server')?.value ||
                   document.getElementById('loginSingleAccountServer')?.value || '';

    if (!username || !password) {
        showLog('Bitte Username und Passwort eingeben', 'error');
        return;
    }

    try {
        showLog(`Logging in ${username}...`, 'info');

        let chars;
        if (isSingle) {
            if (!server) {
                showLog('Bitte Server-URL eingeben', 'error');
                return;
            }
            chars = await invoke('login_single_account', {
                name: username,
                pw: password,
                single: true,
                server: server
            });
        } else {
            chars = await invoke('login', {
                name: username,
                pw: password
            });
        }

        if (chars && chars.length > 0) {
            const newAccount = { accname: username, password, single: isSingle, server };
            accounts.push(newAccount);

            characters = [...characters, ...chars.map(c => ({...c, account: username}))];

            renderAccountsList();
            renderCharacterTable();
            showLog(`${username}: ${chars.length} Charakter(e) geladen`, 'success');

            
            if (document.getElementById('username')) document.getElementById('username').value = '';
            if (document.getElementById('password')) document.getElementById('password').value = '';
            if (document.getElementById('loginAccountName')) document.getElementById('loginAccountName').value = '';
            if (document.getElementById('loginPassword')) document.getElementById('loginPassword').value = '';
        }

    } catch (e) {
        console.error('Login failed:', e);
        showLog(`Login fehlgeschlagen: ${e.message}`, 'error');
    }
}


function setupEventHandlers() {
    const loginForm = document.getElementById('login-form');
    if (loginForm) {
        loginForm.addEventListener('submit', handleLogin);
    }

    const loginBtn = document.getElementById('loginButton');
    if (loginBtn) {
        loginBtn.addEventListener('click', handleLogin);
    }

    const singleCheckbox = document.getElementById('single-server') || document.getElementById('loginIsSingleAccount');
    const serverGroup = document.getElementById('server-group');
    const serverInput = document.getElementById('loginSingleAccountServer');
    const serverLabel = document.getElementById('loginSingleAccountServerLabel');

    if (singleCheckbox) {
        singleCheckbox.addEventListener('change', () => {
            if (serverGroup) {
                serverGroup.style.display = singleCheckbox.checked ? 'block' : 'none';
            }
            if (serverInput) {
                serverInput.style.display = singleCheckbox.checked ? 'block' : 'none';
            }
            if (serverLabel) {
                serverLabel.style.display = singleCheckbox.checked ? 'block' : 'none';
            }
        });
    }

    const startBtn = document.getElementById('btn-start');
    const stopBtn = document.getElementById('btn-stop');
    const pauseBtn = document.getElementById('btn-pause');
    const refreshBtn = document.getElementById('btn-refresh');

    if (startBtn) startBtn.addEventListener('click', startBot);
    if (stopBtn) stopBtn.addEventListener('click', stopBot);
    if (pauseBtn) pauseBtn.addEventListener('click', togglePause);
    if (refreshBtn) refreshBtn.addEventListener('click', () => {
        renderCharacterTable();
        refreshBotStatus();
    });

    const launchBtn = document.getElementById('launchBtn');
    const pauseBotBtn = document.getElementById('pauseBotBtn');

    if (launchBtn) {
        launchBtn.addEventListener('click', () => {
            if (running) {
                stopBot();
            } else {
                startBot();
            }
        });
    }

    if (pauseBotBtn) {
        pauseBotBtn.addEventListener('click', togglePause);
    }

    const clearLogBtn = document.getElementById('btn-clear-log');
    if (clearLogBtn) {
        clearLogBtn.addEventListener('click', clearLog);
    }

    document.addEventListener('change', (e) => {
        if (e.target.classList.contains('char-active-toggle')) {
            const charId = parseInt(e.target.dataset.charId);
            const char = characters.find(c => c.id === charId);
            if (char) {
                char.isActive = e.target.checked;
                saveCharacterActive(char);
            }
        }
    });
}

async function saveCharacterActive(char) {
    try {
        const settings = await invoke('load_character_settings', {
            charactername: char.name,
            characterid: char.id
        }) || {};

        settings.settingCharacterActive = char.isActive;

        await invoke('save_character_settings', {
            charactername: char.name,
            characterid: char.id,
            settings: settings
        });

        showLog(`${char.name}: ${char.isActive ? 'Aktiviert' : 'Deaktiviert'}`, 'info');
    } catch (e) {
        console.error('Failed to save character active state:', e);
    }
}


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

    while (container.children.length > 100) {
        container.removeChild(container.lastChild);
    }
}

function clearLog() {
    const container = document.getElementById('log-container');
    if (container) {
        container.innerHTML = '';
        showLog('Log cleared', 'info');
    }
}

window.openSettingsPage = async function(characterName, characterId) {
    try {
        const settings = await invoke('load_character_settings', {
            charactername: characterName,
            characterid: characterId
        });

        const settingsWindow = document.getElementById('settingsWindow');
        if (settingsWindow) {
            settingsWindow.style.display = 'block';
            document.getElementById('settingCharacterName').textContent = characterName;
            document.getElementById('settingCharacterId').textContent = `ID: ${characterId}`;

        }
    } catch (e) {
        console.error('Failed to load settings:', e);
        showLog(`Settings konnten nicht geladen werden: ${e.message}`, 'error');
    }
};


window.sfBotState = {
    get running() { return running; },
    get paused() { return paused; },
    get characters() { return characters; },
    get accounts() { return accounts; }
};
