/**
 * SF Bot HTTP API Client
 *
 * This replaces the Tauri invoke() calls with fetch() to the Rust HTTP server.
 * The server runs on http://localhost:3000
 */

const API_BASE = 'http://localhost:3000/api';

/**
 * Make an API request
 * @param {string} endpoint - API endpoint path
 * @param {object} options - fetch options
 * @returns {Promise<any>} - JSON response
 */
async function apiRequest(endpoint, options = {}) {
    const url = `${API_BASE}${endpoint}`;
    const defaultOptions = {
        headers: {
            'Content-Type': 'application/json',
        },
    };

    const response = await fetch(url, { ...defaultOptions, ...options });

    if (!response.ok) {
        const error = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(error.error || `HTTP ${response.status}`);
    }

    return response.json();
}

// ============================================================================
// Bot Control API
// ============================================================================

/**
 * Start the bot with given accounts
 * @param {Array} accounts - Account configurations
 * @returns {Promise<object>}
 */
async function startBot(accounts) {
    return apiRequest('/bot/start', {
        method: 'POST',
        body: JSON.stringify({ accounts }),
    });
}

/**
 * Stop the bot
 * @returns {Promise<object>}
 */
async function stopBot() {
    return apiRequest('/bot/stop', { method: 'POST' });
}

/**
 * Shutdown the server process completely
 * @returns {Promise<object>}
 */
async function shutdownServer() {
    return apiRequest('/shutdown', { method: 'POST' });
}

/**
 * Get current bot status
 * @returns {Promise<object>}
 */
async function getBotStatus() {
    return apiRequest('/bot/status');
}

/**
 * Pause the bot
 * @returns {Promise<object>}
 */
async function pauseBot() {
    return apiRequest('/bot/pause', { method: 'POST' });
}

/**
 * Resume the bot
 * @returns {Promise<object>}
 */
async function resumeBot() {
    return apiRequest('/bot/resume', { method: 'POST' });
}

// ============================================================================
// Account Management API
// ============================================================================

/**
 * Get all saved accounts from config
 * @returns {Promise<object>}
 */
async function getAccounts() {
    return apiRequest('/accounts');
}

/**
 * Login to SF account (SSO, multiple characters)
 * @param {string} username
 * @param {string} password
 * @returns {Promise<object>}
 */
async function login(username, password) {
    return apiRequest('/accounts/login', {
        method: 'POST',
        body: JSON.stringify({ username, password }),
    });
}

/**
 * Login to single server account
 * @param {string} username
 * @param {string} password
 * @param {string} server
 * @returns {Promise<object>}
 */
async function loginSingleAccount(username, password, server) {
    return apiRequest('/accounts/login-single', {
        method: 'POST',
        body: JSON.stringify({ username, password, server }),
    });
}

// ============================================================================
// Character Management API
// ============================================================================

/**
 * Get all logged-in characters
 * @returns {Promise<object>}
 */
async function getCharacters() {
    return apiRequest('/characters');
}

/**
 * Get settings for a specific character
 * @param {string} name - Character name
 * @param {number} id - Character ID
 * @returns {Promise<object>}
 */
async function getCharacterSettings(name, id) {
    return apiRequest(`/characters/settings?name=${encodeURIComponent(name)}&id=${id}`);
}

/**
 * Save settings for a character
 * @param {string} name - Character name
 * @param {number} id - Character ID
 * @param {object} settings - Settings object
 * @returns {Promise<object>}
 */
async function apiSaveCharacterSettings(name, id, settings) {
    return apiRequest('/characters/settings', {
        method: 'POST',
        body: JSON.stringify({ name, id, settings }),
    });
}

/**
 * Save settings for all characters
 * @param {object} settings - Settings object
 * @returns {Promise<object>}
 */
async function apiSaveAllCharacterSettings(settings) {
    return apiRequest('/characters/settings-all', {
        method: 'POST',
        body: JSON.stringify({ settings }),
    });
}

/**
 * Get all character settings
 * @returns {Promise<object>}
 */
async function getAllCharacterSettings() {
    return apiRequest('/characters/all-settings');
}

/**
 * Get character log
 * @param {string} name - Character name
 * @param {number} id - Character ID
 * @returns {Promise<object>}
 */
async function getCharacterLog(name, id) {
    return apiRequest(`/characters/log?name=${encodeURIComponent(name)}&id=${id}`);
}

/**
 * Get expedition stats for a character
 * @param {string} name - Character name
 * @param {number} id - Character ID
 * @param {string} server - Server hostname
 * @returns {Promise<object>}
 */
async function getCharacterExpeditionStats(name, id, server) {
    const serverParam = encodeURIComponent(server || '');
    return apiRequest(`/characters/expedition-stats?name=${encodeURIComponent(name)}&id=${id}&server=${serverParam}`);
}

/**
 * Get aggregated expedition stats across all characters
 * @returns {Promise<object>}
 */
async function getExpeditionSummary() {
    return apiRequest('/expeditions/summary');
}

/**
 * Get cached characters (for display before bot starts)
 * @returns {Promise<object>}
 */
async function getCachedCharacters() {
    return apiRequest('/characters/cached');
}

// ============================================================================
// Global Settings API
// ============================================================================

/**
 * Get global settings
 * @returns {Promise<object>}
 */
