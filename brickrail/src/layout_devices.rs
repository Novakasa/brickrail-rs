use crate::{
    ble::BLEHub,
    editor::{SelectionState, SpawnHubEvent},
    layout::EntityMap,
    layout_primitives::*,
};
use bevy::prelude::*;
use bevy_egui::egui::{self, Layout, Ui};
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct LayoutDevice {
    pub id: LayoutDeviceID,
    pub hub_id: Option<HubID>,
    pub port: Option<HubPort>,
}

impl LayoutDevice {
    pub fn from_id(id: LayoutDeviceID) -> Self {
        Self {
            id,
            hub_id: None,
            port: None,
        }
    }

    pub fn ui_label(&self, q_hubs: &Query<&BLEHub>, entity_map: &ResMut<EntityMap>) -> String {
        let hub = if let Some(id) = self.hub_id {
            q_hubs.get(entity_map.hubs[&id]).ok()
        } else {
            None
        };
        format!(
            "{:}: {:}-{:}",
            self.id,
            hub.map(|h| h.name.clone().unwrap_or(h.id.to_string()))
                .unwrap_or("".to_string()),
            self.port.map(|p| p.to_string()).unwrap_or("".to_string()),
        )
    }

    pub fn inspector(
        &mut self,
        ui: &mut Ui,
        hubs: &Query<&BLEHub>,
        spawn_events: &mut EventWriter<SpawnHubEvent>,
        entity_map: &mut ResMut<EntityMap>,
        selection_state: &mut ResMut<SelectionState>,
    ) {
        BLEHub::select_port_ui(
            ui,
            &mut self.hub_id,
            &mut self.port,
            HubType::Layout,
            hubs,
            spawn_events,
            entity_map,
            selection_state,
        )
    }
}

pub trait DeviceComponent: Component {
    type SpawnEvent: SpawnDeviceID;

    fn new_id(entity_map: &mut EntityMap) -> LayoutDeviceID;
}

pub trait SpawnDeviceID: Event {
    fn from_id(id: LayoutDeviceID) -> Self;
}

pub fn select_device_id<T: DeviceComponent>(
    ui: &mut Ui,
    selected_id: &mut Option<LayoutDeviceID>,
    devices: &mut Query<(&mut T, &mut LayoutDevice)>,
    spawn_events: &mut EventWriter<T::SpawnEvent>,
    entity_map: &mut ResMut<EntityMap>,
    hubs: &Query<&BLEHub>,
) {
    ui.push_id("port", |ui| {
        ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
            let selected_dev = if let Some(id) = selected_id {
                devices.get(entity_map.layout_devices[id]).ok()
            } else {
                None
            };
            egui::ComboBox::from_label("")
                .selected_text(format!(
                    "{:}",
                    selected_dev
                        .map(|(_, dev)| dev.ui_label(hubs, entity_map))
                        .unwrap_or("None".to_string())
                ))
                .show_ui(ui, |ui| {
                    ui.selectable_value(selected_id, None, "None");
                    for (_, device) in devices.iter() {
                        ui.selectable_value(
                            selected_id,
                            Some(device.id),
                            format!("{:}", device.ui_label(hubs, entity_map)),
                        );
                    }
                    if ui.button("New").clicked() {
                        let id = T::new_id(entity_map);
                        spawn_events.send(T::SpawnEvent::from_id(id));
                        *selected_id = Some(id);
                    }
                });
        });
    });
}

pub struct LayoutDevicePlugin;

impl Plugin for LayoutDevicePlugin {
    fn build(&self, _app: &mut App) {}
}
