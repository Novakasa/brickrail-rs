use std::io::{Read, Write};

use crate::layout::{Connections, EntityMap};
use crate::layout_primitives::*;
use crate::section::DirectedSection;
use crate::track::{TrackBaseShape, TrackBundle, TrackShapeType, LAYOUT_SCALE};

use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy_egui::egui;
use bevy_mouse_tracking_plugin::{prelude::*, MainCamera, MousePosWorld};
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;
use bevy_trait_query::One;

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

pub fn save_layout(connections: Res<Connections>, keyboard_buttons: Res<Input<KeyCode>>) {
    if keyboard_buttons.just_pressed(KeyCode::S) {
        println!("Saving layout");
        let mut file = std::fs::File::create("layout.json").unwrap();
        let json = serde_json::to_string_pretty(&connections.into_inner()).unwrap();
        file.write(json.as_bytes()).unwrap();
    }
}

pub fn load_layout(mut commands: Commands, keyboard_buttons: Res<Input<KeyCode>>) {
    if keyboard_buttons.just_pressed(KeyCode::L) {
        commands.remove_resource::<Connections>();
        commands.remove_resource::<EntityMap>();
        let mut entity_map = EntityMap::default();
        let mut file = std::fs::File::open("layout.json").unwrap();
        let mut json = String::new();
        file.read_to_string(&mut json).unwrap();
        let connections: Connections = serde_json::from_str(&json).unwrap();
        spawn_tracks_from_connections(&connections, &mut entity_map, &mut commands);
        commands.insert_resource(connections);
        commands.insert_resource(entity_map);
    }
}

fn spawn_tracks_from_connections(
    connections: &Connections,
    entity_map: &mut EntityMap,
    commands: &mut Commands<'_, '_>,
) {
    for track_id in connections.iter_tracks() {
        if entity_map.tracks.get(&track_id).is_some() {
            continue;
        }
        let entity = commands.spawn(TrackBundle::new(track_id)).id();
        entity_map.add_track(track_id, entity);
    }
    for connection_id in connections.iter_connections() {
        if entity_map.connections_outer.get(&connection_id).is_some() {
            continue;
        }
        let outer_id = commands
            .spawn(TrackBaseShape::new(connection_id, TrackShapeType::Outer))
            .id();
        let inner_id = commands
            .spawn(TrackBaseShape::new(connection_id, TrackShapeType::Inner))
            .id();
        entity_map.add_connection(connection_id, outer_id, inner_id);
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
            ),
        );
    }
}
