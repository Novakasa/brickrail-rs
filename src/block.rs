use crate::editor::{GenericID, HoverState, Selectable, Selection, SelectionState};
use crate::layout::{self, EntityMap, MarkerMap};
use crate::marker::{Marker, MarkerColor};
use crate::section::LogicalSection;
use crate::{layout_primitives::*, section::DirectedSection, track::LAYOUT_SCALE};
use bevy::input::keyboard;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy_egui::egui;
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_prototype_lyon::{
    draw::Stroke,
    entity::ShapeBundle,
    path::PathBuilder,
    prelude::{LineCap, StrokeOptions},
};
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

pub const BLOCK_WIDTH: f32 = 20.0;

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, Serialize, Deserialize,
)]
struct LogicalID {
    direction: BlockDirection,
    facing: Facing,
}

#[derive(Debug, Reflect, Default, Serialize, Deserialize, Clone)]
pub struct BlockSettings {
    pub name: String,
    pub length: f32,
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct Block {
    pub id: BlockID,
    section: DirectedSection,
    pub settings: BlockSettings,
}

impl Block {
    pub fn distance_to(&self, pos: Vec2) -> f32 {
        self.section.distance_to(pos)
    }

    pub fn get_logical_section(&self, block_id: LogicalBlockID) -> LogicalSection {
        match block_id.direction {
            BlockDirection::Aligned => self.section.get_logical(block_id.facing),
            BlockDirection::Opposite => self.section.get_opposite().get_logical(block_id.facing),
        }
    }
}

impl Selectable for Block {
    fn inspector_ui(&mut self, ui: &mut egui::Ui, type_registry: &TypeRegistry, _: &mut EntityMap) {
        ui.label("Inspectable block lol");
        ui_for_value(&mut self.settings, ui, type_registry);
    }

    fn get_depth(&self) -> f32 {
        0.0
    }

    fn get_id(&self) -> GenericID {
        GenericID::Block(self.id)
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        let block_dist = self.distance_to(pos) - BLOCK_WIDTH / LAYOUT_SCALE;
        block_dist
    }
}

#[derive(Bundle)]
pub struct BlockBundle {
    shape: ShapeBundle,
    stroke: Stroke,
    block: Block,
}

impl BlockBundle {
    pub fn new(section: DirectedSection) -> Self {
        Self::from_block(Block {
            id: section.to_block_id(),
            section: section,
            settings: BlockSettings::default(),
        })
    }

    pub fn from_block(block: Block) -> Self {
        let shape = generate_block_shape(&block.section);
        let stroke = {
            let stroke = Stroke {
                color: Color::GREEN,
                options: StrokeOptions::default()
                    .with_line_width(BLOCK_WIDTH)
                    .with_line_cap(LineCap::Round),
            };
            stroke
        };

        Self {
            shape: shape,
            stroke: stroke,
            block: block,
        }
    }
}

fn generate_block_shape(section: &DirectedSection) -> ShapeBundle {
    let mut path_builder = PathBuilder::new();
    path_builder.move_to(section.interpolate_pos(0.0) * LAYOUT_SCALE);

    let num_segments = 10 * section.len();
    let length = section.length();

    for i in 1..(num_segments + 1) {
        let dist = i as f32 * length / num_segments as f32;
        path_builder.line_to(section.interpolate_pos(dist) * LAYOUT_SCALE);
    }

    let path = path_builder.build();

    let shape = ShapeBundle {
        path: path,
        ..Default::default()
    };
    shape
}

fn create_block(
    keyboard_input: Res<Input<keyboard::KeyCode>>,
    mut commands: Commands,
    selection_state: Res<SelectionState>,
    mut entity_map: ResMut<layout::EntityMap>,
    mut marker_map: ResMut<MarkerMap>,
) {
    if let Selection::Section(section) = &selection_state.selection {
        if keyboard_input.just_pressed(keyboard::KeyCode::B) {
            let block = BlockBundle::new(section.clone());
            let block_id = block.block.id;
            let entity = commands.spawn(block).id();
            entity_map.add_block(block_id, entity);
            for logical_id in block_id.logical_block_ids() {
                let in_track = logical_id.default_in_marker_track();
                marker_map
                    .in_markers
                    .try_insert(in_track, logical_id)
                    .unwrap();
                let marker_entity = entity_map
                    .get_entity(&GenericID::Track(in_track.track()))
                    .unwrap();
                commands
                    .entity(marker_entity)
                    .insert(Marker::new(in_track.track(), MarkerColor::Blue));
                entity_map.markers.insert(in_track.track(), marker_entity);
                println!("Adding marker {:?} ", in_track.track());
            }
        }
    }
}

fn update_block_color(
    mut q_strokes: Query<(&Block, &mut Stroke)>,
    selection_state: Res<SelectionState>,
    hover_state: Res<HoverState>,
) {
    if !selection_state.is_changed() && !hover_state.is_changed() {
        return;
    }
    for (block, mut stroke) in q_strokes.iter_mut() {
        if let Some(GenericID::Block(block_id)) = &hover_state.hover {
            if block.id == *block_id {
                stroke.color = Color::RED;
                continue;
            }
        }
        if let Selection::Single(GenericID::Block(block_id)) = &selection_state.selection {
            if block.id == *block_id {
                stroke.color = Color::BLUE;
                continue;
            }
        }
        stroke.color = Color::GREEN;
    }
}

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Block>();
        app.register_component_as::<dyn Selectable, Block>();
        app.add_systems(Update, (create_block, update_block_color));
    }
}
