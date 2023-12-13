use crate::block::BLOCK_WIDTH;
use crate::layout::Layout;
use crate::layout_primitives::*;
use crate::section::DirectedSection;
use crate::track::TRACK_WIDTH;
use crate::{block::Block, track::LAYOUT_SCALE};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mouse_tracking_plugin::{prelude::*, MainCamera, MousePosWorld};
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericID {
    Cell(CellID),
    Track(TrackID),
    LogicalTrack(LogicalTrackID),
    Block(BlockID),
    Train(TrainID),
    Switch(DirectedTrackID),
}

#[derive(Default, Debug, Clone)]
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

#[derive(Resource, Default)]
pub struct HoverState {
    pub hover: Option<GenericID>,
}

#[derive(Component)]
pub struct Selectable {
    id: GenericID,
}

impl Selectable {
    pub fn new(id: GenericID) -> Self {
        Self { id: id }
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
    q_selectable: Query<(Entity, &Selectable, Option<&Transform>)>,
    q_blocks: Query<&Block>,
    mut hover_state: ResMut<HoverState>,
) {
    let mut hover_candidate = None;
    let mut min_dist = f32::INFINITY;
    let mut hover_z = f32::NEG_INFINITY;
    for (entity, selectable, transform) in q_selectable.iter() {
        let z = if let Some(t) = transform {
            t.translation.z
        } else {
            f32::INFINITY
        };
        if z < hover_z {
            continue;
        }
        let dist = match selectable.id {
            GenericID::Track(track_id) => {
                track_id.distance_to(mouse_world_pos.truncate() / LAYOUT_SCALE)
                    - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
            }
            GenericID::Block(_) => {
                let block = q_blocks.get(entity).unwrap();
                let block_dist = block.distance_to(mouse_world_pos.truncate() / LAYOUT_SCALE)
                    - BLOCK_WIDTH / LAYOUT_SCALE;
                // println!("block dist: {:}", block_dist);
                block_dist
            }
            _ => 10.0,
        };
        // println!("{:}", dist);
        if dist < min_dist && dist < 0.0 {
            hover_candidate = Some(selectable.id);
            min_dist = dist;
            hover_z = z;
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
) {
    if buttons.just_pressed(MouseButton::Left) {
        match hover_state.hover {
            Some(id) => match id {
                GenericID::Track(track_id) => {
                    let mut section = DirectedSection::new();
                    section
                        .push(
                            track_id.get_directed(TrackDirection::Aligned),
                            &Layout::default(),
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

fn draw_selection(mut gizmos: Gizmos, selection_state: Res<SelectionState>, layout: Res<Layout>) {
    match &selection_state.selection {
        Selection::Section(section) => {
            for track in section.tracks.iter() {
                track.draw_with_gizmos(&mut gizmos, layout.scale, Color::BLUE);
            }
        }
        _ => {}
    }
}

fn extend_selection(
    hover_state: Res<HoverState>,
    buttons: Res<Input<MouseButton>>,
    mut selection_state: ResMut<SelectionState>,
    layout: Res<Layout>,
) {
    if hover_state.is_changed() {
        // println!("{:?}", hover_state.hover);
        if buttons.pressed(MouseButton::Left) {
            match (&hover_state.hover, &mut selection_state.selection) {
                (Some(GenericID::Track(track_id)), Selection::Section(section)) => {
                    match section.push_track(*track_id, &layout) {
                        Ok(()) => {
                            return;
                        }
                        Err(()) => {}
                    }
                    let mut opposite = section.get_opposite();
                    match opposite.push_track(*track_id, &layout) {
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

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Sample8);
        app.add_plugins(PanCamPlugin);
        app.add_plugins(MousePosPlugin);
        app.add_plugins(WorldInspectorPlugin::default());
        app.add_plugins(ShapePlugin);
        app.insert_resource(HoverState::default());
        app.insert_resource(SelectionState::default());
        app.add_systems(Startup, spawn_camera);
        app.add_systems(
            Update,
            (init_select, update_hover, draw_selection, extend_selection),
        );
    }
}
