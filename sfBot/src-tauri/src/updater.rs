#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    env,
    error::Error,
    fs,
    fs::File,
    fs::OpenOptions,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use chrono::Local;
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;

#[derive(Serialize, Deserialize, Debug)]
struct PlatformUpdate
{
    url: String,
    signature: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
struct Platforms
{
    #[serde(rename = "windows-x86_64")]
    windows_x86_64: PlatformUpdate,
}
#[derive(Serialize, Deserialize, Debug)]
struct UpdateInfo
{
    version: String,
    notes: String,
    platforms: Platforms,
}

fn build_http_client() -> Result<Client, Box<dyn Error>>
{
    let user_agent = format!("sfbot/{}", env!("CARGO_PKG_VERSION"));
    let client = Client::builder().user_agent(user_agent).timeout(Duration::from_secs(30)).build()?;
    Ok(client)
}

fn truncate_for_log(input: &str, max_len: usize) -> String
{
    let mut cleaned = input.replace('\r', "\\r").replace('\n', "\\n");
    if cleaned.len() > max_len
    {
        cleaned.truncate(max_len);
        cleaned.push_str("...");
    }
    cleaned
}

fn normalize_json_body(body: &str) -> String
{
    let trimmed = body.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if trimmed.starts_with('{') || trimmed.starts_with('[')
    {
        return trimmed.to_string();
    }
    if let Some(idx) = trimmed.find('{').or_else(|| trimmed.find('['))
    {
        return trimmed[idx..].to_string();
    }
    trimmed.to_string()
}

pub async fn maybe_run_update(current_version: &str) -> Result<bool, Box<dyn Error>>
{
    updater_log(&format!("maybe_run_update current_version={}", current_version));
    if let Some(mode) = env::args().nth(1)
    {
        if mode == "--do-update"
        {
            updater_log("running updater flow");
            run_updater_flow_from_args().await?;
            return Ok(true);
        }
    }

    match check_for_update(current_version).await
    {
        Ok(info) =>
        {
            if should_skip_update(&info.version)
            {
                updater_log("skipping update (recent attempt)");
                return Ok(false);
            }
            updater_log(&format!(
                "update available: current={} new={} url={}",
                current_version,
                info.version,
                info.platforms.windows_x86_64.url
            ));
            write_update_state(&info.version);
            spawn_background_updater(&info)?;
            Ok(true)
        }
        Err(e) =>
        {
            updater_log(&format!("update check result: {}", e));
            Ok(false)
        }
    }
}

async fn check_for_update(current_version: &str) -> Result<UpdateInfo, Box<dyn Error>>
{
    let url = format!(
        "https://downloader.sfbot.eu/updates/latest.json?ts={}",
        Local::now().timestamp_millis()
    );
    updater_log(&format!("checking updates from {}", url));
    let client = build_http_client()?;
    let resp = client.get(url).send().await?;
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = resp.text().await?;
    if !status.is_success()
    {
        let snippet = truncate_for_log(&body, 300);
        return Err(format!("update check http {} body={}", status, snippet).into());
    }
    let cleaned_body = normalize_json_body(&body);
    let info: UpdateInfo = match serde_json::from_str(&cleaned_body)
    {
        Ok(info) => info,
        Err(e) =>
        {
            let snippet = truncate_for_log(&body, 300);
            let cleaned_snippet = truncate_for_log(&cleaned_body, 300);
            updater_log(&format!(
                "update check parse error: {} content-type={} body={} cleaned_body={}",
                e, content_type, snippet, cleaned_snippet
            ));
            return Err(format!("update check parse error: {}", e).into());
        }
    };
    let available = Version::parse(info.version.trim_start_matches('v'))?;
    let current = Version::parse(current_version.trim_start_matches('v'))?;
    if available > current
    {
        Ok(info)
    }
    else
    {
        updater_log(&format!(
            "no update available (current={}, available={})",
            current, available
        ));
        Err("no update available".into())
    }
}

fn spawn_background_updater(info: &UpdateInfo) -> Result<(), Box<dyn Error>>
{
    let current_exe = env::current_exe()?;
    let temp_updater = unique_temp_exe("sfbot_updater")?;

    fs::copy(&current_exe, &temp_updater)?;

    let target_exe = current_exe;
    let url = &info.platforms.windows_x86_64.url;
    let sig = info.platforms.windows_x86_64.signature.clone().unwrap_or_default();

    let mut cmd = Command::new(&temp_updater);
    cmd.arg("--do-update").arg(&target_exe).arg(url).arg(&info.version).arg(sig);

    #[cfg(windows)]
    {
        // Detached + no window
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW);
    }

    updater_log(&format!("spawning updater: {}", temp_updater.display()));
    cmd.spawn()?;
    Ok(())
}

fn unique_temp_exe(prefix: &str) -> Result<PathBuf, Box<dyn Error>>
{
    let mut path = env::temp_dir();
    let ts = Local::now().format("%Y%m%d_%H%M%S%3f");
    let file_name = if cfg!(windows) { format!("{prefix}_{ts}.exe") } else { format!("{prefix}_{ts}") };
    path.push(file_name);
    Ok(path)
}

