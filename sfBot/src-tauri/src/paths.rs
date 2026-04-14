use std::env;
use std::path::PathBuf;



pub fn get_exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}


pub fn exe_relative_path(filename: &str) -> PathBuf {
    get_exe_dir().join(filename)
}


pub fn get_cache_dir() -> PathBuf {
    exe_relative_path("cache")
}


pub fn get_character_settings_path() -> PathBuf {
    exe_relative_path("charactersettings.json")
}


pub fn get_global_settings_path() -> PathBuf {
    exe_relative_path("globalsettings.json")
}


pub fn get_user_config_path() -> PathBuf {
    exe_relative_path("userConfig.json")
}


pub fn get_server_config_path() -> PathBuf {
    exe_relative_path("serverConfig.json")
}
