use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub program_hashes: HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            program_hashes: HashMap::default(),
        }
    }
}

impl Settings {
    fn new() -> Self {
        // check if settings.json exists, otherwise return default
        let settings = std::fs::read_to_string("settings.json");
        match settings {
            Ok(settings) => {
                let settings: Settings = serde_json::from_str(&settings).unwrap();
                settings
            }
            Err(_) => Settings::default(),
        }
    }
}

impl Drop for Settings {
    fn drop(&mut self) {
        // save settings to settings.json
        let settings = serde_json::to_string_pretty(self).unwrap();
        std::fs::write("settings.json", settings).unwrap();
    }
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Settings::new());
    }
}
