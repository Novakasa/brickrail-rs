use crate::layout_primitives::{
    BlockDirection, BlockID, DirectedTrackID, Facing, LogicalTrackID, TrainSpeed,
};
use crate::lifecycle::LayoutElement;

/// Marker type for the block element kind.
#[derive(Clone, Debug)]
pub struct Block;

/// Per-travel-direction configuration for a block.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DirectedBlockConfig {
    pub passthrough_speed: Option<TrainSpeed>,
}

/// Layout data for a block.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BlockData {
    pub name: Option<String>,
    pub section: Vec<DirectedTrackID>,
    /// Config for travel aligned with the section direction.
    pub aligned: DirectedBlockConfig,
    /// Config for travel against the section direction.
    pub against: DirectedBlockConfig,
}

impl BlockData {
    /// Section in travel order for a given direction.
    /// Aligned returns the stored section as-is; Against reverses and flips directions.
    pub fn directed_section(&self, direction: BlockDirection) -> Vec<DirectedTrackID> {
        match direction {
            BlockDirection::Aligned => self.section.clone(),
            BlockDirection::Against => self.section.iter().rev().map(|d| d.opposite()).collect(),
        }
    }

    /// The canonical enter marker track for a given direction and facing.
    /// This is the track where the train's sensor crosses when entering the block.
    pub fn enter_logical_track(&self, direction: BlockDirection, facing: Facing) -> LogicalTrackID {
        // Aligned + Forward → B (last), Aligned + Backward → A (first)
        // Against + Forward → A (first, opposite dir), Against + Backward → B (last, opposite dir)
        match (direction, facing) {
            (BlockDirection::Aligned, Facing::Forward) => {
                LogicalTrackID::new(*self.section.last().unwrap(), Facing::Forward)
            }
            (BlockDirection::Aligned, Facing::Backward) => {
                LogicalTrackID::new(*self.section.first().unwrap(), Facing::Backward)
            }
            (BlockDirection::Against, Facing::Forward) => {
                LogicalTrackID::new(self.section.first().unwrap().opposite(), Facing::Forward)
            }
            (BlockDirection::Against, Facing::Backward) => {
                LogicalTrackID::new(self.section.last().unwrap().opposite(), Facing::Backward)
            }
        }
    }
}

impl LayoutElement for Block {
    type ID = BlockID;
    type Data = BlockData;
}
