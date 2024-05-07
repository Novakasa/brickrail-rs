use std::io::{Read, Write};
use std::path::PathBuf;

use crate::ble::{BLEHub, HubState};
use crate::ble_train::BLETrain;
use crate::block::{Block, BlockSpawnEvent};
use crate::inspector::inspector_system_world;
use crate::layout::{Connections, EntityMap, MarkerMap, TrackLocks};
use crate::layout_devices::LayoutDevice;
use crate::layout_primitives::*;
use crate::marker::{Marker, MarkerSpawnEvent};
use crate::section::DirectedSection;
use crate::switch::{SpawnSwitchEvent, Switch};
use crate::switch_motor::{SpawnSwitchMotorEvent, SwitchMotor};
use crate::track::{SpawnConnectionEvent, SpawnTrackEvent, Track, LAYOUT_SCALE};
use crate::train::Train;

use bevy::prelude::*;
use bevy_ecs::system::{RunSystemOnce, SystemState};
use bevy_egui::egui::panel::TopBottomSide;
use bevy_egui::egui::{Align, Align2, Layout};
use bevy_egui::{egui, EguiContexts};
use bevy_mouse_tracking_plugin::{prelude::*, MainCamera, MousePosWorld};
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Default)]
pub struct InputData {
    pub mouse_over_ui: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DisconnectAction {
    NewLayout,
    LoadLayout(PathBuf),
    Exit,
}

#[derive(Debug, States, Default, Hash, PartialEq, Eq, Clone)]
pub enum EditorState {
    #[default]
    Edit,
    PreparingDeviceControl,
    DeviceControl,
    VirtualControl,
    Disconnecting,
}

impl EditorState {
    pub fn ble_commands_enabled(&self) -> bool {
        match self {
            EditorState::DeviceControl => true,
            _ => false,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect, Hash)]
pub enum GenericID {
    Cell(CellID),
    Track(TrackID),
    LogicalTrack(LogicalTrackID),
    Block(BlockID),
    Train(TrainID),
    Switch(DirectedTrackID),
    TrackConnection(TrackConnectionID),
    Marker(TrackID),
    Hub(HubID),
}

#[derive(Default, Debug, Clone, Reflect, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Single(GenericID),
    Multi(Vec<GenericID>),
    Section(DirectedSection),
}

#[bevy_trait_query::queryable]
pub trait Selectable {
    fn get_id(&self) -> GenericID;

    fn get_depth(&self) -> f32 {
        -100.0
    }

    fn get_distance(&self, _pos: Vec2) -> f32 {
        100.0
    }
}

#[derive(Resource, Debug, Default)]
pub struct SelectionState {
    pub selection: Selection,
    drag_select: bool,
}

impl SelectionState {
    pub fn get_entity(&self, entity_map: &EntityMap) -> Option<Entity> {
        match &self.selection {
            Selection::Single(id) => entity_map.get_entity(id),
            _ => None,
        }
    }
}

#[derive(Resource, Default)]
pub struct HoverState {
    pub hover: Option<GenericID>,
}

pub struct ControlState {
    pub random_targets: bool,
    pub control_devices: bool,
}

fn update_editor_state(
    mut editor_state: ResMut<NextState<EditorState>>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_buttons.just_pressed(KeyCode::Digit1) {
        editor_state.set(EditorState::Edit);
    }
    if keyboard_buttons.just_pressed(KeyCode::Digit2) {
        editor_state.set(EditorState::PreparingDeviceControl);
    }
}

pub fn top_panel(
    mut egui_contexts: EguiContexts,
    mut input_data: ResMut<InputData>,
    mut next_editor_state: ResMut<NextState<EditorState>>,
    editor_state: Res<State<EditorState>>,
    mut load_events: EventWriter<LoadLayoutEvent>,
    mut save_events: EventWriter<SaveLayoutEvent>,
    mut new_events: EventWriter<NewLayoutEvent>,
) {
    if let Some(ctx) = &egui_contexts.try_ctx_mut().cloned() {
        egui::TopBottomPanel::new(TopBottomSide::Top, "Mode").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    new_events.send(NewLayoutEvent {});
                }

                if ui.button("Load").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("brickrail layouts", &["json"])
                        .pick_file()
                    {
                        load_events.send(LoadLayoutEvent { path: path });
                    }
                }
                if ui.button("Save").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("brickrail layouts", &["json"])
                        .save_file()
                    {
                        save_events.send(SaveLayoutEvent { path: path });
                    }
                }
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(format!("Mode: {:?}", editor_state.get()));
                    ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                        if ui.button("Edit").clicked() {
                            next_editor_state.set(EditorState::Edit);
                        }
                        if ui.button("Virtual control").clicked() {
                            next_editor_state.set(EditorState::VirtualControl);
                        }
                        if ui.button("Device control").clicked() {
                            next_editor_state.set(EditorState::PreparingDeviceControl);
                        }
                        if ui.button("Disconnect").clicked() {
                            next_editor_state.set(EditorState::Disconnecting);
                        }
                    });
                });
            });
        });

        input_data.mouse_over_ui |= ctx.wants_pointer_input() || ctx.is_pointer_over_area();
    }
}

