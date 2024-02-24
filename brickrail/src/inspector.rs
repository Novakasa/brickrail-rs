use bevy::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_egui::{
    egui::{self, Layout, Ui},
    EguiContexts, EguiMousePosition,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::{
    ble::BLEHub,
    ble_train::BLETrain,
    block::Block,
    editor::*,
    layout::EntityMap,
    layout_primitives::{HubID, HubType},
    marker::Marker,
    track::Track,
    train::train_inspector,
};

pub fn select_hub_ui(
    ui: &mut Ui,
    selected: &mut Option<HubID>,
    kind: HubType,
    hubs: &Query<&BLEHub>,
    spawn_events: &mut EventWriter<SpawnEvent<SerializedHub>>,
    entity_map: &mut ResMut<EntityMap>,
    selection_state: &mut ResMut<SelectionState>,
) {
    ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
        egui::ComboBox::from_label("")
            .selected_text(match selected {
                Some(id) => get_hub_label(hubs, id),
                None => "None".to_string(),
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(selected, None, "None");
                for hub in hubs.iter().filter(|hub| hub.id.kind == kind) {
                    ui.selectable_value(
                        selected,
                        Some(hub.id.clone()),
                        get_hub_label(hubs, &hub.id),
                    );
                }
                if ui
                    .button("New Hub")
                    .on_hover_text("Create a new hub")
                    .clicked()
                {
                    *selected = Some(entity_map.new_hub_id(kind));
                    let hub = BLEHub::new(selected.unwrap().clone());
                    spawn_events.send(SpawnEvent(SerializedHub { hub }));
                };
            });
        if let Some(hub_id) = selected {
            if ui.button("edit").clicked() {
                selection_state.selection = Selection::Single(GenericID::Hub(hub_id.clone()));
            }
        }
    });
}

fn get_hub_label(hubs: &Query<&BLEHub>, id: &HubID) -> String {
    for hub in hubs.iter() {
        if &hub.id == id {
            match hub.name.as_ref() {
                Some(name) => return name.clone(),
                None => return format!("Unkown {:?}", id),
            }
        }
    }
    return format!("Unkown {:?}", id);
}

fn inspector_system_world(world: &mut World) {
    let mut state = SystemState::<(EguiContexts,)>::new(world);
    let (mut egui_contexts,) = state.get_mut(world);
    let inner_response = egui::SidePanel::new(egui::panel::Side::Right, "Inspector").show(
        &egui_contexts.ctx_mut().clone(),
        |ui| {
            ui.label("Inspector 2");
            {
                ui.separator();
                train_inspector(ui, world);
                ui.separator();
                BLETrain::inspector(ui, world);
                BLEHub::inspector(ui, world);
                Block::inspector(ui, world);
                Track::inspector(ui, world);
                ui.separator();
                Marker::inspector(ui, world);
            };
        },
    );

    let mut state = SystemState::<(Res<EguiMousePosition>, ResMut<InputData>)>::new(world);
    let (egui_mouse_pos, mut input_data) = state.get_mut(world);
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
