use std::io::{Read, Write};

use crate::block::{Block, BlockBundle};
use crate::layout::{Connections, EntityMap, MarkerMap};
use crate::layout_primitives::*;
use crate::marker::Marker;
use crate::section::DirectedSection;
use crate::track::{
    SpawnConnection, SpawnTrack, Track, TrackBaseShape, TrackBundle, TrackConnection,
    TrackShapeType, LAYOUT_SCALE,
};

use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy_egui::egui;
use bevy_mouse_tracking_plugin::{prelude::*, MainCamera, MousePosWorld};
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;
use bevy_trait_query::One;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Default)]
pub struct InputData {
    pub mouse_over_ui: bool,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum GenericID {
    Cell(CellID),
    Track(TrackID),
    LogicalTrack(LogicalTrackID),
    Block(BlockID),
    Train(TrainID),
    Switch(DirectedTrackID),
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
    fn inspector_ui(
        &mut self,
        ui: &mut egui::Ui,
        type_registry: &TypeRegistry,
        entity_map: &mut EntityMap,
    );

    fn get_id(&self) -> GenericID;

    fn get_depth(&self) -> f32;

    fn get_distance(&self, pos: Vec2) -> f32;
}

#[derive(Resource, Debug, Default)]
pub struct SelectionState {
    pub selection: Selection,
    drag_select: bool,
}

#[derive(Resource, Default)]
pub struct HoverState {
    pub hover: Option<GenericID>,
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
    q_selectable: Query<One<&mut dyn Selectable>>,
    mut hover_state: ResMut<HoverState>,
) {
    let mut hover_candidate = None;
    let mut min_dist = f32::INFINITY;
    let mut hover_depth = f32::NEG_INFINITY;
    for selectable in q_selectable.iter() {
        if selectable.get_depth() < hover_depth {
            continue;
        }
        let dist = selectable.get_distance(mouse_world_pos.truncate() / LAYOUT_SCALE);
        if (dist < min_dist || selectable.get_depth() > hover_depth) && dist < 0.0 {
            hover_candidate = Some(selectable.get_id());
            min_dist = dist;
            hover_depth = selectable.get_depth();
        }
    }
    if hover_candidate != hover_state.hover {
        hover_state.hover = hover_candidate;
    }
}

fn init_select(
    buttons: Res<Input<MouseButton>>,
    hover_state: Res<HoverState>,
    mut selection_state: ResMut<SelectionState>,
    input_data: Res<InputData>,
) {
    if input_data.mouse_over_ui {
        return;
    }
    if buttons.just_pressed(MouseButton::Left) {
        match hover_state.hover {
            Some(id) => match id {
                GenericID::Track(track_id) => {
                    let mut section = DirectedSection::new();
                    section
                        .push(
                            track_id.get_directed(TrackDirection::First),
                            &Connections::default(),
                        )
                        .unwrap();
                    selection_state.selection = Selection::Section(section);
                }
                generic => {
                    selection_state.selection = Selection::Single(generic);
                }
            },
            None => {
                selection_state.selection = Selection::None;
            }
        }
        println!("{:?}", selection_state.selection);
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
    buttons: Res<Input<MouseButton>>,
    mut selection_state: ResMut<SelectionState>,
    connections: Res<Connections>,
) {
    if hover_state.is_changed() {
        // println!("{:?}", hover_state.hover);
        if buttons.pressed(MouseButton::Left) {
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

#[derive(Serialize, Deserialize, Clone)]
struct SerializableLayout {
    marker_map: MarkerMap,
    tracks: Vec<Track>,
    connections: Vec<TrackConnection>,
    blocks: Vec<Block>,
    markers: Vec<Marker>,
}

pub fn save_layout(
    marker_map: Res<MarkerMap>,
    q_blocks: Query<&Block>,
    q_markers: Query<&Marker>,
    q_tracks: Query<&Track>,
    q_connections: Query<&TrackConnection>,
    keyboard_buttons: Res<Input<KeyCode>>,
) {
    if keyboard_buttons.just_pressed(KeyCode::S) {
        println!("Saving layout");
        let mut file = std::fs::File::create("layout.json").unwrap();
        let blocks = q_blocks.iter().map(|b| b.clone()).collect();
        let markers = q_markers.iter().map(|m| m.clone()).collect();
        let tracks = q_tracks.iter().map(|t| t.clone()).collect();
        let connections = q_connections.iter().map(|c| c.clone()).collect();
        let layout_val = SerializableLayout {
            marker_map: marker_map.clone(),
            blocks,
            markers,
            tracks,
            connections,
        };
        let json = serde_json::to_string_pretty(&layout_val).unwrap();
        file.write(json.as_bytes()).unwrap();
    }
}

pub fn load_layout(
    mut commands: Commands,
    keyboard_buttons: Res<Input<KeyCode>>,
    mut track_event: EventWriter<SpawnTrack>,
    mut connection_event: EventWriter<SpawnConnection>,
) {
    if keyboard_buttons.just_pressed(KeyCode::L) {
        commands.remove_resource::<Connections>();
        commands.remove_resource::<EntityMap>();
        commands.remove_resource::<MarkerMap>();
        let mut entity_map = EntityMap::default();
        let mut file = std::fs::File::open("layout.json").unwrap();
        let mut json = String::new();
        file.read_to_string(&mut json).unwrap();
        let layout_value: SerializableLayout = serde_json::from_str(&json).unwrap();
        let marker_map = layout_value.marker_map.clone();
        // commands.insert_resource(connections);
        for track in layout_value.tracks {
            track_event.send(SpawnTrack { track });
        }
        for connection in layout_value.connections {
            connection_event.send(SpawnConnection { connection });
        }
        for block in layout_value.blocks {
            continue;
            let block_id = block.id.clone();
            let entity = commands.spawn(BlockBundle::from_block(block)).id();
            entity_map.add_block(block_id, entity);
        }
        for marker in layout_value.markers {
            continue;
            let track_id = marker.track;
            let entity = entity_map.get_entity(&GenericID::Track(track_id)).unwrap();
            commands.entity(entity).insert(marker);
            entity_map.add_marker(track_id, entity);
        }
        println!("markers: {:?}", marker_map.in_markers);
        commands.insert_resource(entity_map);
        commands.insert_resource(marker_map);
        commands.insert_resource(Connections::default());
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
            ),
        );
    }
}
