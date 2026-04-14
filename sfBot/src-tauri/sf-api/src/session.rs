use base64::Engine;
use log::{error, trace, warn};
use reqwest::{header::*, Client, Proxy};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::{
    borrow::Borrow,
    env,
    fmt::Debug,
    path::PathBuf,
    str::FromStr,
    time::Duration,
};
use url::Url;

pub use crate::response::*;
use crate::{
    command::Command,
    error::SFError,
    gamestate::{
        character::{Class, Gender, Race},
        GameState,
    },
    misc::{
        sha1_hash, DEFAULT_CRYPTO_ID, DEFAULT_CRYPTO_KEY, DEFAULT_SESSION_ID,
        HASH_CONST,
    },
};


#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Session {
    
    login_data: LoginData,
    
    server_url: url::Url,
    
    
    session_id: String,
    
    player_id: u32,
    login_count: u32,
    crypto_id: String,
    crypto_key: String,
    
    
    
    client: reqwest::Client,
    options: ConnectionOptions,
}


#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PWHash(String);

impl PWHash {
    
    
    #[must_use]
    pub fn new(password: &str) -> Self {
        Self(sha1_hash(&(password.to_string() + HASH_CONST)))
    }
    
    
    #[must_use]
    pub fn from_hash(hash: String) -> Self {
        Self(hash)
    }

    
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }
}

impl Session {
    
    
    
    
    #[must_use]
    pub fn new(
        username: &str,
        password: &str,
        server: ServerConnection,
    ) -> Self {
        Self::new_hashed(username, PWHash::new(password), server)
    }

    
    #[must_use]
    pub fn new_hashed(
        username: &str,
        pw_hash: PWHash,
        server: ServerConnection,
    ) -> Self {
        let ld = LoginData::Basic {
            username: username.to_string(),
            pw_hash,
        };
        Self::new_full(ld, server.client, server.options, server.url)
    }

    fn new_full(
        ld: LoginData,
        client: Client,
        options: ConnectionOptions,
        url: Url,
    ) -> Self {
        Self {
            login_data: ld,
            server_url: url,
            client,
            session_id: DEFAULT_SESSION_ID.to_string(),
            crypto_id: DEFAULT_CRYPTO_ID.to_string(),
            crypto_key: DEFAULT_CRYPTO_KEY.to_string(),
            login_count: 1,
            options,
            player_id: 0,
        }
    }

    
    
    
    fn logout(&mut self) {
        self.crypto_key = DEFAULT_CRYPTO_KEY.to_string();
        self.crypto_id = DEFAULT_CRYPTO_ID.to_string();
        self.login_count = 1;
        self.session_id = DEFAULT_SESSION_ID.to_string();
        self.player_id = 0;
    }

    
    
    #[must_use]
    pub fn server_url(&self) -> &url::Url {
        &self.server_url
    }

    
    
    
    
    #[must_use]
    pub fn has_session_id(&self) -> bool {
        self.session_id.chars().any(|a| a != '0')
    }

    
    
    
    
    
    
    
    pub async fn login(&mut self) -> Result<Response, SFError> {
        self.logout();
        #[allow(deprecated)]
        let login_cmd = match self.login_data.clone() {
            LoginData::Basic { username, pw_hash } => Command::Login {
                username,
                pw_hash: pw_hash.get().to_string(),
                login_count: self.login_count,
            },
            #[cfg(feature = "sso")]
            LoginData::SSO {
                character_id,
                session,
                ..
            } => Command::SSOLogin {
                uuid: session.uuid,
                character_id,
                bearer_token: session.bearer_token,
            },
        };

        self.send_command(&login_cmd).await
    }

    
    
    
    
    
    
