use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::bevy_egui;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::editor::*;
use crate::layout::EntityMap;
use crate::selectable::Selectable;

fn name_editor(ui: &mut egui::Ui, world: &mut World) {
    let mut state =
        SystemState::<(Query<&mut Name>, Res<SelectionState>, Res<EntityMap>)>::new(world);
    let (mut names, selection_state, entity_map) = state.get_mut(world);
    if let Some(entity) = selection_state.get_entity(&entity_map) {
        let id = if let Selection::Single(id) = selection_state.selection {
            id
        } else {
            return;
        };
        if let Ok(mut name) = names.get_mut(entity) {
            ui.label("Name:");
            if id.editable_name() {
                let mut name_edit = name.to_string();
                ui.text_edit_singleline(&mut name_edit);
                name.set(name_edit);
            } else {
                ui.label(name.to_string());
            }
        }
    }
    state.apply(world);
}

pub fn inspector_system_world<T: Selectable>(world: &mut World) {
    let mut state = SystemState::<(EguiContexts,)>::new(world);
    let (mut egui_contexts,) = state.get_mut(world);
    if let Some(ctx) = &egui_contexts.try_ctx_mut().cloned() {
        egui::SidePanel::new(egui::panel::Side::Right, "Inspector").show(ctx, |ui| {
            ui.heading("Inspector");
            {
                name_editor(ui, world);
                ui.separator();
                T::inspector(ui, world);
            };
            ui.set_min_width(200.0);
        });
        state.apply(world);

        let mut state = SystemState::<ResMut<InputData>>::new(world);
        let mut input_data = state.get_mut(world);
        input_data.mouse_over_ui = ctx.wants_pointer_input() || ctx.is_pointer_over_area();
    }
}

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultInspectorConfigPlugin);
    }
}
