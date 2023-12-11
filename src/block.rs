use crate::{layout_primitives::*, section::DirectedSection};
use bevy::{prelude::*, utils::HashMap};
use bevy_prototype_lyon::{
    draw::Stroke,
    entity::ShapeBundle,
    path::PathBuilder,
    prelude::{LineCap, StrokeOptions},
};

#[derive(Component, Debug)]
struct Block {
    id: BlockID,
    logical_blocks: HashMap<LogicalBlockID, Entity>,
}

#[derive(Bundle)]
pub struct BlockBundle {
    shape: ShapeBundle,
    stroke: Stroke,
    block: Block,
}

impl BlockBundle {
    pub fn new(section: DirectedSection) -> Self {
        let mut path_builder = PathBuilder::new();
        path_builder.move_to(section.interpolate_pos(0.0) * 40.0);

        let num_segments = 10 * section.len();
        let length = section.length();

        for i in 1..(num_segments + 1) {
            let dist = i as f32 * length / num_segments as f32;
            path_builder.line_to(section.interpolate_pos(dist) * 40.0);
        }

        let path = path_builder.build();

        let shape = ShapeBundle {
            path: path,
            ..Default::default()
        };
        let stroke = Stroke {
            color: Color::GREEN,
            options: StrokeOptions::default()
                .with_line_width(20.0)
                .with_line_cap(LineCap::Round),
        };

        Self {
            shape: shape,
            stroke: stroke,
            block: Block {
                id: section.to_block_id(),
                logical_blocks: HashMap::new(),
            },
        }
    }
}

#[derive(Component, Debug)]
struct LogicalBlock {
    id: LogicalBlockID,
    enter_marks: Vec<LogicalTrackID>,
    in_mark: LogicalTrackID,
    train: Option<TrainID>,
}
