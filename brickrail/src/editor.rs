use core::fmt;
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::ble::{BLEHub, HubState};
use crate::block::{Block, BlockSpawnEvent, BlockSpawnEventQuery};
use crate::destination::{Destination, SpawnDestinationEvent, SpawnDestinationEventQuery};
use crate::inspector::inspector_system_world;
use crate::layout::{Connections, EntityMap, MarkerMap, TrackLocks};
use crate::layout_devices::LayoutDevice;
use crate::layout_primitives::*;
use crate::marker::{Marker, MarkerSpawnEvent};
use crate::schedule::{ControlInfo, SpawnScheduleEvent, SpawnScheduleEventQuery, TrainSchedule};
use crate::section::DirectedSection;
use crate::selectable::Selectable;
use crate::switch::{SpawnSwitchEvent, SpawnSwitchEventQuery, Switch};
use crate::switch_motor::{PulseMotor, SpawnPulseMotorEvent};
use crate::track::{SpawnConnectionEvent, SpawnTrackEvent, Track, LAYOUT_SCALE};
use crate::train::{SpawnTrainEvent, SpawnTrainEventQuery, Train, TrainWagon};

use bevy::color::palettes::css::BLUE;
use bevy::ecs::system::{RunSystemOnce, SystemState};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowCloseRequested};
use bevy_egui::egui::panel::TopBottomSide;
use bevy_egui::egui::{Align, Align2, Layout};
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::bevy_egui;
use bevy_inspector_egui::bevy_inspector::ui_for_all_assets;
use bevy_inspector_egui::egui::ComboBox;
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use strum_macros::Display;

