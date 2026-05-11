use bevy::prelude::*;

use crate::route::AppendLegs;
use crate::train_position::{AdvanceLeg, TrainMarkerHit};

/// Canonical simulation event message. Wraps all mutation event types into a single
/// serializable enum. Logic systems write these; the fan-out system in
/// `SimulationStatePlugin` dispatches them to individual message handlers.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SimulationEvent {
    AppendLegs(AppendLegs),
    TrainMarkerHit(TrainMarkerHit),
    AdvanceLeg(AdvanceLeg),
}

/// Extraction resource: collects `SimulationEvent`s produced by logic systems
/// so they can be forwarded to the client (via SubApp extract or network).
#[derive(Resource, Default)]
pub struct SimulationEventQueue(pub Vec<SimulationEvent>);
