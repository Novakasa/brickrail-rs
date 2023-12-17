use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiMousePosition};
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::{block::Block, editor::*, layout};

fn inspector_system(
    type_registry: Res<AppTypeRegistry>,
    mut q_context: Query<&mut EguiContext>,
    mut q_blocks: Query<&mut Block>,
    selection_state: Res<SelectionState>,
    layout: Res<layout::Layout>,
    egui_mouse_pos: Res<EguiMousePosition>,
    mut input_data: ResMut<InputData>,
) {
    let context = q_context.get_single_mut().unwrap().get_mut().clone();
    let response = egui::Window::new("Inspector").show(&context, |ui| {
        ui.label("Hello World!");
        let selection = selection_state.selection.clone();
        if let Selection::Single(generic_id) = selection {
            if let GenericID::Block(_) = generic_id {
                if let Some(entity) = layout.get_entity(generic_id.clone()) {
                    let mut block = q_blocks.get_mut(entity).unwrap();

                    ui_for_value(&mut block.settings, ui, &type_registry.read());
                }
            }
        }
    });
    if let Some(inner) = response {
        if let Some((_, mouse_pos)) = egui_mouse_pos.0 {
            input_data.mouse_over_ui = inner.response.rect.contains(mouse_pos.to_pos2());
        }
    }
}

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inspector_system);
        app.add_plugins(DefaultInspectorConfigPlugin);
    }
}
