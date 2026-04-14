#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] 
#![allow(warnings)]
use std::{error::Error, fs, fs::OpenOptions, panic, process::Command};

use chrono::Local;
use reqwest::blocking::get;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{io::Write, os::windows::process::CommandExt};
use std::{thread, time::Duration};

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

fn main()
{
    set_panic_hook();

    #[cfg(not(target_os = "windows"))]
    {
        use std::{fs::File, io};
        let _ = File::create("/dev/null");
        io::set_output(File::create("/dev/null").unwrap());
    }

    let current_version = "0.1.5";

    match check_for_update(current_version)
    {
        Ok(update_info) =>
            {
                println!("Found new version: {}", update_info.version);
                println!("Downloading from: {}", update_info.platforms.windows_x86_64.url);

                let new_file_name = format!("sfbot-{}.exe", update_info.version);

                if let Err(e) = download_update(&update_info.platforms.windows_x86_64.url, &new_file_name)
                {
                    eprintln!("Error during download: {}", e);
                }
                else
                {
                    println!("Downloaded: {}", new_file_name);

                    if let Err(e) = open_new_cmd_to_update(&new_file_name)
                    {
                        eprintln!("Error opening new command prompt: {}", e);
                    }

                    if let Err(e) = stop_old_bot()
                    {
                        eprintln!("Error stopping old bot: {}", e);
                    }

                    return;
                }
            }
        Err(e) =>
            {
                println!("No update: {}", e);
            }
    }
    cleanup_old_versions(current_version).unwrap_or_else(|e| eprintln!("Cleanup failed: {}", e));
    println!("Running current version.");
    sfbot_lib::run();
}

fn set_panic_hook()
{
    panic::set_hook(Box::new(move |panic_info| {
        let panic_message = format!("{:?}", panic_info);
        log_panic_to_file(&panic_message);
    }));
}

fn stop_old_bot() -> Result<(), Box<dyn Error>>
{
    println!("Stopping old bot process...");

    
    let status = Command::new("taskkill").arg("/IM").arg("sfbot.exe").arg("/F").spawn().and_then(|mut child| child.wait())?;

    if !status.success()
    {
        return Err("Failed to stop the old bot.".into());
    }

    println!("Old bot stopped.");
    Ok(())
}

fn open_new_cmd_to_update(new_exe: &str) -> Result<(), Box<dyn std::error::Error>>
{
    println!("Opening hidden Command Prompt to run new version...");

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let status = Command::new("cmd.exe").arg("/C").arg(format!("start {} && exit", new_exe)).creation_flags(CREATE_NO_WINDOW).spawn().and_then(|mut child| child.wait())?;

    if !status.success()
    {
        return Err("Failed to start new bot.".into());
    }

    Ok(())
}
fn cleanup_old_versions(current_version: &str) -> Result<(), Box<dyn std::error::Error>>
{


    thread::sleep(Duration::from_secs(5));

    let current_semver = Version::parse(current_version)?;

    for entry in fs::read_dir(".")?
    {
        let entry = entry?;
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
        {
            let is_generic = file_name == "sfbot.exe";
            let is_versioned = file_name.starts_with("sfbot-") && file_name.ends_with(".exe");

            if is_generic || is_versioned
            {
                if let Ok(current_exe) = std::env::current_exe()
                {
                    if let Some(current_name) = current_exe.file_name().and_then(|n| n.to_str())
                    {
                        if file_name == current_name
                        {
                            continue;
                        }
                    }
                }

                
                if is_generic
                {
                    println!("Deleting legacy file: {}", file_name);
                    fs::remove_file(&path)?;
                }
                else if let Some(version_str) = file_name.strip_prefix("sfbot-").and_then(|s| s.strip_suffix(".exe"))
                {
                    if let Ok(file_version) = Version::parse(version_str)
                    {
                        if file_version < current_semver
                        {
                            println!("Deleting old version {}: {}", version_str, file_name);
                            fs::remove_file(&path)?;
                        }
                        else
                        {
                            println!("Keeping newer or same version: {}", file_name);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_for_update(current_version: &str) -> Result<UpdateInfo, Box<dyn Error>>
{
    let response = get("https://downloader.sfbot.eu/updates/latest.json")?;
    if response.status().is_success()
    {
        let body = response.text()?;
        let update_info: UpdateInfo = serde_json::from_str(&body)?;

        let available_version = Version::parse(&update_info.version)?;
        let current_version = Version::parse(current_version)?;

        if available_version > current_version
        {
            Ok(update_info)
        }
        else
        {
            Err("No update available".into())
        }
    }
    else
    {
        Err(format!("Error checking updates {}", response.status()).into())
    }
}


fn log_panic_to_file(info: &str)
{
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let file_name = format!("panic_log_{}.txt", timestamp);

    let mut file = OpenOptions::new().create_new(true).write(true).open(file_name).unwrap();
    if let Err(e) = writeln!(file, "Panic occurred at {}: {}", timestamp, info)
    {
        eprintln!("Failed to write to panic log file: {}", e);
    }
}

fn download_update(url: &str, output_file: &str) -> Result<(), Box<dyn Error>>
{
    let mut response = get(url)?;
    let mut file = std::fs::File::create(output_file)?;
    std::io::copy(&mut response, &mut file)?;
    println!("Update downloaded: {}", output_file);
    Ok(())
}