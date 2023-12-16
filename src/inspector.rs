use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiContexts};
use bevy_inspector_egui::bevy_inspector::{ui_for_entity, ui_for_value};

use crate::{block::Block, editor::*, layout};

fn inspector_system(world: &mut World) {
    let mut binding = world
        .query::<&mut EguiContext>()
        .get_single_mut(world)
        .unwrap();
    let mut context = binding.get_mut().clone();
    egui::Window::new("Inspector").show(&context, |ui| {
        ui.label("Hello World!");
        let selection_state = world.get_resource::<SelectionState>().unwrap();
        let layout = world.get_resource::<layout::Layout>().unwrap();

        if let Selection::Single(generic_id) = &selection_state.selection {
            if let GenericID::Block(block_id) = generic_id {
                if let Some(entity) = layout.get_entity(generic_id.clone()) {
                    ui_for_entity(world, entity, ui);
                }
            }
        }
    });
}

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inspector_system);
    }
}
