use bevy::prelude::*;
use bevy_prototype_lyon::draw::Stroke;
use serde::{Deserialize, Serialize};

use crate::{
    editor::{GenericID, Selectable},
    layout_primitives::{LayoutDeviceID, TrackID},
    track::{LAYOUT_SCALE, TRACK_WIDTH},
};

pub enum CrossingPosition {
    Open,
    Closed,
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Component)]
pub struct LevelCrossing {
    id: TrackID,
    pub motors: Vec<Option<LayoutDeviceID>>,
}

impl Selectable for LevelCrossing {
    type ID = TrackID;
    type SpawnEvent = SpawnCrossingEvent;

    fn id(&self) -> Self::ID {
        self.id
    }

    fn generic_id(&self) -> crate::editor::GenericID {
        GenericID::Crossing(self.id)
    }

    fn get_depth(&self) -> f32 {
        1.5
    }

    fn get_distance(
        &self,
        pos: Vec2,
        _transform: Option<&Transform>,
        _stroke: Option<&Stroke>,
    ) -> f32 {
        self.id.distance_to(pos) - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
    }
}

#[derive(Serialize, Deserialize, Clone, Event)]
pub struct SpawnCrossingEvent {
    pub switch: LevelCrossing,
    pub name: Option<String>,
}
