use bevy::prelude::*;

use crate::layout_primitives::{Facing, MarkerColor, TrainID};
use crate::route::{MarkerRole, RouteLeg, RouteLegMarker};

/// A driver-facing route leg. Contains only the information a driver needs:
/// facing direction and ordered marker positions with colors.
/// Deliberately excludes block IDs, track IDs, and section geometry —
/// those are simulation-layer concerns.
#[derive(Clone, Debug)]
pub struct DriverLeg {
    pub facing: Facing,
    pub markers: Vec<DriverMarker>,
}

/// A marker as seen by the driver: color for hardware validation,
/// role for state transitions, position for progress tracking.
#[derive(Clone, Debug)]
pub struct DriverMarker {
    pub color: MarkerColor,
    pub role: Option<MarkerRole>,
    pub position: f32,
}

impl From<&RouteLegMarker> for DriverMarker {
    fn from(m: &RouteLegMarker) -> Self {
        Self {
            color: m.color,
            role: m.role,
            position: m.position,
        }
    }
}

impl From<&RouteLeg> for DriverLeg {
    fn from(leg: &RouteLeg) -> Self {
        Self {
            facing: leg.facing,
            markers: leg.markers.iter().map(DriverMarker::from).collect(),
        }
    }
}

/// Message: queue a driver leg onto a driver for a given train.
/// Sent by the dispatch proxy in the simulation logic layer.
#[derive(Message, Clone)]
pub struct QueueDriverLeg {
    pub train: TrainID,
    pub leg: DriverLeg,
}

impl QueueDriverLeg {
    pub fn new(train: TrainID, leg: DriverLeg) -> Self {
        Self { train, leg }
    }
}

/// Message: a driver reports that its train crossed a marker.
/// Translated by the simulation layer into a `TrainMarkerHit`.
#[derive(Message, Clone)]
pub struct DriverMarkerHit {
    pub train: TrainID,
}

impl DriverMarkerHit {
    pub fn new(train: TrainID) -> Self {
        Self { train }
    }
}