    pub async fn register(
        username: &str,
        password: &str,
        server: ServerConnection,
        gender: Gender,
        race: Race,
        class: Class,
    ) -> Result<(Self, Response), SFError> {
        let mut s = Self::new(username, password, server);
        #[allow(deprecated)]
        let resp = s
            .send_command(&Command::Register {
                username: username.to_string(),
                password: password.to_string(),
                gender,
                race,
                class,
            })
            .await?;

        let Some(tracking) = resp.values().get("tracking") else {
            error!("Got no tracking response from server after registering");
            return Err(SFError::ParsingError(
                "register response",
                resp.raw_response().to_string(),
            ));
        };

        if tracking.as_str() != "signup" {
            error!("Got something else than signup response during register");
            return Err(SFError::ParsingError(
                "register tracking response",
                tracking.as_str().to_string(),
            ));
        }

        
        
        let resp = s.login().await?;
        Ok((s, resp))
    }

    
    
    
    
    
    
    
    
    
    
    
    
    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    pub async fn send_command_raw<T: Borrow<Command>>(
        &self,
        command: T,
    ) -> Result<Response, SFError> {
        let command = command.borrow();
        trace!("Sending a {command:?} command");

        let old_cmd = command.request_string()?;
        trace!("Command string: {old_cmd}");

        let (cmd_name, cmd_args) =
            old_cmd.split_once(':').unwrap_or((old_cmd.as_str(), ""));

        let url = format!(
            "{}cmd.php?req={cmd_name}&params={}&sid={}",
            self.server_url,
            base64::engine::general_purpose::URL_SAFE.encode(cmd_args),
            &self.crypto_id,
        );

        trace!("Full request url: {url}");

        
        url::Url::parse(&url).map_err(|_| {
            SFError::InvalidRequest("Could not parse command url")
        })?;

        #[allow(unused_mut)]
        let mut req = self
            .client
            .get(&url)
            .header(REFERER, &self.server_url.to_string());

        #[cfg(feature = "sso")]
        if let LoginData::SSO { session, .. } = &self.login_data {
            req = req.bearer_auth(&session.bearer_token);
        }
        if self.has_session_id() {
            req = req.header(
                HeaderName::from_str("PG-Session").unwrap(),
                HeaderValue::from_str(&self.session_id).map_err(|_| {
                    SFError::InvalidRequest("Invalid session id")
                })?,
            );
        }
        req = req.header(
            HeaderName::from_str("PG-Player").unwrap(),
            HeaderValue::from_str(&self.player_id.to_string())
                .map_err(|_| SFError::InvalidRequest("Invalid player id"))?,
        );

        let resp = req.send().await.map_err(|_| SFError::ConnectionError)?;

        if !resp.status().is_success() {
            return Err(SFError::ConnectionError);
        }

        let response_body =
            resp.text().await.map_err(|_| SFError::ConnectionError)?;

        match response_body {
            body if body.is_empty() => Err(SFError::EmptyResponse),
            body => {
                let resp =
                    Response::parse(body, chrono::Local::now().naive_local())?;
                if let Some(lc) = resp.values().get("serverversion").copied() {
                    let version: u32 = lc.into("server version")?;
                    if version > self.options.expected_server_version {
                        warn!("Untested S&F Server version: {version}");
                        if self.options.error_on_unsupported_version {
                            return Err(SFError::UnsupportedVersion(version));
                        }
                    }
                }
                Ok(resp)
            }
        }
    }

    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    pub async fn send_command<T: Borrow<Command>>(
        &mut self,
        command: T,
    ) -> Result<Response, SFError> {
        let res = self.send_command_raw(command).await?;
        self.update(&res);
        Ok(res)
    }

    
    
    pub fn update(&mut self, res: &Response) {
        let data = res.values();
        if let Some(lc) = data.get("login count") {
            self.login_count = (*lc).into("login count").unwrap_or_default();
        }
        if let Some(lc) = data.get("sessionid") {
            self.session_id.clear();
            self.session_id.push_str(lc.as_str());
        }
        if let Some(player_id) = data
            .get("ownplayersave")
            .and_then(|a| a.as_str().split('/').nth(1))
            .and_then(|a| a.parse::<u32>().ok())
        {
            self.player_id = player_id;
        }
        if let Some(lc) = data.get("cryptoid") {
            self.crypto_id.clear();
            self.crypto_id.push_str(lc.as_str());
        }
    }