#[derive(Resource, Debug, Default)]
pub struct InputData {
    pub mouse_over_ui: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DisconnectAction {
    NewLayout,
    LoadLayout(PathBuf),
    Exit,
    Nothing,
}

#[derive(Resource, Debug)]
pub struct EditorInfo {
    pub disconnect_action: DisconnectAction,
}

impl Default for EditorInfo {
    fn default() -> Self {
        Self {
            disconnect_action: DisconnectAction::Nothing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControlState;

impl ComputedStates for ControlState {
    type SourceStates = EditorState;
    fn compute(sources: EditorState) -> Option<ControlState> {
        match sources {
            EditorState::VirtualControl => Some(ControlState),
            EditorState::DeviceControl => Some(ControlState),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates, Display)]
#[source(ControlState = ControlState)]
pub enum ControlStateMode {
    #[default]
    Manual,
    Random,
    Schedule,
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
    Crossing(TrackID),
    TrackConnection(TrackConnectionID),
    Marker(TrackID),
    Hub(HubID),
    Destination(DestinationID),
    Schedule(ScheduleID),
    LayoutDevice(LayoutDeviceID),
}

impl GenericID {
    pub fn editable_name(&self) -> bool {
        match self {
            GenericID::Hub(_) => false,
            GenericID::Track(_) => false,
            _ => true,
        }
    }
}

impl fmt::Display for GenericID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericID::Cell(id) => write!(f, "{:?}", id),
            GenericID::LayoutDevice(id) => write!(f, "{:?}", id),
            GenericID::Track(id) => write!(f, "{}", id),
            GenericID::LogicalTrack(id) => write!(f, "{}", id),
            GenericID::Block(id) => write!(f, "{}", id),
            GenericID::Train(id) => write!(f, "{}", id),
            GenericID::Switch(id) => write!(f, "Switch({})", id),
            GenericID::TrackConnection(id) => write!(f, "{}", id),
            GenericID::Marker(id) => write!(f, "Marker({})", id),
            GenericID::Hub(id) => write!(f, "{}", id),
            GenericID::Destination(id) => write!(f, "{}", id),
            GenericID::Schedule(id) => write!(f, "{}", id),
            GenericID::Crossing(id) => write!(f, "Crossing({})", id),
        }
    }
}

#[derive(Default, Debug, Clone, Reflect, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Single(GenericID),
    Multi(Vec<GenericID>),
    Section(DirectedSection),
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

#[derive(Debug, Default)]
pub enum HoverFilter {
    #[default]
    All,
    Blocks,
}

impl HoverFilter {
    pub fn matches(&self, id: &GenericID) -> bool {
        match (self, id) {
            (HoverFilter::Blocks, GenericID::Block(_)) => true,
            (HoverFilter::All, _) => true,
            _ => false,
        }
    }
}

#[derive(Resource, Default)]
pub struct HoverState {
    pub hover: Option<GenericID>,
    pub filter: HoverFilter,
    min_dist: f32,
    hover_depth: f32,
    candidate: Option<GenericID>,
    pub button_hover: bool,
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

pub fn directory_panel(world: &mut World) {
    let mut state = SystemState::<(EguiContexts,)>::new(world);
    let (mut egui_contexts,) = state.get_mut(world);
    if let Some(ctx) = &egui_contexts.try_ctx_mut().cloned() {
        egui::SidePanel::new(egui::panel::Side::Left, "Directory").show(ctx, |ui| {
            ui.heading("Directory");
            {
                directory_ui::<Train>(ui, world, "Trains");
                directory_ui::<Block>(ui, world, "Blocks");
                directory_ui::<Switch>(ui, world, "Switches");
                directory_ui::<BLEHub>(ui, world, "Hubs");
                directory_ui::<Destination>(ui, world, "Destinations");
                directory_ui::<TrainSchedule>(ui, world, "Schedules");
            };
            ui.set_min_width(200.0);
            ui.separator();

            ui.collapsing("Assets", |ui| {
                ui_for_all_assets(world, ui);
            });
        });
        state.apply(world);

        let mut state = SystemState::<ResMut<InputData>>::new(world);
        let mut input_data = state.get_mut(world);
        input_data.mouse_over_ui = ctx.wants_pointer_input() || ctx.is_pointer_over_area();
    }
}

pub fn directory_ui<T: Sized + Component + Selectable>(
    ui: &mut egui::Ui,
    world: &mut World,
    heading: &str,
) {
    let mut state = SystemState::<(
        Query<(&T, Option<&Name>)>,
        ResMut<SelectionState>,
        ResMut<HoverState>,
        ResMut<EntityMap>,
        EventWriter<T::SpawnEvent>,
    )>::new(world);
    let (query, mut selection_state, mut hover_state, mut entity_map, mut spawner) =
        state.get_mut(world);
    let mut selected = None;
    let mut hovered = None;
    let selection = if let Selection::Single(sel) = selection_state.selection {
        Some(sel)
    } else {
        None
    };
    ui.collapsing(heading, |ui| {
        for (selectable, name) in query.iter() {
            ui.push_id(selectable.generic_id(), |ui| {
                ui.add_enabled_ui(Some(selectable.generic_id()) != selection, |ui| {
                    let button = &ui.button(format!(
                        "{:}",
                        name.unwrap_or(&Name::from(selectable.name()))
                    ));
                    if button.clicked() {
                        selected = Some(selectable.generic_id());
                    }
                    if button.hovered() {
                        hovered = Some(selectable.generic_id());
                    }
                });
            });
        }
        if let Some(event) = T::default_spawn_event(&mut entity_map) {
            ui.separator();
            if ui.button("New").clicked() {
                spawner.send(event);
            }
        }
        ui.separator();
    });
    if let Some(id) = selected {
        selection_state.selection = Selection::Single(id);
    }
    if let Some(id) = hovered {
        hover_state.hover = Some(id);
        hover_state.button_hover = true;
    }
}

pub fn top_panel(
    mut egui_contexts: EguiContexts,
    mut input_data: ResMut<InputData>,
    mut next_editor_state: ResMut<NextState<EditorState>>,
    editor_state: Res<State<EditorState>>,
    control_state: Option<Res<State<ControlState>>>,
    control_mode: Option<Res<State<ControlStateMode>>>,
    mut next_mode: ResMut<NextState<ControlStateMode>>,
    mut editor_info: ResMut<EditorInfo>,
    control_info: Res<ControlInfo>,
    mut save_events: EventWriter<SaveLayoutEvent>,
) {
    if let Some(ctx) = &egui_contexts.try_ctx_mut().cloned() {
        egui::TopBottomPanel::new(TopBottomSide::Top, "Mode").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    next_editor_state.set(EditorState::Disconnecting);
                    editor_info.disconnect_action = DisconnectAction::NewLayout;
                }

                if ui.button("Load").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("brickrail layouts", &["json"])
                        .pick_file()
                    {
                        next_editor_state.set(EditorState::Disconnecting);
                        editor_info.disconnect_action = DisconnectAction::LoadLayout(path);
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
                    ui.label(format!("Layout mode: {:?}", editor_state.get()));
                    ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.add_enabled_ui(editor_state.get() != &EditorState::Edit, |ui| {
                            if ui.button("Edit").clicked() {
                                next_editor_state.set(EditorState::Edit);
                            }
                        });
                        ui.add_enabled_ui(
                            editor_state.get() != &EditorState::VirtualControl,
                            |ui| {
                                if ui.button("Virtual control").clicked() {
                                    next_editor_state.set(EditorState::VirtualControl);
                                }
                            },
                        );
                        ui.add_enabled_ui(
                            editor_state.get() != &EditorState::DeviceControl
                                && editor_state.get() != &EditorState::PreparingDeviceControl,
                            |ui| {
                                if ui.button("Device control").clicked() {
                                    next_editor_state.set(EditorState::PreparingDeviceControl);
                                }
                            },
                        );
                        ui.separator();
                        if ui.button("Disconnect").clicked() {
                            next_editor_state.set(EditorState::Disconnecting);
                            editor_info.disconnect_action = DisconnectAction::Nothing;
                        }
                        ui.separator();
                        ui.add_enabled_ui(control_state.is_some(), |ui| {
                            let mode =
                                control_mode.map_or(ControlStateMode::Manual, |v| v.get().clone());
                            let mut editable_mode = mode.clone();
                            ComboBox::from_label("")
                                .selected_text(format!("{:}", editable_mode))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut editable_mode,
                                        ControlStateMode::Manual,
                                        "Manual",
                                    );
                                    ui.selectable_value(
                                        &mut editable_mode,
                                        ControlStateMode::Random,
                                        "Random",
                                    );
                                    ui.selectable_value(
                                        &mut editable_mode,
                                        ControlStateMode::Schedule,
                                        "Schedule",
                                    );
                                });

                            if editable_mode != mode {
                                next_mode.set(editable_mode);
                            }
                            ui.heading(format!("Time: {:1.1}", control_info.time))
                        });
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
    commands.spawn((Camera2d::default(), pancam));
}

pub fn init_hover(mut hover_state: ResMut<HoverState>) {
    hover_state.min_dist = f32::INFINITY;
    hover_state.hover_depth = f32::NEG_INFINITY;
    hover_state.candidate = None;
    hover_state.button_hover = false;
}

pub fn finish_hover(mut hover_state: ResMut<HoverState>) {
    hover_state.min_dist = f32::INFINITY;
    hover_state.hover_depth = f32::NEG_INFINITY;
    hover_state.hover = hover_state.candidate;
    hover_state.candidate = None;
}

#[derive(Resource, Debug, Default)]
pub struct MousePosWorld {
    pub pos: Vec2,
}

fn update_world_mouse_pos(
    mut mouse_pos_world: ResMut<MousePosWorld>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = q_camera.single();
    let Ok(window) = q_window.get_single() else {
        return;
    };

    if let Some(world_pos) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
    {
        mouse_pos_world.pos = world_pos;
    }
}

pub fn update_hover<T: Selectable + Component>(
    mouse_world_pos: Res<MousePosWorld>,
    q_selectable: Query<(&mut T, Option<&Transform>, Option<&Stroke>)>,
    mut hover_state: ResMut<HoverState>,
) {
    for (selectable, transform, stroke) in q_selectable.iter() {
        {
            if !hover_state.filter.matches(&selectable.generic_id()) {
                continue;
            }
            if selectable.get_depth() < hover_state.hover_depth {
                continue;
            }
            let dist =
                selectable.get_distance(mouse_world_pos.pos / LAYOUT_SCALE, transform, stroke);
            if dist > 0.0 {
                continue;
            }
            if dist < hover_state.min_dist || selectable.get_depth() > hover_state.hover_depth {
                hover_state.candidate = Some(selectable.generic_id());
                hover_state.min_dist = dist;
                hover_state.hover_depth = selectable.get_depth();
            }
        }
    }
    if hover_state.candidate != hover_state.hover {
        hover_state.hover = hover_state.candidate;
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
                    despawn_events.send(DespawnEvent(component.id()));
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
                track.draw_with_gizmos(&mut gizmos, LAYOUT_SCALE, Color::from(BLUE));
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
    match hover_state.hover {
        Some(GenericID::Track(_)) => {}
        _ => {
            return;
        }
    }
    // println!("{:?}", hover_state.hover);
    if buttons.pressed(MouseButton::Left) {
        if let Selection::Single(GenericID::Track(track_id)) = selection_state.selection {
            if hover_state.hover == Some(GenericID::Track(track_id)) {
                return;
            }

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

#[derive(Serialize, Deserialize, Clone, Event)]
pub struct SpawnHubEvent {
    pub hub: BLEHub,
}

#[derive(Serialize, Deserialize, Clone)]
struct SerializableLayout {
    marker_map: MarkerMap,
    tracks: Vec<SpawnTrackEvent>,
    connections: Vec<SpawnConnectionEvent>,
    blocks: Vec<BlockSpawnEvent>,
    markers: Vec<Marker>,
    #[serde(default)]
    trains: Vec<SpawnTrainEvent>,
    #[serde(default)]
    hubs: Vec<SpawnHubEvent>,
    #[serde(default)]
    switches: Vec<SpawnSwitchEvent>,
    #[serde(default)]
    switch_motors: Vec<SpawnPulseMotorEvent>,
    #[serde(default)]
    destinations: Vec<SpawnDestinationEvent>,
    #[serde(default)]
    schedules: Vec<SpawnScheduleEvent>,
}

pub fn save_layout(
    marker_map: Res<MarkerMap>,
    q_trains: SpawnTrainEventQuery,
    q_switches: SpawnSwitchEventQuery,
    q_blocks: BlockSpawnEventQuery,
    q_markers: Query<&Marker>,
    q_tracks: Query<&Track>,
    q_hubs: Query<&BLEHub>,
    q_switch_motors: Query<(&PulseMotor, &LayoutDevice)>,
    q_destinations: SpawnDestinationEventQuery,
    q_schedules: SpawnScheduleEventQuery,
    connections: Res<Connections>,
    mut save_events: EventReader<SaveLayoutEvent>,
) {
    for event in save_events.read() {
        println!("Saving layout");
        let mut file = std::fs::File::create(event.path.clone()).unwrap();
        let tracks = q_tracks
            .iter()
            .map(|t| SpawnTrackEvent(t.clone()))
            .collect();
        let hubs = q_hubs
            .iter()
            .map(|hub| SpawnHubEvent { hub: hub.clone() })
            .collect();
        let switch_motors = q_switch_motors
            .iter()
            .map(|(motor, device)| SpawnPulseMotorEvent {
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
            blocks: q_blocks.get(),
            markers: q_markers.iter().map(|m| m.clone()).collect(),
            tracks,
            connections,
            trains: q_trains.get(),
            hubs,
            switches: q_switches.get(),
            switch_motors,
            destinations: q_destinations.get(),
            schedules: q_schedules.get(),
        };
        let json = serde_json::to_string_pretty(&layout_val).unwrap();
        file.write(json.as_bytes()).unwrap();
    }
}

#[derive(Event)]
pub struct DespawnEvent<T: Selectable>(pub T::ID);

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
    world.run_system_once(new_layout).unwrap();
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
                commands.queue(|world: &mut World| {
                    world.send_event(track);
                });
            }
            for connection in layout_value.connections {
                commands.queue(|world: &mut World| {
                    world.send_event(connection);
                });
            }
            for block in layout_value.blocks {
                commands.queue(|world: &mut World| {
                    world.send_event(block);
                });
            }
            for marker in layout_value.markers {
                commands.queue(|world: &mut World| {
                    world.send_event(MarkerSpawnEvent(marker));
                });
            }
            for serialized_train in layout_value.trains {
                commands.queue(|world: &mut World| {
                    world.send_event(serialized_train);
                });
            }
            for serialized_hub in layout_value.hubs {
                commands.queue(|world: &mut World| {
                    world.send_event(serialized_hub);
                });
            }
            for serialized_switch in layout_value.switches {
                commands.queue(|world: &mut World| {
                    world.send_event(serialized_switch);
                });
            }
            for serialized_switch_motor in layout_value.switch_motors {
                commands.queue(|world: &mut World| {
                    world.send_event(serialized_switch_motor);
                });
            }
            for destination in layout_value.destinations {
                commands.queue(|world: &mut World| {
                    world.send_event(destination);
                });
            }
            for schedule in layout_value.schedules {
                commands.queue(|world: &mut World| {
                    world.send_event(schedule);
                });
            }
            commands.insert_resource(marker_map);
        }
    }
    params.apply(world);
}

fn new_layout(
    world: &mut World,
    params: &mut SystemState<(Res<EntityMap>, Commands, EventReader<NewLayoutEvent>)>,
) {
    {
        let (entity_map, mut commands, mut events) = params.get_mut(world);
        events.clear();
        for entity in entity_map.iter_all_entities() {
            commands.entity(*entity).despawn_recursive();
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

pub fn close_event(
    mut state: ResMut<NextState<EditorState>>,
    mut closed: EventReader<WindowCloseRequested>,
    mut editor_info: ResMut<EditorInfo>,
) {
    for _event in closed.read() {
        state.set(EditorState::Disconnecting);
        editor_info.disconnect_action = DisconnectAction::Exit;
    }
}

pub fn disconnect_finish(
    mut editor_info: ResMut<EditorInfo>,
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut load_events: EventWriter<LoadLayoutEvent>,
    mut new_events: EventWriter<NewLayoutEvent>,
) {
    match &editor_info.disconnect_action {
        DisconnectAction::Exit => {
            commands.entity(primary_window.single()).despawn();
        }
        DisconnectAction::NewLayout => {
            new_events.send(NewLayoutEvent {});
        }
        DisconnectAction::LoadLayout(path) => {
            load_events.send(LoadLayoutEvent { path: path.clone() });
        }
        DisconnectAction::Nothing => {}
    }
    editor_info.disconnect_action = DisconnectAction::Nothing;
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanCamPlugin);
        app.init_state::<EditorState>();
        app.add_computed_state::<ControlState>();
        app.add_sub_state::<ControlStateMode>();
        app.add_event::<SpawnTrainEvent>();
        app.add_event::<SpawnHubEvent>();
        app.add_event::<LoadLayoutEvent>();
        app.add_event::<SaveLayoutEvent>();
        app.add_event::<NewLayoutEvent>();
        app.insert_resource(HoverState::default());
        app.insert_resource(SelectionState::default());
        app.insert_resource(InputData::default());
        app.insert_resource(EditorInfo::default());
        app.insert_resource(MousePosWorld::default());
        app.add_systems(Startup, spawn_camera);
        app.add_systems(OnExit(EditorState::Disconnecting), disconnect_finish);
        app.add_systems(PreUpdate, update_world_mouse_pos);
        app.add_systems(
            Update,
            (
                (
                    init_hover,
                    finish_hover,
                    init_select,
                    extend_selection,
                    draw_selection,
                )
                    .chain(),
                save_layout.run_if(on_event::<SaveLayoutEvent>),
                load_layout.run_if(on_event::<LoadLayoutEvent>),
                new_layout.run_if(on_event::<NewLayoutEvent>),
                update_editor_state,
                close_event.run_if(on_event::<WindowCloseRequested>),
            ),
        );
        app.add_systems(
            Update,
            (
                (
                    inspector_system_world,
                    directory_panel.after(finish_hover),
                    top_panel,
                )
                    .chain(),
                hub_status_window
                    .after(top_panel)
                    .run_if(in_state(EditorState::PreparingDeviceControl)),
                hub_status_window
                    .after(top_panel)
                    .run_if(in_state(EditorState::Disconnecting)),
            ),
        );
    }
}
