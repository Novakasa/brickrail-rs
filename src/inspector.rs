use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_egui::{
    egui::{self, Id},
    EguiContexts, EguiMousePosition,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_trait_query::One;

use crate::{editor::*, layout};

#[bevy_trait_query::queryable]
pub trait Inspectable {
    fn inspector_ui(&mut self, ui: &mut egui::Ui, type_registry: &TypeRegistry);
}

fn inspector_system(
    type_registry: Res<AppTypeRegistry>,
    mut contexts: EguiContexts,
    mut q_inspectable: Query<One<&mut dyn Inspectable>>,
    selection_state: Res<SelectionState>,
    layout: Res<layout::Layout>,
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
                if let Some(entity) = layout.get_entity(&generic_id) {
                    let mut inspectable = q_inspectable.get_mut(entity).unwrap();
                    inspectable.inspector_ui(ui, &type_registry.read());
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
