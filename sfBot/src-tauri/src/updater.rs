#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    env,
    error::Error,
    fs,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use chrono::Local;
use semver::Version;
use serde::{Deserialize, Serialize};

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

pub fn maybe_run_update(current_version: &str) -> Result<bool, Box<dyn Error>>
{
    if let Some(mode) = env::args().nth(1)
    {
        if mode == "--do-update"
        {
            run_updater_flow_from_args()?;
            return Ok(true);
        }
    }

    match check_for_update(current_version)
    {
        Ok(info) =>
        {
            spawn_background_updater(&info)?;
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}

fn check_for_update(current_version: &str) -> Result<UpdateInfo, Box<dyn Error>>
{
    let resp = reqwest::blocking::get("https://downloader.sfbot.eu/updates/latest.json")?;
    if !resp.status().is_success()
    {
        return Err(format!("update check http {}", resp.status()).into());
    }
    let body = resp.text()?;
    let info: UpdateInfo = serde_json::from_str(&body)?;
    let available = Version::parse(&info.version)?;
    let current = Version::parse(current_version)?;
    if available > current
    {
        Ok(info)
    }
    else
    {
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

fn run_updater_flow_from_args() -> Result<(), Box<dyn Error>>
{
    let target_exe = PathBuf::from(env::args().nth(2).ok_or("missing target_exe")?);
    let download_url = env::args().nth(3).ok_or("missing download_url")?;
    let _new_version = env::args().nth(4).ok_or("missing version")?;
    let expected_sha = env::args().nth(5).unwrap_or_default();

    thread::sleep(Duration::from_millis(400));

    let backup = target_exe.with_extension("old");
    wait_and_rename(&target_exe, &backup, Duration::from_secs(30))?;

    let tmp_dl = target_exe.with_extension("download");
    download_to_file(&download_url, &tmp_dl)?;

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

fn download_to_file(url: &str, dest: &Path) -> Result<(), Box<dyn Error>>
{
    let mut resp = reqwest::blocking::get(url)?;
    if !resp.status().is_success()
    {
        return Err(format!("download http {}", resp.status()).into());
    }
    let mut out = File::create(dest)?;
    io::copy(&mut resp, &mut out)?;
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
