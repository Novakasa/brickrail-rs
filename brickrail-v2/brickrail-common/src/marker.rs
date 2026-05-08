use crate::layout_primitives::{MarkerColor, TrackID};
use crate::lifecycle::LayoutElement;

/// Marker type for the marker element kind.
#[derive(Clone, Debug)]
pub struct Marker;

/// Layout data for a marker.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkerData {
    pub color: MarkerColor,
}

impl LayoutElement for Marker {
    type ID = TrackID;
    type Data = MarkerData;
}