    #[cfg(feature = "sso")]
    pub(super) async fn from_sso_char(
        character: crate::sso::SSOCharacter,
        account: std::sync::Arc<tokio::sync::Mutex<crate::sso::SFAccount>>,
        server_lookup: &crate::sso::ServerLookup,
    ) -> Result<Session, SFError> {
        let url = server_lookup.get(character.server_id)?;
        let session = account.lock().await.session.clone();
        let client = account.lock().await.client.clone();
        let options = account.lock().await.options.clone();

        let ld = LoginData::SSO {
            username: character.name,
            character_id: character.id,
            account,
            session,
        };
        Ok(Session::new_full(ld, client, options, url))
    }

    
    #[must_use]
    pub fn username(&self) -> &str {
        match &self.login_data {
            LoginData::Basic { username, .. } => username,
            #[cfg(feature = "sso")]
            LoginData::SSO {
                username: character_name,
                ..
            } => character_name,
        }
    }

    
    
    
    
    
    
    
    
    
    #[cfg(feature = "sso")]
    pub async fn renew_sso_creds(&mut self) -> Result<(), SFError> {
        let LoginData::SSO {
            account, session, ..
        } = &mut self.login_data
        else {
            return Err(SFError::InvalidRequest(
                "Can not renew sso credentials for a non-sso account",
            ));
        };
        let mut account = account.lock().await;

        if &account.session == session {
            account.refresh_login().await?;
        } else {
            *session = account.session.clone();
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::upper_case_acronyms)]
#[non_exhaustive]
enum LoginData {
    Basic {
        username: String,
        pw_hash: PWHash,
    },
    #[cfg(feature = "sso")]
    SSO {
        username: String,
        character_id: String,
        
        
        account: std::sync::Arc<tokio::sync::Mutex<crate::sso::SFAccount>>,
        
        
        
        
        
        
        
        
        
        
        session: crate::sso::AccountSession,
    },
}





#[derive(Debug, Clone)]
pub struct ServerConnection {
    url: url::Url,
    client: Client,
    options: ConnectionOptions,
}

impl ServerConnection {
    
    
    #[must_use]
    pub fn new(server_url: &str) -> Option<ServerConnection> {
        ServerConnection::new_with_options(
            server_url,
            ConnectionOptions::default(),
        )
    }

    
    
    
    #[must_use]
    pub fn new_with_options(
        server_url: &str,
        options: ConnectionOptions,
    ) -> Option<ServerConnection> {
        let url = if server_url.starts_with("http") {
            server_url.parse().ok()?
        } else {
            format!("https://{server_url}").parse().ok()?
        };

        Some(ServerConnection {
            url,
            client: reqwest_client(&options)?,
            options,
        })
    }
}

pub(crate) fn reqwest_client(options: &ConnectionOptions) -> Option<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static(ACCEPT_LANGUAGE.as_str()),
        HeaderValue::from_static("en;q=0.7,en-US;q=0.6"),
    );

    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .tcp_keepalive(Duration::from_secs(30))
        .pool_idle_timeout(Duration::from_secs(60))
        .default_headers(headers);

    if let Some(settings) = &options.proxy {
        let mut proxy = Proxy::https(&settings.url).ok()?;
        if let Some(username) = &settings.username {
            let password = settings.password.as_deref().unwrap_or("");
            proxy = proxy.basic_auth(username, password);
        }
        builder = builder.proxy(proxy);
    }

    let ua = options.user_agent.as_deref().unwrap_or(DEFAULT_USER_AGENT);
    builder = builder.user_agent(ua);
    builder.build().ok()
}


#[derive(Debug, Clone)]
pub struct ConnectionOptions {
    
    pub user_agent: Option<String>,
    
    pub proxy: Option<ProxySettings>,
    
    pub expected_server_version: u32,
    
    
    
    
    pub error_on_unsupported_version: bool,
}

