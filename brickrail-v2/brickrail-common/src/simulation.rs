use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;

use crate::driver::{DriverLeg, DriverMarkerHit, QueueDriverLeg};
use crate::lifecycle::ElementId;
use crate::route::{LegOf, Locked, RouteLeg, TrainLegs};
use crate::train::Train;
use crate::train_position::{AdvanceLeg, TrainLegState, TrainMarkerHit, TrainPosition};

/// System sets for ordering simulation systems within `Update`.
/// State mutation runs first (processing messages), then logic reacts to the new state.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimulationSet {
    /// Systems that mutate state in response to messages (e.g. handle_train_marker_hit).
    StateMutation,
    /// Systems that read state and emit new messages (e.g. advance_leg_logic).
    Logic,
}

/// Simulation state plugin. Shared by both server and client.
/// Registers message types, mutation systems, and system set ordering.
/// Does NOT include logic systems — those go in `SimulationLogicPlugin`.
pub struct SimulationStatePlugin;

impl Plugin for SimulationStatePlugin {
    fn build(&self, app: &mut App) {
        use crate::route::RouteStatePlugin;
        use crate::train_position::TrainPositionStatePlugin;

        app.configure_sets(Update, SimulationSet::Logic.after(SimulationSet::StateMutation));
        app.add_plugins(RouteStatePlugin);
        app.add_plugins(TrainPositionStatePlugin);
    }
}

/// Simulation logic plugin. Server-side only.
/// Contains logic systems that react to state and emit new messages:
/// leg advancement, driver dispatch, and driver↔simulation translation.
/// Requires `SimulationStatePlugin`.
pub struct SimulationLogicPlugin;

impl Plugin for SimulationLogicPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<QueueDriverLeg>();
        app.add_message::<DriverMarkerHit>();
        app.add_observer(dispatch_locked_leg);
        app.add_systems(
            Update,
            (
                advance_leg_logic,
                translate_driver_marker_hit.run_if(on_message::<DriverMarkerHit>),
            )
                .in_set(SimulationSet::Logic),
        );
    }
}

/// Simulation logic: when a train has fully entered its target block and has
/// a next leg queued, advance to that next leg.
fn advance_leg_logic(
    query: Query<(&TrainPosition, &TrainLegs, &ElementId<Train>)>,
    mut advance_writer: MessageWriter<AdvanceLeg>,
) {
    for (position, legs, element_id) in &query {
        if position.leg_state == TrainLegState::EnteredTarget && legs.collection().len() > 1 {
            advance_writer.write(AdvanceLeg::new(element_id.0));
        }
    }
}

/// Observer: when a leg gets `Locked`, dispatch it to the driver.
/// Converts the `RouteLeg` to a `DriverLeg` and sends `QueueDriverLeg`.
fn dispatch_locked_leg(
    trigger: On<Add, Locked>,
    leg_query: Query<(&RouteLeg, &LegOf)>,
    train_id_query: Query<&ElementId<Train>>,
    mut queue_writer: MessageWriter<QueueDriverLeg>,
) {
    let leg_entity = trigger.event().entity;
    let Ok((leg, leg_of)) = leg_query.get(leg_entity) else {
        return;
    };
    let Ok(train_element_id) = train_id_query.get(leg_of.0) else {
        return;
    };
    queue_writer.write(QueueDriverLeg::new(train_element_id.0, DriverLeg::from(leg)));
}

/// Translation: converts `DriverMarkerHit` from the driver layer into
/// `TrainMarkerHit` for the simulation state layer.
fn translate_driver_marker_hit(
    mut driver_hits: MessageReader<DriverMarkerHit>,
    mut sim_hits: MessageWriter<TrainMarkerHit>,
) {
    for hit in driver_hits.read() {
        sim_hits.write(TrainMarkerHit::new(hit.train));
    }
}
