use bevy::{ecs::system::CommandQueue, prelude::*, reflect::TypeRegistry, window::PrimaryWindow};
use bevy_egui::{
    egui::{self, Id},
    EguiContext, EguiContexts, EguiMousePosition,
};
use bevy_inspector_egui::{
    reflect_inspector::{Context, InspectorUi},
    DefaultInspectorConfigPlugin,
};
use bevy_trait_query::One;

use crate::{editor::*, layout};

fn inspector_system(
    type_registry: Res<AppTypeRegistry>,
    mut contexts: EguiContexts,
    mut q_inspectable: Query<One<&mut dyn Selectable>>,
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

fn inspector_system_world(world: &mut World) {
    let selection_state = world.resource::<SelectionState>();
    let selection = selection_state.selection.clone();
    let layout = world.resource::<layout::Layout>();
    let mut q_selectable = world.query::<One<&mut dyn Selectable>>();
    let selected_inspectable = if let Selection::Single(generic_id) = selection {
        Some(
            q_selectable
                .get(world, layout.get_entity(&generic_id).unwrap())
                .unwrap(),
        )
    } else {
        None
    };

    let mut egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
        .unwrap()
        .clone();
    let type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = type_registry.read();
    let mut queue = CommandQueue::default();
    let mut cx = Context {
        world: Some(world.into()),
        queue: Some(&mut queue),
    };
    let mut env = InspectorUi::for_bevy(&type_registry, &mut cx);

    let inner_response = egui::SidePanel::new(egui::panel::Side::Right, Id::new("Inspector")).show(
        egui_context.get_mut(),
        |ui| {
            ui.label("Hello World!");
            // ui_for_value_readonly(&layout.in_markers, ui, &type_registry.read());
            ui.separator();
            if let Some(inspectable) = selected_inspectable {
                inspectable.inspector_ui_env(ui, &mut env);
            }
        },
    );

    let egui_mouse_pos = world.resource::<EguiMousePosition>().0.clone();
    let mut input_data = world.resource_mut::<InputData>();
    if let Some((_, mouse_pos)) = egui_mouse_pos {
        input_data.mouse_over_ui = inner_response.response.rect.contains(mouse_pos.to_pos2());
    }

    queue.apply(world);
}

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (inspector_system, inspector_system_world));
        app.add_plugins(DefaultInspectorConfigPlugin);
    }
}