async function getGlobalSettings() {
    return apiRequest('/settings');
}

/**
 * Save global settings
 * @param {object} settings - Settings object
 * @returns {Promise<object>}
 */
async function apiSaveGlobalSettings(settings) {
    return apiRequest('/settings', {
        method: 'POST',
        body: JSON.stringify({ settings }),
    });
}

// ============================================================================
// User Config API
// ============================================================================

/**
 * Get user config (accounts)
 * @returns {Promise<object>}
 */
async function getUserConfig() {
    return apiRequest('/config');
}

/**
 * Save user config (add account)
 * @param {string} accname - Account name
 * @param {string} password - Password
 * @param {boolean} single - Is single server account
 * @param {string} server - Server URL (for single accounts)
 * @returns {Promise<object>}
 */
async function saveUserConfig(accname, password, single, server) {
    return apiRequest('/config', {
        method: 'POST',
        body: JSON.stringify({ accname, password, single, server }),
    });
}

// ============================================================================
// Misc API
// ============================================================================

/**
 * Get app version
 * @returns {Promise<object>}
 */
async function getVersion() {
    return apiRequest('/version');
}

/**
 * Check if user is authorized
 * @returns {Promise<object>}
 */
async function checkAuth() {
    return apiRequest('/auth/check');
}

/**
 * Get user's hardware hash
 * @returns {Promise<object>}
 */
async function getHash() {
    return apiRequest('/auth/hash');
}

// ============================================================================
// Compatibility layer for existing code
// ============================================================================

/**
 * Replacement for Tauri's invoke() function
 * Maps old Tauri commands to new HTTP API calls
 * @param {string} cmd - Command name
 * @param {object} args - Arguments
 * @returns {Promise<any>}
 */
async function invoke(cmd, args = {}) {
    switch (cmd) {
        // Bot control
        case 'start_bot':
            return startBot(args.accounts);
        case 'stop_bot':
            return stopBot();
        case 'get_bot_status':
            return getBotStatus();
        case 'pause_bot':
            return pauseBot();
        case 'resume_bot':
            return resumeBot();

        // Account management
        case 'login':
            return login(args.name, args.pw).then(r => r.characters);
        case 'login_single_account':
            return loginSingleAccount(args.name, args.pw, args.server).then(r => r.characters);
        case 'read_user_conf':
            return getUserConfig().then(r => r.accounts);
        case 'save_user_conf':
            return saveUserConfig(args.accname, args.password, args.single, args.server);

        // Character settings
        case 'load_character_settings':
            return getCharacterSettings(args.charactername, args.characterid).then(r => r.settings);
        case 'save_character_settings':
            return apiSaveCharacterSettings(args.charactername, args.characterid, args.settings);
        case 'save_all_character_settings':
            return apiSaveAllCharacterSettings(args.settings);
        case 'load_all_character_settings':
            return getAllCharacterSettings().then(r => r.settings);
        case 'get_character_log':
            return getCharacterLog(args.name, args.id);
        case 'get_character_expedition_stats':
            return getCharacterExpeditionStats(args.name, args.id, args.server);
        case 'get_expedition_summary':
            return getExpeditionSummary();

        // Global settings
        case 'get_global_settings':
            return getGlobalSettings().then(r => r.settings);
        case 'save_global_settings':
            return apiSaveGlobalSettings(args.settings);

        // Misc
        case 'get_app_version':
            return getVersion().then(r => r.version);
        case 'perform_check_whether_user_is_allowed_to_start_bot_impl':
            return checkAuth().then(r => r.allowed);
        case 'generate_hash_impl':
            return getHash().then(r => r.hash);

        // These commands are now handled by the Rust bot runner
        case 'startedenbot2':
        case 'singleAccountExecution':
        case 'getFunctionNamesToExecute':
            console.warn(`Command ${cmd} is now handled by the Rust bot runner`);
            return Promise.resolve([]);

        // Misc commands that may not need HTTP
        case 'init':
            return Promise.resolve();
        case 'clear_current_characters':
            return Promise.resolve();
        case 'get_current_character':
            return getBotStatus().then(r => ({ list: r.current_character ? [r.current_character] : [] }));
        case 'debug_log':
            console.log('[DEBUG]', args.message);
            return Promise.resolve();
        case 'write_name_of_failed_func':
            console.warn('[FAILED FUNC]', args.fnName, args.identifier);
            return Promise.resolve();
        case 'kill':
            window.close();
            return Promise.resolve();

        default:
            console.error(`Unknown command: ${cmd}`);
            throw new Error(`Unknown command: ${cmd}`);
    }
}

// Export for use in modules or make global
if (typeof window !== 'undefined') {
    window.invoke = invoke;
    window.sfBotApi = {
        startBot,
        stopBot,
        shutdownServer,
        getBotStatus,
        pauseBot,
        resumeBot,
        getAccounts,
        login,
        loginSingleAccount,
        getCharacters,
        getCharacterSettings,
        apiSaveCharacterSettings,
        getAllCharacterSettings,
        getCharacterExpeditionStats,
        getExpeditionSummary,
        getCachedCharacters,
        getGlobalSettings,
        saveGlobalSettings: apiSaveGlobalSettings,
        getUserConfig,
        saveUserConfig,
        getVersion,
        checkAuth,
        getHash,
    };
}
