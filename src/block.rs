use crate::layout_primitives::*;
use bevy::{prelude::*, utils::HashMap};

#[derive(Component, Debug)]
struct Block {
    id: BlockID,
    logical_blocks: HashMap<LogicalBlockID, Entity>,
}

#[derive(Component, Debug)]
struct LogicalBlock {
    id: LogicalBlockID,
    enter_marks: Vec<LogicalTrackID>,
    in_mark: LogicalTrackID,
    train: Option<TrainID>,
}