pub fn hub_status_window(
    mut egui_contexts: EguiContexts,
    mut input_data: ResMut<InputData>,
    mut q_hubs: Query<&mut BLEHub>,
    mut editor_state: ResMut<NextState<EditorState>>,
) {
    if let Some(ctx) = &egui_contexts.try_ctx_mut().cloned() {
        egui::Window::new("Hub status")
            .movable(false)
            .collapsible(false)
            .resizable(false)
            .default_width(200.0)
            .max_width(200.0)
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("Preparing hubs...");
                ui.separator();
                for mut hub in q_hubs.iter_mut() {
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        ui.heading(hub.name.clone().unwrap_or("Unknown".to_string()));
                        if hub.state == HubState::Ready || !hub.active {
                            // ui.heading("✔".to_string());
                            ui.label(
                                egui::RichText::new("✔".to_string())
                                    .heading()
                                    .color(egui::Color32::GREEN),
                            );
                        }
                    });
                    if hub.active {
                        ui.label("Active");
                    } else {
                        ui.label("Inactive");
                    }
                    match &hub.state {
                        HubState::Downloading(progress) => {
                            ui.horizontal(|ui| {
                                ui.label("Downloading...");
                                ui.add(egui::ProgressBar::new(*progress));
                            });
                        }
                        HubState::Connecting => {
                            ui.horizontal(|ui| {
                                ui.label("Connecting...");
                                ui.add(egui::Spinner::default());
                            });
                        }
                        HubState::StartingProgram => {
                            ui.horizontal(|ui| {
                                ui.label("Starting program...");
                                ui.add(egui::Spinner::default());
                            });
                        }
                        HubState::Configuring => {
                            ui.horizontal(|ui| {
                                ui.label("Configuring...");
                                ui.add(egui::Spinner::default());
                            });
                        }
                        HubState::Ready => {}
                        state => {
                            ui.label(format!("{:?}", state));
                        }
                    }
                    if let Some(err) = &hub.error {
                        ui.label(format!("Error: {:?}", err));
                        if ui.button("Retry").clicked() {
                            hub.error = None;
                        }
                    }
                    ui.separator();
                }
                if ui.button("Cancel").clicked() {
                    editor_state.set(EditorState::Edit);
                }
            });

        input_data.mouse_over_ui |= ctx.is_pointer_over_area() || ctx.wants_pointer_input();
    }
}

fn spawn_camera(mut commands: Commands) {
    let pancam = PanCam {
        grab_buttons: vec![MouseButton::Middle],
        ..default()
    };
    commands
        .spawn((Camera2dBundle::default(), pancam))
        .add(InitWorldTracking)
        .insert(MainCamera);
}

fn update_hover(
    mouse_world_pos: Res<MousePosWorld>,
    q_selectable: Query<&mut dyn Selectable>,
    mut hover_state: ResMut<HoverState>,
) {
    let mut hover_candidate = None;
    let mut min_dist = f32::INFINITY;
    let mut hover_depth = f32::NEG_INFINITY;
    for entity in q_selectable.iter() {
        for selectable in entity.iter() {
            if selectable.get_depth() < hover_depth {
                continue;
            }
            let dist = selectable.get_distance(mouse_world_pos.truncate() / LAYOUT_SCALE);
            if dist > 0.0 {
                continue;
            }
            if dist < min_dist || selectable.get_depth() > hover_depth {
                hover_candidate = Some(selectable.get_id());
                min_dist = dist;
                hover_depth = selectable.get_depth();
            }
        }
    }
    if hover_candidate != hover_state.hover {
        hover_state.hover = hover_candidate;
        // println!("Hovering {:?}", hover_state.hover);
    }
}

fn init_select(
    buttons: Res<ButtonInput<MouseButton>>,
    hover_state: Res<HoverState>,
    mut selection_state: ResMut<SelectionState>,
    input_data: Res<InputData>,
) {
    if input_data.mouse_over_ui {
        return;
    }
    if buttons.just_pressed(MouseButton::Left) {
        match hover_state.hover {
            Some(id) => {
                selection_state.selection = Selection::Single(id);
            }
            None => {
                selection_state.selection = Selection::None;
            }
        }
        println!("{:?}", selection_state.selection);
    }
}

