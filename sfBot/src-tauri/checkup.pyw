import requests
import os
import psutil
import time
import subprocess
from packaging import version
from datetime import datetime, timedelta
import threading
import glob
import win32api
import sys

if getattr(sys, 'frozen', False):
    script_dir = os.path.dirname(sys.executable)
else:
    script_dir = os.path.dirname(os.path.abspath(__file__))

os.chdir(script_dir)
sfbot_lock = threading.Lock()

def get_exe_version(file_path):
    try:
        info = win32api.GetFileVersionInfo(file_path, "\\")
        ms = info['FileVersionMS']
        ls = info['FileVersionLS']
        return f"{ms >> 16}.{ms & 0xFFFF}.{ls >> 16}.{ls & 0xFFFF}"
    except Exception as e:
        print(f"Error reading version from {file_path}: {e}")
        return None

def get_local_sfbot_version():
    base_dir = os.getcwd()
    exe_path = os.path.join(base_dir, "sfbot.exe")

    versioned_candidates = glob.glob(os.path.join(base_dir, "sfbot-*.exe"))
    if versioned_candidates:
        print("Found versioned EXE(s): forcing fresh download...")
        return "0.0.0", False, True

    if os.path.exists(exe_path):
        version_str = get_exe_version(exe_path)
        if version_str:
            print(f"Version from sfbot.exe: {version_str}")
            return version_str, True, False

    print("No valid sfbot executable found. Will force update.")
    return "0.0.0", False, True

def fetch_latest_version_info():
    json_url = "https://downloader.sfbot.eu/updates/latest.json"
    response = requests.get(json_url)
    response.raise_for_status()
    return response.json()

def kill_sfbot():
    for proc in psutil.process_iter(['pid', 'name']):
        try:
            if proc.info['name'].lower() == "sfbot.exe":
                print(f"Killing sfbot.exe (PID: {proc.pid})")
                proc.kill()
                proc.wait()
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue

def is_sfbot_running():
    for proc in psutil.process_iter(['name']):
        if proc.info['name'].lower() == "sfbot.exe":
            return True
    return False

def start_sfbot():
    exe_path = os.path.join(os.getcwd(), "sfbot.exe")
    if os.path.exists(exe_path):
        print("Starting sfbot.exe")
        subprocess.Popen([exe_path], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    else:
        print("sfbot.exe not found!")

def delete_old_executables():
    base_path = os.path.join(os.getcwd(), "sfbot.exe")
    if os.path.exists(base_path):
        try:
            os.remove(base_path)
            print("Deleted old sfbot.exe")
        except Exception as e:
            print(f"Could not delete sfbot.exe: {e}")

    versioned_files = glob.glob(os.path.join(os.getcwd(), "sfbot-*.exe"))
    for file_path in versioned_files:
        try:
            os.remove(file_path)
            print(f"Deleted versioned file: {os.path.basename(file_path)}")
        except Exception as e:
            print(f"Could not delete {file_path}: {e}")

def update_if_needed(cur_version):
    latest_data = fetch_latest_version_info()
    latest_version = latest_data.get("version")
    download_url = latest_data["platforms"]["windows-x86_64"]["url"]
    print(f"Latest available version: {latest_version}")

    if version.parse(latest_version) > version.parse(cur_version):
        print("New version available. Updating...")
        if is_sfbot_running():
            kill_sfbot()

        delete_old_executables()

        filename = os.path.basename(download_url)
        download_path = os.path.join(os.getcwd(), filename)

        with requests.get(download_url, stream=True) as r:
            r.raise_for_status()
            with open(download_path, "wb") as f:
                for chunk in r.iter_content(chunk_size=2048):
                    f.write(chunk)

        print(f"Downloaded and updated: {filename}")
        return latest_version
    else:
        print("Already up-to-date.")
        return cur_version

cur_version, exe_exists, force_update = get_local_sfbot_version()
print(f"Current version from local EXE: {cur_version}")
last_restart = datetime.now()

if not exe_exists or force_update:
    delete_old_executables()
    cur_version = update_if_needed(cur_version)

while True:
    now = datetime.now()

    with sfbot_lock:
        if not is_sfbot_running():
            print("sfbot.exe not running. Checking for updates before starting...")
            cur_version, _, _ = get_local_sfbot_version()
            cur_version = update_if_needed(cur_version)
            start_sfbot()

    if now - last_restart >= timedelta(hours=2):
        with sfbot_lock:
            print("Scheduled restart triggered.")
            kill_sfbot()
            cur_version, _, _ = get_local_sfbot_version()
            cur_version = update_if_needed(cur_version)
            start_sfbot()
            last_restart = now

    time.sleep(60)