#[derive(Debug, Clone)]
pub struct ProxySettings {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

static DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                                   AppleWebKit/537.36 (KHTML, like Gecko) \
                                   Chrome/115.0.0.0 Safari/537.36";

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self {
            user_agent: Some(DEFAULT_USER_AGENT.to_string()),
            expected_server_version: 2020,
            error_on_unsupported_version: false,
            proxy: None,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GlobalSettings {
    settings: HashMap<String, Value>,
}

fn global_settings_path() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("globalsettings.json")
}

fn get_u64_setting(
    settings: &HashMap<String, Value>,
    key: &str,
    default: u64,
) -> u64 {
    settings
        .get(key)
        .and_then(|v| v.as_u64()) 
        .unwrap_or(default)
}

pub async fn get_global_settings() -> Result<HashMap<String, Value>, String> {
    let file_path = global_settings_path();
    let mut file_content = String::new();
    if let Ok(mut file) = OpenOptions::new().read(true).open(&file_path) {
        file.read_to_string(&mut file_content)
            .map_err(|e| e.to_string())?;

        if !file_content.trim().is_empty() {
            let global_settings: GlobalSettings =
                serde_json::from_str(&file_content)
                    .map_err(|e| e.to_string())?;
            return Ok(global_settings.settings);
        }
    }
    Ok(HashMap::new())
}

#[derive(Debug, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct SimpleSession {
    session: Session,
    gamestate: Option<GameState>,
}

impl SimpleSession {
    async fn short_sleep() {
        let global_map = get_global_settings().await.unwrap_or_default();
        let configured_min = get_u64_setting(&global_map, "globalSleepTimesMin", 50);
        let configured_max = get_u64_setting(&global_map, "globalSleepTimesMax", 100);

        
        let min_wait = configured_min.max(75);
        let max_wait = configured_max.max(250);
        let end_exclusive = if max_wait > min_wait {
            max_wait.saturating_add(1)
        } else {
            min_wait.saturating_add(1)
        };

        tokio::time::sleep(Duration::from_millis(fastrand::u64(
            min_wait..end_exclusive,
        )))
        .await;
    }

    
    
    
    
    pub async fn login(
        username: &str,
        password: &str,
        server_url: &str,
    ) -> Result<Self, SFError> {
        let connection = ServerConnection::new(server_url)
            .ok_or(SFError::ConnectionError)?;
        let mut session = Session::new(username, password, connection);
        let resp = session.login().await?;
        let gs = GameState::new(resp)?;
        Self::short_sleep().await;
        Ok(Self {
            session,
            gamestate: Some(gs),
        })
    }

    
    
    
    
    
    #[cfg(feature = "sso")]
    pub async fn login_sf_account(
        username: &str,
        password: &str,
    ) -> Result<Vec<Self>, SFError> {
        let acc = crate::sso::SFAccount::login(
            username.to_string(),
            password.to_string(),
        )
        .await?;

        Ok(acc
            .characters()
            .await?
            .into_iter()
            .flatten()
            .map(|a| Self {
                session: a,
                gamestate: None,
            })
            .collect())
    }

    
    
    #[must_use]
    pub fn server_url(&self) -> &url::Url {
        self.session.server_url()
    }

    
    #[must_use]
    pub fn username(&self) -> &str {
        self.session.username()
    }

    
    
    
    
    #[must_use]
    pub fn has_session_id(&self) -> bool {
        self.session.has_session_id()
    }

    
    
    #[must_use]
    pub fn game_state(&self) -> Option<&GameState> {
        self.gamestate.as_ref()
    }

    
    
    #[must_use]
    pub fn game_state_mut(&mut self) -> Option<&mut GameState> {
        self.gamestate.as_mut()
    }

    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    pub async fn send_command<T: Borrow<Command>>(
        &mut self,
        cmd: T,
    ) -> Result<&mut GameState, SFError> {
        if self.gamestate.is_none() {
            let resp = self.session.login().await?;
            let gs = GameState::new(resp)?;
            self.gamestate = Some(gs);
            Self::short_sleep().await;
        }

        let resp = match self.session.send_command(cmd).await {
            Ok(resp) => resp,
            Err(err) => {
                self.gamestate = None;
                return Err(err);
            }
        };

        if let Some(gs) = &mut self.gamestate {
            if let Err(e) = gs.update(resp) {
                self.gamestate = None;
                return Err(e);
            }
        }

        Ok(self.gamestate.as_mut().unwrap())
    }
}
