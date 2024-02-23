use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_ecs::system::SystemState;
use bevy_egui::{
    egui::{self, Id, Layout, Ui},
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
    pub type_registry: &'a TypeRegistry,
    pub entity_map: &'a EntityMap,
    pub selection_state: &'a mut SelectionState,
    pub commands: Commands<'a, 'a>,
}

impl<'a> InspectorContext<'a> {
    pub fn select_hub_ui(&mut self, ui: &mut Ui, selected: &mut Option<HubID>, kind: HubType) {
        ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
            egui::ComboBox::from_label("")
                .selected_text(match selected {
                    Some(id) => self.get_hub_label(id),
                    None => "None".to_string(),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(selected, None, "None");
                    for id in self.entity_map.hubs.keys().filter(|id| id.kind == kind) {
                        ui.selectable_value(selected, Some(id.clone()), self.get_hub_label(id));
                    }
                    if ui
                        .button("New Hub")
                        .on_hover_text("Create a new hub")
                        .clicked()
                    {
                        *selected = Some(self.entity_map.new_hub_id(kind));
                        let hub = BLEHub::new(selected.unwrap().clone());
                        self.commands.add(|world: &mut World| {
                            world.send_event(SpawnEvent(SerializedHub { hub }))
                        });
                    };
                });
            if let Some(hub_id) = selected {
                if ui.button("edit").clicked() {
                    self.selection_state.selection =
                        Selection::Single(GenericID::Hub(hub_id.clone()));
                }
            }
        });
    }

    fn get_hub_label(&mut self, id: &HubID) -> String {
        let label = self
            .entity_map
            .names
            .get(&GenericID::Hub(id.clone()))
            .cloned()
            .unwrap_or(format!("Unknown {:}", id));
        label
    }
}

fn inspector_widget(ui: &mut Ui, world: &mut World) {
    let mut state =
        SystemState::<(Query<&dyn Selectable>, Res<SelectionState>, Res<EntityMap>)>::new(world);
    let (inspectable, selection_state, entity_map) = state.get(world);
    ui.separator();
    let selection = selection_state.selection.clone();
    if let Selection::Single(generic_id) = selection {
        if let Some(entity) = entity_map.get_entity(&generic_id) {
            let inspectable_iter = inspectable.get(entity).unwrap();
            for inspectable in inspectable_iter.iter() {
                if inspectable.get_id() != generic_id {
                    continue;
                }
                ui.label(format!("Inspectable {:?}", generic_id));
                inspectable.inspector_ui_world(ui, world);
                ui.separator();
            }
        }
    }
}

fn inspector_system_world(world: &mut World) {
    let mut state = SystemState::<(EguiContexts,)>::new(world);
    let (mut egui_contexts,) = state.get_mut(world);
    let inner_response = egui::SidePanel::new(egui::panel::Side::Right, "Inspector").show(
        &egui_contexts.ctx_mut().clone(),
        |ui| {
            ui.label("Inspector 2");
            inspector_widget(ui, world);
        },
    );

    let mut state = SystemState::<(Res<EguiMousePosition>, ResMut<InputData>)>::new(world);
    let (egui_mouse_pos, mut input_data) = state.get_mut(world);
    if let Some((_, mouse_pos)) = egui_mouse_pos.0 {
        input_data.mouse_over_ui = inner_response.response.rect.contains(mouse_pos.to_pos2());
    }
}

fn inspector_system(
    type_registry: Res<AppTypeRegistry>,
    mut contexts: EguiContexts,
    mut q_inspectable: Query<&mut dyn Selectable>,
    mut selection_state: ResMut<SelectionState>,
    entity_map: ResMut<EntityMap>,
    egui_mouse_pos: Res<EguiMousePosition>,
    mut input_data: ResMut<InputData>,
    commands: Commands,
) {
    let inner_response = egui::SidePanel::new(egui::panel::Side::Right, Id::new("Inspector")).show(
        contexts.ctx_mut(),
        |ui| {
            ui.label("Inspector");
            // ui_for_value_readonly(&layout.in_markers, ui, &type_registry.read());
            ui.separator();
            let selection = selection_state.selection.clone();
            if let Selection::Single(generic_id) = selection {
                if let Some(entity) = entity_map.get_entity(&generic_id) {
                    let mut context = InspectorContext {
                        type_registry: &type_registry.read(),
                        entity_map: &entity_map,
                        commands: commands,
                        selection_state: &mut selection_state,
                    };

                    let mut inspectable_iter = q_inspectable.get_mut(entity).unwrap();
                    for mut inspectable in inspectable_iter.iter_mut() {
                        if inspectable.get_id() != generic_id {
                            continue;
                        }
                        inspectable.inspector_ui(ui, &mut context);
                        ui.separator();
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
        app.add_systems(Update, inspector_system_world);
        app.add_plugins(DefaultInspectorConfigPlugin);
    }
}
