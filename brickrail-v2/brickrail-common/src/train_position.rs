use bevy::prelude::*;

use crate::layout_primitives::*;
use crate::lifecycle::Registry;
use crate::route::{MarkerRole, RouteLeg, TrainLegs};
use crate::simulation::SimulationSet;
use crate::train::Train;
use bevy::ecs::relationship::RelationshipTarget;

/// The train's state within its current leg.
/// Variants explicitly name which block (start or target) they reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TrainLegState {
    /// Train has fully entered the target block (or is idle in a block).
    EnteredTarget,
    /// Train is exiting the start block.
    ExitingStart,
    /// Train is between blocks (after exiting start, before entering target).
    Outside,
    /// Train is entering the target block.
    EnteringTarget,
}

impl TrainLegState {
    /// The leg state when the train is at a marker with the given role.
    /// Exiting → ExitingStart (train is at the start block's exit marker).
    /// Entering → EnteringTarget (train is at the target block's enter marker).
    /// Entered → EnteredTarget (train has fully entered the target block).
    pub fn from_marker_role(role: Option<MarkerRole>) -> Self {
        match role {
            Some(MarkerRole::Exiting) => TrainLegState::ExitingStart,
            Some(MarkerRole::Entering) => TrainLegState::EnteringTarget,
            Some(MarkerRole::Entered) => TrainLegState::EnteredTarget,
            None => TrainLegState::Outside,
        }
    }
}

/// Simulation state: where the train currently is within its leg sequence.
/// The current leg is always the first entry in `TrainLegs`.
/// Inserted by `handle_append_legs` when a train receives its first legs.
#[derive(Component, Clone, Debug)]
pub struct TrainPosition {
    /// The train's state within the current leg.
    pub leg_state: TrainLegState,
    /// Index of the last marker the train passed within the current leg.
    /// Starts at 0 (the first marker, which is the enter/exiting marker — already passed).
    /// Incremented by each TrainMarkerHit.
    pub marker_index: usize,
}

/// State event: a train hit a marker. Increments marker_index and updates leg_state
/// based on the role of the next marker in the current leg.
/// No leg advancement — that's handled by `AdvanceLeg`.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TrainMarkerHit {
    pub train: TrainID,
}

impl TrainMarkerHit {
    pub fn new(train: TrainID) -> Self {
        Self { train }
    }
}

/// State event: advance to the next leg in the queue.
/// Emitted by the logic layer when the train is ready to proceed.
/// Panics if no next leg exists — the strategy layer must always append
/// a trailing idle leg when building a route.
/// Despawns the old leg and resets marker_index to 0.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AdvanceLeg {
    pub train: TrainID,
}

impl AdvanceLeg {
    pub fn new(train: TrainID) -> Self {
        Self { train }
    }
}

/// Train position simulation state sub-plugin.
/// Registers TrainMarkerHit and AdvanceLeg state events.
/// TrainPosition creation is handled by AppendLegs in route.rs.
pub struct TrainPositionStatePlugin;

impl Plugin for TrainPositionStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TrainMarkerHit>();
        app.add_message::<AdvanceLeg>();
        app.add_systems(
            Update,
            (
                handle_train_marker_hit.run_if(on_message::<TrainMarkerHit>),
                handle_advance_leg.run_if(on_message::<AdvanceLeg>),
            )
                .in_set(SimulationSet::StateMutation),
        );
    }
}

/// Mutation system for TrainMarkerHit. Increments marker_index and updates leg_state.
fn handle_train_marker_hit(
    mut messages: MessageReader<TrainMarkerHit>,
    train_registry: Res<Registry<Train>>,
    mut train_position_query: Query<&mut TrainPosition>,
    train_legs_query: Query<&TrainLegs>,
    leg_query: Query<&RouteLeg>,
) {
    for msg in messages.read() {
        let train_entity = train_registry
            .get(&msg.train)
            .expect("TrainMarkerHit: train not found in registry");

        let mut position = train_position_query
            .get_mut(train_entity)
            .expect("TrainMarkerHit: train has no TrainPosition");

        position.marker_index += 1;

        let legs = train_legs_query
            .get(train_entity)
            .expect("TrainMarkerHit: train has no TrainLegs");
        let current_leg_entity = *legs
            .collection()
            .first()
            .expect("TrainMarkerHit: train has no legs");
        let leg = leg_query
            .get(current_leg_entity)
            .expect("TrainMarkerHit: current leg entity not found");

        let marker = &leg.markers[position.marker_index];
        position.leg_state = TrainLegState::from_marker_role(marker.role);
    }
}

/// Mutation system for AdvanceLeg. Despawns the current (first) leg, making the next leg current.
/// Resets marker_index and sets leg_state from the new current leg's first marker.
fn handle_advance_leg(
    mut commands: Commands,
    mut messages: MessageReader<AdvanceLeg>,
    train_registry: Res<Registry<Train>>,
    mut train_position_query: Query<&mut TrainPosition>,
    train_legs_query: Query<&TrainLegs>,
    leg_query: Query<&RouteLeg>,
) {
    for msg in messages.read() {
        let train_entity = train_registry
            .get(&msg.train)
            .expect("AdvanceLeg: train not found in registry");

        let mut position = train_position_query
            .get_mut(train_entity)
            .expect("AdvanceLeg: train has no TrainPosition");

        let legs = train_legs_query
            .get(train_entity)
            .expect("AdvanceLeg: train has no TrainLegs");

        let current_leg_entity = *legs
            .collection()
            .first()
            .expect("AdvanceLeg: train has no legs");
        let next_leg_entity = *legs
            .collection()
            .get(1)
            .expect("AdvanceLeg: no next leg — strategy layer must append trailing idle leg");

        let next_leg = leg_query
            .get(next_leg_entity)
            .expect("AdvanceLeg: next leg entity not found");

        position.leg_state = TrainLegState::from_marker_role(next_leg.markers[0].role);
        position.marker_index = 0;

        commands.entity(current_leg_entity).despawn();
    }
}
