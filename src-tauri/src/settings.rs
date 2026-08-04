//! Settings persistence: a JSON file in the platform config directory.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::models::ModelId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationMode {
    /// Record while the hotkey is held; stop on release.
    PushToTalk,
    /// Press once to start, press again to stop.
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Global shortcut in the `tauri-plugin-global-shortcut` string format.
    pub hotkey: String,
    pub mode: DictationMode,
    pub model: ModelId,
    /// When false, the transcript is only placed on the clipboard.
    pub auto_paste: bool,
    pub launch_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey().to_string(),
            mode: DictationMode::Toggle,
            model: ModelId::ParakeetTdt06bV2Int8,
            auto_paste: true,
            launch_at_login: false,
        }
    }
}

pub fn default_hotkey() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd+Shift+Space"
    } else {
        "Ctrl+Shift+Space"
    }
}

impl Settings {
    /// Platform config file, e.g. `~/Library/Application Support/com.murmur.app`
    /// on macOS, `~/.config/murmur` on Linux, `%APPDATA%\murmur\config` on
    /// Windows.
    pub fn default_path() -> anyhow::Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "murmur", "murmur")
            .context("no home directory")?;
        Ok(dirs.config_dir().join("settings.json"))
    }

    /// Load settings, falling back to defaults if the file is missing or
    /// unreadable. A corrupt file is never fatal: dictation must keep
    /// working even if the config was mangled.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("settings file corrupt ({e}); using defaults");
                    Settings::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => {
                log::warn!("could not read settings ({e}); using defaults");
                Settings::default()
            }
        }
    }

    /// Save atomically: write a temp file, then rename over the target.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, &text).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, path).with_context(|| format!("renaming into {}", path.display()))?;
        Ok(())
    }
}

/// App-wide settings holder with its backing file path.
pub struct SettingsState {
    pub path: PathBuf,
    pub current: Mutex<Settings>,
}

impl SettingsState {
    pub fn load_or_default() -> Self {
        let path = Settings::default_path().unwrap_or_else(|e| {
            log::error!("cannot resolve config dir ({e}); settings will not persist");
            std::env::temp_dir().join("murmur-settings.json")
        });
        let current = Settings::load(&path);
        log::info!("settings loaded from {}: {current:?}", path.display());
        Self {
            path,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().expect("settings lock").clone()
    }

    pub fn update(&self, new: Settings) -> anyhow::Result<()> {
        new.save(&self.path)?;
        *self.current.lock().expect("settings lock") = new;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("murmur-settings-test");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn roundtrip_preserves_all_fields() {
        let path = tmp_path("roundtrip.json");
        let settings = Settings {
            hotkey: "Alt+Space".into(),
            mode: DictationMode::PushToTalk,
            model: ModelId::WhisperBaseEn,
            auto_paste: false,
            launch_at_login: true,
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn missing_file_gives_defaults() {
        let path = tmp_path("does-not-exist.json");
        let _ = std::fs::remove_file(&path);
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn corrupt_file_gives_defaults() {
        let path = tmp_path("corrupt.json");
        std::fs::write(&path, "{not json at all").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn partial_file_fills_in_defaults() {
        let path = tmp_path("partial.json");
        std::fs::write(&path, r#"{"mode": "push_to_talk"}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.mode, DictationMode::PushToTalk);
        assert_eq!(s.hotkey, default_hotkey());
        assert!(s.auto_paste);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn mode_serializes_to_stable_strings() {
        assert_eq!(
            serde_json::to_string(&DictationMode::PushToTalk).unwrap(),
            "\"push_to_talk\""
        );
        assert_eq!(
            serde_json::to_string(&DictationMode::Toggle).unwrap(),
            "\"toggle\""
        );
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = std::env::temp_dir().join("murmur-settings-test/nested/deeper");
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("murmur-settings-test/nested"));
        let path = dir.join("settings.json");
        Settings::default().save(&path).unwrap();
        assert!(path.is_file());
        std::fs::remove_dir_all(std::env::temp_dir().join("murmur-settings-test/nested")).unwrap();
    }
}
