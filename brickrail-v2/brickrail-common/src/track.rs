use serde::{Deserialize, Serialize};

use crate::layout_primitives::TrackID;
use crate::lifecycle::LayoutElement;

/// Marker type for the track element kind. Not a component itself.
#[derive(Clone, Debug)]
pub struct Track;

/// Layout data for a track segment. Currently empty — all identity is in the ID.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrackData;

impl LayoutElement for Track {
    type ID = TrackID;
    type Data = TrackData;
}
