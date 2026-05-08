use crate::layout_primitives::TrainID;
use crate::lifecycle::LayoutElement;

/// Marker type for the train element kind.
#[derive(Clone, Debug)]
pub struct Train;

/// Layout data for a train.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TrainData {
    pub name: String,
}

impl LayoutElement for Train {
    type ID = TrainID;
    type Data = TrainData;
}
