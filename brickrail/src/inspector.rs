use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_egui::{
    egui::{self, Id},
    EguiContexts, EguiMousePosition,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::{
    ble::BLEHub,
    editor::*,
    layout::EntityMap,
    layout_primitives::{HubID, HubType},
};

pub struct InspectorContext<'a> {
    pub ui: &'a mut egui::Ui,
    pub type_registry: &'a TypeRegistry,
    pub entity_map: &'a EntityMap,
    pub commands: Commands<'a, 'a>,
}

impl<'a> InspectorContext<'a> {
    pub fn select_hub_ui(&mut self, selected: &mut Option<HubID>, kind: HubType) {
        egui::ComboBox::from_label("Hub")
            .selected_text(format!("{:?}", selected))
            .show_ui(self.ui, |ui| {
                ui.selectable_value(selected, None, "None");
                for id in self.entity_map.hubs.keys().filter(|id| id.kind == kind) {
                    ui.selectable_value(selected, Some(id.clone()), format!("{:?}", id));
                }
                if ui
                    .button("New Hub")
                    .on_hover_text("Create a new hub")
                    .clicked()
                {
                    *selected = Some(self.entity_map.new_hub_id(kind));
                    let hub = BLEHub::new(selected.unwrap().clone());
                    self.commands
                        .add(|world: &mut World| world.send_event(SpawnEvent(hub)));
                };
            });
    }
}

fn inspector_system(
    type_registry: Res<AppTypeRegistry>,
    mut contexts: EguiContexts,
    mut q_inspectable: Query<&mut dyn Selectable>,
    selection_state: Res<SelectionState>,
    entity_map: ResMut<EntityMap>,
    egui_mouse_pos: Res<EguiMousePosition>,
    mut input_data: ResMut<InputData>,
    commands: Commands,
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
                    let mut context = InspectorContext {
                        ui,
                        type_registry: &type_registry.read(),
                        entity_map: &entity_map,
                        commands: commands,
                    };

                    let mut inspectable_iter = q_inspectable.get_mut(entity).unwrap();
                    for mut inspectable in inspectable_iter.iter_mut() {
                        if inspectable.get_id() != generic_id {
                            continue;
                        }
                        inspectable.inspector_ui(&mut context);
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
