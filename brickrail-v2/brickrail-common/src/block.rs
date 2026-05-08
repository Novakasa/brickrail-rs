use crate::layout_primitives::{BlockID, DirectedTrackID, TrainSpeed};
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

impl LayoutElement for Block {
    type ID = BlockID;
    type Data = BlockData;
}
