use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEntry {
    pub name: String,
    pub regex: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub watched_folder: PathBuf,
    pub patterns: Vec<PatternEntry>,
    pub extensions: Vec<String>,
    /// true = overwrite existing Finder comment; false = append/preserve.
    pub overwrite: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            watched_folder: dirs::download_dir().unwrap_or_else(|| PathBuf::from(".")),
            patterns: vec![
                PatternEntry {
                    name: "Episode (SxxExx)".to_string(),
                    regex: r"(?i)\bS\d{1,2}E\d{1,3}\b".to_string(),
                    enabled: true,
                },
                PatternEntry {
                    name: "Tome/Volume".to_string(),
                    regex: r"\b(?:(?i:tome|volume)\.?\s*\d{1,3}|T\.?\s?\d{1,3}|Vol\.?\s?\d{1,3})\b"
                        .to_string(),
                    enabled: true,
                },
            ],
            extensions: vec![
                "mkv", "mp4", "avi", "mov", "m4v", "cbz", "cbr", "pdf", "epub",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            overwrite: true,
        }
    }
}

impl Config {
    fn config_path() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("Spine");
        std::fs::create_dir_all(&dir).ok();
        dir.push("config.json");
        dir
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
