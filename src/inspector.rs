use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::reflect_inspector::ui_for_value;

use crate::editor::*;

fn inspector_system(
    mut egui_context: EguiContexts,
    mut selection_state: ResMut<SelectionState>,
    type_registry: Res<AppTypeRegistry>,
) {
    egui::Window::new("Inspector").show(&egui_context.ctx_mut(), |ui| {
        ui.label("Hello World!");
        ui_for_value(
            &mut selection_state.selection,
            ui,
            &type_registry.0.clone().read(),
        )
    });
}

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inspector_system);
    }
}