pub fn delete_selection_shortcut<T: Selectable + Component + Clone>(
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    mut selection_state: ResMut<SelectionState>,
    mut q_selectable: Query<&mut T>,
    mut despawn_events: EventWriter<DespawnEvent<T>>,
    entity_map: Res<EntityMap>,
) {
    if keyboard_buttons.just_pressed(KeyCode::Delete) {
        match &selection_state.selection {
            Selection::Single(id) => {
                let entity = entity_map.get_entity(id).unwrap();
                if let Ok(component) = q_selectable.get_mut(entity) {
                    despawn_events.send(DespawnEvent(component.clone()));
                    selection_state.selection = Selection::None;
                }
            }
            _ => {}
        }
    }
}

fn draw_selection(mut gizmos: Gizmos, selection_state: Res<SelectionState>) {
    match &selection_state.selection {
        Selection::Section(section) => {
            for track in section.tracks.iter() {
                track.draw_with_gizmos(&mut gizmos, LAYOUT_SCALE, Color::BLUE);
            }
        }
        _ => {}
    }
}

fn extend_selection(
    hover_state: Res<HoverState>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut selection_state: ResMut<SelectionState>,
    connections: Res<Connections>,
) {
    if hover_state.is_changed() {
        // println!("{:?}", hover_state.hover);
        if buttons.pressed(MouseButton::Left) {
            if let Selection::Single(GenericID::Track(track_id)) = selection_state.selection {
                let mut section = DirectedSection::new();
                section
                    .push(
                        track_id.get_directed(TrackDirection::First),
                        &Connections::default(),
                    )
                    .unwrap();
                selection_state.selection = Selection::Section(section);
            }
            match (&hover_state.hover, &mut selection_state.selection) {
                (Some(GenericID::Track(track_id)), Selection::Section(section)) => {
                    match section.push_track(*track_id, &connections) {
                        Ok(()) => {
                            return;
                        }
                        Err(()) => {}
                    }
                    let mut opposite = section.get_opposite();
                    match opposite.push_track(*track_id, &connections) {
                        Ok(()) => {
                            println!("opposite");
                            selection_state.selection = Selection::Section(opposite);
                            return;
                        }
                        Err(()) => {}
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Event)]
pub struct SpawnTrainEvent {
    pub train: Train,
    pub ble_train: Option<BLETrain>,
}

#[derive(Serialize, Deserialize, Clone, Event)]
pub struct SpawnHubEvent {
    pub hub: BLEHub,
}

#[derive(Serialize, Deserialize, Clone)]
struct SerializableLayout {
    marker_map: MarkerMap,
    tracks: Vec<SpawnTrackEvent>,
    connections: Vec<SpawnConnectionEvent>,
    blocks: Vec<Block>,
    markers: Vec<Marker>,
    #[serde(default)]
    trains: Vec<SpawnTrainEvent>,
    #[serde(default)]
    hubs: Vec<SpawnHubEvent>,
    #[serde(default)]
    switches: Vec<SpawnSwitchEvent>,
    #[serde(default)]
    switch_motors: Vec<SpawnSwitchMotorEvent>,
}

pub fn save_layout(
    marker_map: Res<MarkerMap>,
    q_trains: Query<(&Train, &BLETrain)>,
    q_switches: Query<&Switch>,
    q_blocks: Query<&Block>,
    q_markers: Query<&Marker>,
    q_tracks: Query<&Track>,
    q_hubs: Query<&BLEHub>,
    q_switch_motors: Query<(&SwitchMotor, &LayoutDevice)>,
    connections: Res<Connections>,
    mut save_events: EventReader<SaveLayoutEvent>,
) {
    for event in save_events.read() {
        println!("Saving layout");
        let mut file = std::fs::File::create(event.path.clone()).unwrap();
        let blocks = q_blocks.iter().map(|b| b.clone()).collect();
        let markers = q_markers.iter().map(|m| m.clone()).collect();
        let tracks = q_tracks
            .iter()
            .map(|t| SpawnTrackEvent(t.clone()))
            .collect();
        let trains = q_trains
            .iter()
            .map(|(train, ble_train)| SpawnTrainEvent {
                train: train.clone(),
                ble_train: Some(ble_train.clone()),
            })
            .collect();
        let switches = q_switches
            .iter()
            .map(|switch| SpawnSwitchEvent {
                switch: switch.clone(),
            })
            .collect();
        let hubs = q_hubs
            .iter()
            .map(|hub| SpawnHubEvent { hub: hub.clone() })
            .collect();
        let switch_motors = q_switch_motors
            .iter()
            .map(|(motor, device)| SpawnSwitchMotorEvent {
                motor: motor.clone(),
                device: device.clone(),
            })
            .collect();
        let connections = connections
            .connection_graph
            .all_edges()
            .map(|(_, _, c)| SpawnConnectionEvent {
                id: c.clone(),
                update_switches: false,
            })
            .collect();
        let layout_val = SerializableLayout {
            marker_map: marker_map.clone(),
            blocks,
            markers,
            tracks,
            connections,
            trains,
            hubs,
            switches,
            switch_motors,
        };
        let json = serde_json::to_string_pretty(&layout_val).unwrap();
        file.write(json.as_bytes()).unwrap();
    }
}

#[derive(Event)]
pub struct DespawnEvent<T>(pub T);

#[derive(Event)]
pub struct LoadLayoutEvent {
    path: PathBuf,
}

#[derive(Event)]
pub struct SaveLayoutEvent {
    path: PathBuf,
}

#[derive(Event)]
pub struct NewLayoutEvent {}

pub fn load_layout(
    world: &mut World,
    params: &mut SystemState<(Commands, EventReader<LoadLayoutEvent>)>,
) {
    world.run_system_once(new_layout);
    {
        let (mut commands, mut load_events) = params.get_mut(world);
        for event in load_events.read() {
            commands.remove_resource::<Connections>();
            commands.remove_resource::<EntityMap>();
            commands.remove_resource::<MarkerMap>();
            commands.insert_resource(EntityMap::default());
            commands.insert_resource(Connections::default());
            let mut file = std::fs::File::open(event.path.clone()).unwrap();
            let mut json = String::new();
            file.read_to_string(&mut json).unwrap();
            let layout_value: SerializableLayout = serde_json::from_str(&json).unwrap();
            let marker_map = layout_value.marker_map.clone();
            println!("Sending spawn events");
            // commands.insert_resource(connections);
            for track in layout_value.tracks {
                commands.add(|world: &mut World| {
                    world.send_event(track);
                });
            }
            for connection in layout_value.connections {
                commands.add(|world: &mut World| {
                    world.send_event(connection);
                });
            }
            for block in layout_value.blocks {
                commands.add(|world: &mut World| {
                    world.send_event(BlockSpawnEvent(block));
                });
            }
            for marker in layout_value.markers {
                commands.add(|world: &mut World| {
                    world.send_event(MarkerSpawnEvent(marker));
                });
            }
            for serialized_train in layout_value.trains {
                commands.add(|world: &mut World| {
                    world.send_event(serialized_train);
                });
            }
            for serialized_hub in layout_value.hubs {
                commands.add(|world: &mut World| {
                    world.send_event(serialized_hub);
                });
            }
            for serialized_switch in layout_value.switches {
                commands.add(|world: &mut World| {
                    world.send_event(serialized_switch);
                });
            }
            for serialized_switch_motor in layout_value.switch_motors {
                commands.add(|world: &mut World| {
                    world.send_event(serialized_switch_motor);
                });
            }
            commands.insert_resource(marker_map);
        }
    }
    params.apply(world);
}

fn draw_markers(q_markers: Query<&Marker>, mut gizmos: Gizmos) {
    for marker in q_markers.iter() {
        marker.draw_with_gizmos(&mut gizmos);
    }
}

fn new_layout(
    world: &mut World,
    params: &mut SystemState<(Res<EntityMap>, Commands, EventReader<NewLayoutEvent>)>,
) {
    {
        let (entity_map, mut commands, mut events) = params.get_mut(world);
        events.clear();
        for entity in entity_map.iter_all_entities() {
            commands.entity(*entity).despawn();
        }
    }
    params.apply(world);
    world.remove_resource::<Connections>();
    world.remove_resource::<EntityMap>();
    world.remove_resource::<MarkerMap>();
    world.remove_resource::<TrackLocks>();
    world.insert_resource(EntityMap::default());
    world.insert_resource(Connections::default());
    world.insert_resource(MarkerMap::default());
    world.insert_resource(TrackLocks::default());
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Sample8);
        app.add_plugins(PanCamPlugin);
        app.add_plugins(MousePosPlugin);
        app.add_plugins(ShapePlugin);
        app.init_state::<EditorState>();
        app.add_event::<SpawnTrainEvent>();
        app.add_event::<SpawnHubEvent>();
        app.add_event::<LoadLayoutEvent>();
        app.add_event::<SaveLayoutEvent>();
        app.add_event::<NewLayoutEvent>();
        app.insert_resource(HoverState::default());
        app.insert_resource(SelectionState::default());
        app.insert_resource(InputData::default());
        app.add_systems(Startup, spawn_camera);
        app.add_systems(
            Update,
            (
                init_select,
                update_hover,
                draw_selection,
                extend_selection,
                save_layout.run_if(on_event::<SaveLayoutEvent>()),
                load_layout.run_if(on_event::<LoadLayoutEvent>()),
                new_layout.run_if(on_event::<NewLayoutEvent>()),
                draw_markers,
                update_editor_state,
            ),
        );
        app.add_systems(
            Update,
            (
                top_panel.after(inspector_system_world),
                hub_status_window
                    .after(top_panel)
                    .run_if(in_state(EditorState::PreparingDeviceControl)),
            ),
        );
    }
}
