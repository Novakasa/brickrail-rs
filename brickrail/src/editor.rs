use std::io::{Read, Write};

use crate::ble::{BLEHub, HubState};
use crate::ble_switch::BLESwitch;
use crate::ble_train::BLETrain;
use crate::block::{Block, BlockSpawnEvent};
use crate::layout::{Connections, EntityMap, MarkerMap};
use crate::layout_primitives::*;
use crate::marker::{Marker, MarkerSpawnEvent};
use crate::section::DirectedSection;
use crate::switch::{SpawnSwitchEvent, Switch};
use crate::track::{SpawnConnectionEvent, SpawnTrackEvent, Track, TrackConnection, LAYOUT_SCALE};
use crate::train::Train;

use bevy::prelude::*;
use bevy_mouse_tracking_plugin::{prelude::*, MainCamera, MousePosWorld};
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Default)]
pub struct InputData {
    pub mouse_over_ui: bool,
}

#[derive(Debug, States, Default, Hash, PartialEq, Eq, Clone)]
pub enum EditorState {
    #[default]
    Edit,
    PreparingDeviceControl,
    DeviceControl,
    VirtualControl,
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

// runs on enter prepare_control state
fn update_active_hubs(mut hubs: Query<&mut BLEHub>) {
    for mut hub in hubs.iter_mut() {
        hub.active = true;

        if hub.state == HubState::ProgramError {
            hub.state = HubState::Connected;
        }
        if hub.state == HubState::ConnectError {
            hub.state = HubState::Disconnected;
        }
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

pub fn delete_selection<T: Selectable + Component + Clone>(
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    selection_state: Res<SelectionState>,
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
    tracks: Vec<Track>,
    connections: Vec<SpawnConnectionEvent>,
    blocks: Vec<Block>,
    markers: Vec<Marker>,
    #[serde(default)]
    trains: Vec<SpawnTrainEvent>,
    #[serde(default)]
    hubs: Vec<SpawnHubEvent>,
    #[serde(default)]
    switches: Vec<SpawnSwitchEvent>,
}

pub fn save_layout(
    marker_map: Res<MarkerMap>,
    q_trains: Query<(&Train, &BLETrain)>,
    q_switches: Query<(&Switch, &BLESwitch)>,
    q_blocks: Query<&Block>,
    q_markers: Query<&Marker>,
    q_tracks: Query<&Track>,
    q_connections: Query<&TrackConnection>,
    q_hubs: Query<&BLEHub>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_buttons.just_pressed(KeyCode::KeyS) {
        println!("Saving layout");
        let mut file = std::fs::File::create("layout.json").unwrap();
        let blocks = q_blocks.iter().map(|b| b.clone()).collect();
        let markers = q_markers.iter().map(|m| m.clone()).collect();
        let tracks = q_tracks.iter().map(|t| t.clone()).collect();
        let trains = q_trains
            .iter()
            .map(|(train, ble_train)| SpawnTrainEvent {
                train: train.clone(),
                ble_train: Some(ble_train.clone()),
            })
            .collect();
        let switches = q_switches
            .iter()
            .map(|(switch, ble_switch)| SpawnSwitchEvent {
                switch: switch.clone(),
                ble_switch: ble_switch.clone(),
            })
            .collect();
        let hubs = q_hubs
            .iter()
            .map(|hub| SpawnHubEvent { hub: hub.clone() })
            .collect();
        let connections = q_connections
            .iter()
            .map(|c| SpawnConnectionEvent {
                id: c.id,
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
        };
        let json = serde_json::to_string_pretty(&layout_val).unwrap();
        file.write(json.as_bytes()).unwrap();
    }
}

#[derive(Event)]
pub struct DespawnEvent<T>(pub T);

pub fn load_layout(mut commands: Commands, keyboard_buttons: Res<ButtonInput<KeyCode>>) {
    if keyboard_buttons.just_pressed(KeyCode::KeyL) {
        commands.remove_resource::<Connections>();
        commands.remove_resource::<EntityMap>();
        commands.remove_resource::<MarkerMap>();
        commands.insert_resource(EntityMap::default());
        commands.insert_resource(Connections::default());
        let mut file = std::fs::File::open("layout.json").unwrap();
        let mut json = String::new();
        file.read_to_string(&mut json).unwrap();
        let layout_value: SerializableLayout = serde_json::from_str(&json).unwrap();
        let marker_map = layout_value.marker_map.clone();
        println!("Sending spawn events");
        // commands.insert_resource(connections);
        for track in layout_value.tracks {
            commands.add(|world: &mut World| {
                world.send_event(SpawnTrackEvent(track));
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
        commands.insert_resource(marker_map);
    }
}

fn draw_markers(q_markers: Query<&Marker>, mut gizmos: Gizmos) {
    for marker in q_markers.iter() {
        marker.draw_with_gizmos(&mut gizmos);
    }
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
                save_layout,
                load_layout,
                draw_markers,
                update_editor_state,
            ),
        );
        app.add_systems(
            OnEnter(EditorState::PreparingDeviceControl),
            update_active_hubs,
        );
    }
}
