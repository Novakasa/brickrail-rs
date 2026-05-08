use crate::layout_primitives::{BlockID, DirectedTrackID, TrainSpeed};
use crate::lifecycle::LayoutElement;

/// Marker type for the block element kind.
#[derive(Clone, Debug)]
pub struct Block;

/// Layout data for a block.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BlockData {
    pub name: Option<String>,
    pub section: Vec<DirectedTrackID>,
    pub passthrough_speed: Option<TrainSpeed>,
}

impl LayoutElement for Block {
    type ID = BlockID;
    type Data = BlockData;
}
