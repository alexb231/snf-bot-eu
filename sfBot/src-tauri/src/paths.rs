use std::env;
use std::path::PathBuf;

/// Get the directory where the executable is located
/// This ensures files are always relative to the EXE, not the current working directory
pub fn get_exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Get a path relative to the executable directory
pub fn exe_relative_path(filename: &str) -> PathBuf {
    get_exe_dir().join(filename)
}

/// Get the cache directory path (relative to EXE)
pub fn get_cache_dir() -> PathBuf {
    exe_relative_path("cache")
}

/// Get path for charactersettings.json
pub fn get_character_settings_path() -> PathBuf {
    exe_relative_path("charactersettings.json")
}

/// Get path for globalsettings.json
pub fn get_global_settings_path() -> PathBuf {
    exe_relative_path("globalsettings.json")
}

/// Get path for userConfig.json
pub fn get_user_config_path() -> PathBuf {
    exe_relative_path("userConfig.json")
}
