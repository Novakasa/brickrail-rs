use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

use crate::ble::HubConfiguration;

#[derive(Resource, Debug, Serialize, Deserialize)]
pub struct PersistentHubState {
    pub program_hashes: HashMap<String, String>,
    pub configs: HashMap<String, HubConfiguration>,
}

impl Default for PersistentHubState {
    fn default() -> Self {
        Self {
            program_hashes: HashMap::default(),
            configs: HashMap::default(),
        }
    }
}

impl PersistentHubState {
    fn load_from_disk() -> Self {
        // check if hub_state.json exists, otherwise return default
        let settings = std::fs::read_to_string("hub_state.json");
        match settings {
            Ok(state_json) => serde_json::from_str(&state_json).unwrap(),
            Err(_) => PersistentHubState::default(),
        }
    }

    pub fn sync_configured_hub(&mut self, hub_name: &str, config: &HubConfiguration) {
        self.configs.insert(hub_name.to_string(), config.clone());
    }

    pub fn config_matches(&self, hub_name: &str, config: &HubConfiguration) -> bool {
        match self.configs.get(hub_name) {
            Some(stored_config) => stored_config == config,
            None => false,
        }
    }
}

impl Drop for PersistentHubState {
    fn drop(&mut self) {
        // save settings to hub_state.json
        let state_json = serde_json::to_string_pretty(self).unwrap();
        std::fs::write("hub_state.json", state_json).unwrap();
    }
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PersistentHubState::load_from_disk());
    }
}