async fn run_updater_flow_from_args() -> Result<(), Box<dyn Error>>
{
    let target_exe = PathBuf::from(env::args().nth(2).ok_or("missing target_exe")?);
    let download_url = env::args().nth(3).ok_or("missing download_url")?;
    let _new_version = env::args().nth(4).ok_or("missing version")?;
    let expected_sha = env::args().nth(5).unwrap_or_default();

    updater_log(&format!(
        "update flow: target_exe={} url={}",
        target_exe.display(),
        download_url
    ));
    thread::sleep(Duration::from_millis(400));

    let backup = target_exe.with_extension("old");
    wait_and_rename(&target_exe, &backup, Duration::from_secs(30))?;

    let tmp_dl = target_exe.with_extension("download");
    let download_url = with_cache_bust(&download_url);
    download_to_file(&download_url, &tmp_dl).await?;

    if !expected_sha.is_empty()
    {
        let got = sha256_file_hex(&tmp_dl)?;
        if got.to_lowercase() != expected_sha.to_lowercase()
        {
            return Err(format!("sha256 mismatch: expected {}, got {}", expected_sha, got).into());
        }
    }

    replace_file(&tmp_dl, &target_exe)?;

    launch_detached(&target_exe)?;

    // schedule_delete_later(&backup);

    clear_update_state();
    updater_log("update flow finished");
    Ok(())
}

/// Löscht die beim Update entstandenen Dateien neben der EXE:
/// - `<deinexe>.old`  (Backup der alten Version)
/// - `<deinexe>.download` (abgebrochener Download)
pub fn cleanup_old_backups()
{
    // Wo liegt unsere EXE?
    let Ok(mut exe_path) = env::current_exe()
    else
    {
        return;
    };

    // Exakt die zu unserer EXE gehörenden Artefakte löschen
    let old = exe_path.with_extension("old");
    let dl = exe_path.with_extension("download");

    if old.exists()
    {
        let _ = fs::remove_file(&old);
    }
    if dl.exists()
    {
        let _ = fs::remove_file(&dl);
    }

    // (Optional) falls du noch mehrere EXEs im selben Ordner hast und
    // aufräumen willst, kannst du hier zusätzlich read_dir() nutzen.
}

/// Versucht bis timeout, `from` -> `to` umzubenennen (nützlich wenn Windows die
/// Datei noch lockt).
fn wait_and_rename(from: &Path, to: &Path, timeout: Duration) -> Result<(), Box<dyn Error>>
{
    let start = std::time::Instant::now();
    loop
    {
        match fs::rename(from, to)
        {
            Ok(_) => return Ok(()),
            Err(e) =>
            {
                if start.elapsed() >= timeout
                {
                    return Err(io::Error::new(io::ErrorKind::Other, format!("rename timeout: {e}")).into());
                }
                thread::sleep(Duration::from_millis(300));
            }
        }
    }
}

async fn download_to_file(url: &str, dest: &Path) -> Result<(), Box<dyn Error>>
{
    let client = build_http_client()?;
    let mut resp = client.get(url).send().await?;
    if !resp.status().is_success()
    {
        return Err(format!("download http {}", resp.status()).into());
    }
    let mut out = TokioFile::create(dest).await?;
    while let Some(chunk) = resp.chunk().await?
    {
        out.write_all(&chunk).await?;
    }
    Ok(())
}

fn sha256_file_hex(path: &Path) -> Result<String, Box<dyn Error>>
{
    use sha2::{Digest, Sha256};

    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop
    {
        let n = file.read(&mut buf)?;
        if n == 0
        {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in hash
    {
        s.push_str(&format!("{:02x}", b));
    }
    Ok(s)
}

fn replace_file(from: &Path, to: &Path) -> Result<(), Box<dyn Error>>
{
    match fs::rename(from, to)
    {
        Ok(_) => Ok(()),
        Err(_) =>
        {
            // Fallback: nach to kopieren, dann tmp löschen
            fs::copy(from, to)?;
            let _ = fs::remove_file(from);
            Ok(())
        }
    }
}

fn launch_detached(target: &Path) -> Result<(), Box<dyn std::error::Error>>
{
    let mut cmd = Command::new(target);
    #[cfg(windows)]
    {
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW);
    }
    cmd.spawn()?; // keine Quotes, kein format!
    Ok(())
}

fn schedule_delete_later(path: &Path)
{
    #[cfg(windows)]
    {
        let mut cmd = Command::new("cmd.exe");
        // Achtung: Pfad quoten
        let p = format!(r#""{}""#, path.display());
        cmd.arg("/C").arg(format!(r#"start "" /B cmd /C "timeout /T 5 /NOBREAK >NUL & del /Q {}""#, p));
        let _ = cmd.spawn();
    }
}

fn with_cache_bust(url: &str) -> String
{
    let ts = Local::now().timestamp_millis();
    if url.contains('?')
    {
        format!("{url}&ts={ts}")
    }
    else
    {
        format!("{url}?ts={ts}")
    }
}

fn update_state_path() -> PathBuf
{
    let mut path = env::temp_dir();
    path.push("sfbot_update_state.txt");
    path
}

fn should_skip_update(version: &str) -> bool
{
    let path = update_state_path();
    let content = match fs::read_to_string(&path)
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let mut parts = content.splitn(2, '|');
    let last_version = parts.next().unwrap_or("");
    let last_ts = parts.next().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    let now = Local::now().timestamp();
    let recent = now.saturating_sub(last_ts) < 300;
    recent && last_version == version
}

fn write_update_state(version: &str)
{
    let path = update_state_path();
    let ts = Local::now().timestamp();
    let _ = fs::write(path, format!("{}|{}", version, ts));
}

fn clear_update_state()
{
    let path = update_state_path();
    let _ = fs::remove_file(path);
}

fn updater_log(message: &str)
{
    let mut path = env::temp_dir();
    path.push("sfbot_updater.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path)
    {
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", ts, message);
    }
}
