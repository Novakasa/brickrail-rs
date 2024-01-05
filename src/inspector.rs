use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Id},
    EguiContexts, EguiMousePosition,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::{editor::*, layout::EntityMap};

fn inspector_system(
    type_registry: Res<AppTypeRegistry>,
    mut contexts: EguiContexts,
    mut q_inspectable: Query<&mut dyn Selectable>,
    selection_state: Res<SelectionState>,
    mut entity_map: ResMut<EntityMap>,
    egui_mouse_pos: Res<EguiMousePosition>,
    mut input_data: ResMut<InputData>,
) {
    let inner_response = egui::SidePanel::new(egui::panel::Side::Right, Id::new("Inspector")).show(
        contexts.ctx_mut(),
        |ui| {
            ui.label("Hello World!");
            // ui_for_value_readonly(&layout.in_markers, ui, &type_registry.read());
            ui.separator();
            let selection = selection_state.selection.clone();
            if let Selection::Single(generic_id) = selection {
                if let Some(entity) = entity_map.get_entity(&generic_id) {
                    let mut inspectable_iter = q_inspectable.get_mut(entity).unwrap();
                    for mut inspectable in inspectable_iter.iter_mut() {
                        if inspectable.get_id() != generic_id {
                            continue;
                        }
                        inspectable.inspector_ui(ui, &type_registry.read(), &mut entity_map);
                    }
                }
            }
        },
    );
    if let Some((_, mouse_pos)) = egui_mouse_pos.0 {
        input_data.mouse_over_ui = inner_response.response.rect.contains(mouse_pos.to_pos2());
    }
}

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inspector_system);
        app.add_plugins(DefaultInspectorConfigPlugin);
    }
}
