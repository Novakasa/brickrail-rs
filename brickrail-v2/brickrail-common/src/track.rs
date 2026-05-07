use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::layout_primitives::TrackID;
use crate::lifecycle::LayoutElement;

/// Layout data component for a track segment.
#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackID,
}

impl Track {
    pub fn new(id: TrackID) -> Self {
        Self { id }
    }
}

impl LayoutElement for Track {
    type ID = TrackID;

    fn id(&self) -> TrackID {
        self.id
    }
}
