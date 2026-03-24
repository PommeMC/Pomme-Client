use crate::storage;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub language: String,
    pub keep_launcher_open: bool,
    pub launch_with_console: bool,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        LauncherSettings {
            language: "French".into(),
            keep_launcher_open: false,
            launch_with_console: true,
        }
    }
}

impl LauncherSettings {
    pub fn save(&self) -> Result<(), String> {
        let path = storage::settings_file();
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load() -> Self {
        let path = storage::settings_file();

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<LauncherSettings>(&content) {
                Ok(cfg) => return cfg,
                Err(err) => {
                    log::warn!("Settings file invalid ({}), using defaults", err);
                }
            },
            Err(err) => {
                log::info!(
                    "Settings file not found or unreadable ({}), using defaults",
                    err
                );
            }
        }

        LauncherSettings::default()
    }

    pub fn set_language(&mut self, language: String) -> Result<(), String> {
        self.language = language;
        self.save()
    }

    pub fn set_keep_launcher_open(&mut self, keep: bool) -> Result<(), String> {
        self.keep_launcher_open = keep;
        self.save()
    }

    pub fn set_launch_with_console(&mut self, launch_with_console: bool) -> Result<(), String> {
        self.launch_with_console = launch_with_console;
        self.save()
    }
}
