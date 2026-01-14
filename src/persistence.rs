use directories::ProjectDirs;
use std::{fs, path::PathBuf};

pub fn get_config_path() -> PathBuf {
    let proj_dirs = ProjectDirs::from("org", "jlodenius", "stockfin")
        .expect("Could not determine config directory");

    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir).ok();
    config_dir.join("tickers.json")
}

pub fn save_tickers(tickers: Vec<(String, String)>) {
    let path = get_config_path();
    if let Ok(json) = serde_json::to_string(&tickers) {
        fs::write(path, json).ok();
    }
}

pub fn load_tickers() -> Vec<(String, String)> {
    let path = get_config_path();

    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str::<Vec<(String, String)>>(&data).ok())
        .unwrap_or_default()
}
