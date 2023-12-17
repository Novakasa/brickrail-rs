use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_egui::{egui, EguiContext, EguiContexts};
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_inspector_egui::{
    bevy_inspector::ui_for_entity, reflect_inspector::ui_for_value_readonly,
};

use crate::{block::Block, editor::*, layout};

fn inspector_system(
    type_registry: Res<AppTypeRegistry>,
    mut q_context: Query<&mut EguiContext>,
    mut q_blocks: Query<&mut Block>,
    selection_state: Res<SelectionState>,
    layout: Res<layout::Layout>,
) {
    let mut context = q_context.get_single_mut().unwrap().get_mut().clone();
    egui::Window::new("Inspector").show(&context, |ui| {
        ui.label("Hello World!");
        let selection = selection_state.selection.clone();
        if let Selection::Single(generic_id) = selection {
            if let GenericID::Block(block_id) = generic_id {
                if let Some(entity) = layout.get_entity(generic_id.clone()) {
                    let mut block = q_blocks.get_mut(entity).unwrap();

                    ui_for_value(&mut block.settings, ui, &type_registry.read());
                }
            }
        }
    });
}

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inspector_system);
        app.add_plugins(DefaultInspectorConfigPlugin);
    }
}
