use bevy::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_egui::{
    egui::{self},
    EguiContexts, EguiMousePosition,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::{
    ble::BLEHub, ble_switch::BLESwitch, ble_train::BLETrain, block::Block, editor::*,
    marker::Marker, track::Track, train::Train,
};

fn inspector_system_world(world: &mut World) {
    let mut state = SystemState::<(EguiContexts,)>::new(world);
    let (mut egui_contexts,) = state.get_mut(world);
    let inner_response = egui::SidePanel::new(egui::panel::Side::Right, "Inspector").show(
        &egui_contexts.ctx_mut().clone(),
        |ui| {
            ui.label("Inspector");
            {
                ui.separator();
                Train::inspector(ui, world);
                BLETrain::inspector(ui, world);
                BLEHub::inspector(ui, world);
                Block::inspector(ui, world);
                Track::inspector(ui, world);
                Marker::inspector(ui, world);
                BLESwitch::inspector(ui, world);
            };
        },
    );

    let mut state = SystemState::<(Res<EguiMousePosition>, ResMut<InputData>)>::new(world);
    let (egui_mouse_pos, mut input_data) = state.get_mut(world);
    if let Some((_, mouse_pos)) = egui_mouse_pos.0 {
        input_data.mouse_over_ui = inner_response.response.rect.contains(mouse_pos.to_pos2());
    }
}

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inspector_system_world);
        app.add_plugins(DefaultInspectorConfigPlugin);
    }
}
