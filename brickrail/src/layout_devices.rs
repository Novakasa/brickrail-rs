use crate::{layout::EntityMap, layout_primitives::*};
use bevy::prelude::*;
use bevy_egui::egui::{self, Layout, Ui};
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct LayoutDevice {
    id: LayoutDeviceID,
}

#[derive(Debug, Event)]
pub struct SpawnLayoutDeviceEvent(LayoutDevice);

pub fn select_device_id(
    ui: &mut Ui,
    selected_id: &mut Option<LayoutDeviceID>,
    kind: LayoutDeviceType,
    devices: &Query<&mut LayoutDevice>,
    spawn_events: &mut EventWriter<SpawnLayoutDeviceEvent>,
    entity_map: &mut ResMut<EntityMap>,
) {
    ui.push_id("port", |ui| {
        ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
            egui::ComboBox::from_label("")
                .selected_text(format!("{:?}", selected_id))
                .show_ui(ui, |ui| {
                    ui.selectable_value(selected_id, None, "None");
                    for option in devices.iter().filter(|d| d.id.kind == kind) {
                        ui.selectable_value(
                            selected_id,
                            Some(option.id),
                            format!("{:?}", option.id),
                        );
                    }
                    if ui.button("New").clicked() {
                        let id = entity_map.new_layout_device_id(LayoutDeviceType::Switch);
                        spawn_events.send(SpawnLayoutDeviceEvent(LayoutDevice { id }));
                        *selected_id = Some(id);
                    }
                });
        });
    });
}
