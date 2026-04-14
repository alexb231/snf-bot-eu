const API_BASE = '/api';







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







async function startBot(accounts) {
    return apiRequest('/bot/start', {
        method: 'POST',
        body: JSON.stringify({ accounts }),
    });
}





async function stopBot() {
    return apiRequest('/bot/stop', { method: 'POST' });
}





async function shutdownServer() {
    return apiRequest('/shutdown', { method: 'POST' });
}





async function getBotStatus() {
    return apiRequest('/bot/status');
}





async function pauseBot() {
    return apiRequest('/bot/pause', { method: 'POST' });
}





async function resumeBot() {
    return apiRequest('/bot/resume', { method: 'POST' });
}






async function getAccounts() {
    return apiRequest('/accounts');
}







async function login(username, password) {
    return apiRequest('/accounts/login', {
        method: 'POST',
        body: JSON.stringify({ username, password }),
    });
}








async function loginSingleAccount(username, password, server) {
    return apiRequest('/accounts/login-single', {
        method: 'POST',
        body: JSON.stringify({ username, password, server }),
    });
}






async function getCharacters() {
    return apiRequest('/characters');
}







async function getCharacterSettings(name, id) {
    return apiRequest(`/characters/settings?name=${encodeURIComponent(name)}&id=${id}`);
}








async function apiSaveCharacterSettings(name, id, settings) {
    return apiRequest('/characters/settings', {
        method: 'POST',
        body: JSON.stringify({ name, id, settings }),
    });
}






async function apiSaveAllCharacterSettings(settings) {
    return apiRequest('/characters/settings-all', {
        method: 'POST',
        body: JSON.stringify({ settings }),
    });
}





async function getAllCharacterSettings() {
    return apiRequest('/characters/all-settings');
}







async function getCharacterLog(name, id) {
    return apiRequest(`/characters/log?name=${encodeURIComponent(name)}&id=${id}`);
}








async function getCharacterExpeditionStats(name, id, server) {
    const serverParam = encodeURIComponent(server || '');
    return apiRequest(`/characters/expedition-stats?name=${encodeURIComponent(name)}&id=${id}&server=${serverParam}`);
}





async function getExpeditionSummary() {
    return apiRequest('/expeditions/summary');
}





async function getCachedCharacters() {
    return apiRequest('/characters/cached');
}






async function redeemCoupon(code) {
    return apiRequest('/coupons/redeem', {
        method: 'POST',
        body: JSON.stringify({ code }),
    });
}





async function getCouponStatus() {
    return apiRequest('/coupons/status');
}






async function getGlobalSettings() {
    return apiRequest('/settings');
}






async function apiSaveGlobalSettings(settings) {
    return apiRequest('/settings', {
        method: 'POST',
        body: JSON.stringify({ settings }),
    });
}






async function getUserConfig() {
    return apiRequest('/config');
}









async function saveUserConfig(accname, password, single, server) {
    return apiRequest('/config', {
        method: 'POST',
        body: JSON.stringify({ accname, password, single, server }),
    });
}







async function getVersion() {
    return apiRequest('/version');
}





async function checkAuth() {
    return apiRequest('/auth/check');
}





async function getHash() {
    return apiRequest('/auth/hash');
}









async function invoke(cmd, args = {}) {
    switch (cmd) {
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

        case 'login':
            return login(args.name, args.pw).then(r => r.characters);
        case 'login_single_account':
            return loginSingleAccount(args.name, args.pw, args.server).then(r => r.characters);
        case 'read_user_conf':
            return getUserConfig().then(r => r.accounts);
        case 'save_user_conf':
            return saveUserConfig(args.accname, args.password, args.single, args.server);

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
        case 'redeem_coupon_all':
            return redeemCoupon(args.code);
        case 'get_coupon_status':
            return getCouponStatus();

        case 'get_global_settings':
            return getGlobalSettings().then(r => r.settings);
        case 'save_global_settings':
            return apiSaveGlobalSettings(args.settings);

        case 'get_app_version':
            return getVersion().then(r => r.version);
        case 'perform_check_whether_user_is_allowed_to_start_bot_impl':
            return checkAuth().then(r => r.allowed);
        case 'generate_hash_impl':
            return getHash().then(r => r.hash);

        case 'startedenbot2':
        case 'singleAccountExecution':
        case 'getFunctionNamesToExecute':
            console.warn(`Command ${cmd} is now handled by the Rust bot runner`);
            return Promise.resolve([]);

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
        redeemCoupon,
        getCouponStatus,
        getGlobalSettings,
        saveGlobalSettings: apiSaveGlobalSettings,
        getUserConfig,
        saveUserConfig,
        getVersion,
        checkAuth,
        getHash,
    };
}
