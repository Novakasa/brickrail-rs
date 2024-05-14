use std::path::PathBuf;

use bevy::{prelude::*, utils::HashMap};

use crate::layout_primitives::HubID;

#[derive(Resource, Debug)]
pub struct Settings {}

impl Default for Settings {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Component, Debug)]
pub struct LayoutCache {
    pub path: PathBuf,
    pub hub_programs: HashMap<HubID, Option<String>>,
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Settings::default());
    }
}
