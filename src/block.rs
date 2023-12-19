use crate::editor::{GenericID, HoverState, Selectable, Selection, SelectionState};
use crate::layout;
use crate::section::LogicalSection;
use crate::{layout_primitives::*, section::DirectedSection, track::LAYOUT_SCALE};
use bevy::input::keyboard;
use bevy::{prelude::*, utils::HashMap};
use bevy_prototype_lyon::{
    draw::Stroke,
    entity::ShapeBundle,
    path::PathBuilder,
    prelude::{LineCap, StrokeOptions},
};

pub const BLOCK_WIDTH: f32 = 20.0;

#[derive(Debug, Reflect, Default)]
pub struct BlockSettings {
    pub name: String,
    pub length: f32,
}

#[derive(Component, Debug, Reflect)]
pub struct Block {
    pub id: BlockID,
    logical_blocks: HashMap<LogicalBlockID, Entity>,
    section: DirectedSection,
    pub settings: BlockSettings,
}

impl Block {
    pub fn distance_to(&self, pos: Vec2) -> f32 {
        self.section.distance_to(pos)
    }
}

#[derive(Bundle)]
pub struct BlockBundle {
    shape: ShapeBundle,
    stroke: Stroke,
    block: Block,
    selectable: Selectable,
}

impl BlockBundle {
    pub fn new(section: DirectedSection) -> Self {
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
        let stroke = Stroke {
            color: Color::GREEN,
            options: StrokeOptions::default()
                .with_line_width(BLOCK_WIDTH)
                .with_line_cap(LineCap::Round),
        };

        Self {
            shape: shape,
            stroke: stroke,
            selectable: Selectable::new(
                crate::editor::GenericID::Block(section.to_block_id()),
                0.0,
            ),
            block: Block {
                id: section.to_block_id(),
                logical_blocks: HashMap::new(),
                section: section,
                settings: BlockSettings::default(),
            },
        }
    }
}

#[derive(Component, Debug)]
pub struct LogicalBlock {
    pub id: LogicalBlockID,
    enter_marks: Vec<LogicalTrackID>,
    in_mark: LogicalTrackID,
    train: Option<TrainID>,
    section: LogicalSection,
}

fn create_block(
    keyboard_input: Res<Input<keyboard::KeyCode>>,
    mut commands: Commands,
    selection_state: Res<SelectionState>,
    mut layout: ResMut<layout::Layout>,
) {
    if let Selection::Section(section) = &selection_state.selection {
        if keyboard_input.just_pressed(keyboard::KeyCode::B) {
            let block = BlockBundle::new(section.clone());
            let block_id = block.block.id;
            let entity = commands.spawn(block).id();
            layout.add_block(block_id, entity)
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
        app.add_systems(Update, (create_block, update_block_color));
    }
}
