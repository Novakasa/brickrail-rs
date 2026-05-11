use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;

use crate::driver::{DriverLeg, DriverMarkerHit, QueueDriverLeg};
use crate::lifecycle::ElementId;
use crate::route::{AppendLegs, LegOf, Locked, RouteLeg, TrainLegs};
use crate::simulation_event::{SimulationEvent, SimulationEventQueue};
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
/// Registers `SimulationEvent` message and fan-out, plus mutation systems.
/// Does NOT include logic systems — those go in `SimulationLogicPlugin`.
pub struct SimulationStatePlugin;

impl Plugin for SimulationStatePlugin {
    fn build(&self, app: &mut App) {
        use crate::route::RouteStatePlugin;
        use crate::train_position::TrainPositionStatePlugin;

        app.configure_sets(Update, SimulationSet::Logic.after(SimulationSet::StateMutation));
        app.add_message::<SimulationEvent>();
        app.add_systems(
            Update,
            fan_out_simulation_events
                .run_if(on_message::<SimulationEvent>)
                .before(SimulationSet::StateMutation),
        );
        app.add_plugins(RouteStatePlugin);
        app.add_plugins(TrainPositionStatePlugin);
    }
}

/// Fan-out: reads `SimulationEvent` messages and dispatches them to individual
/// message types for the modular handlers to consume.
fn fan_out_simulation_events(
    mut event_reader: MessageReader<SimulationEvent>,
    mut append_writer: MessageWriter<AppendLegs>,
    mut marker_hit_writer: MessageWriter<TrainMarkerHit>,
    mut advance_writer: MessageWriter<AdvanceLeg>,
) {
    for event in event_reader.read() {
        match event {
            SimulationEvent::AppendLegs(e) => { append_writer.write(e.clone()); }
            SimulationEvent::TrainMarkerHit(e) => { marker_hit_writer.write(e.clone()); }
            SimulationEvent::AdvanceLeg(e) => { advance_writer.write(e.clone()); }
        }
    }
}

/// Simulation logic plugin. Server-side only.
/// Contains logic systems that react to state and emit `SimulationEvent` messages:
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

/// Simulation event collector plugin. Server-side only.
/// Collects `SimulationEvent` messages into `SimulationEventQueue` for
/// extraction by the client (via SubApp extract or network).
pub struct SimulationCollectorPlugin;

impl Plugin for SimulationCollectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationEventQueue>();
        app.add_systems(
            Update,
            collect_simulation_events
                .run_if(on_message::<SimulationEvent>)
                .in_set(SimulationSet::Logic),
        );
    }
}

/// Collects `SimulationEvent` messages into the extraction queue.
fn collect_simulation_events(
    mut event_reader: MessageReader<SimulationEvent>,
    mut queue: ResMut<SimulationEventQueue>,
) {
    for event in event_reader.read() {
        queue.0.push(event.clone());
    }
}

/// Simulation logic: when a train has fully entered its target block and has
/// a next leg queued, advance to that next leg.
fn advance_leg_logic(
    query: Query<(&TrainPosition, &TrainLegs, &ElementId<Train>)>,
    mut event_writer: MessageWriter<SimulationEvent>,
) {
    for (position, legs, element_id) in &query {
        if position.leg_state == TrainLegState::EnteredTarget && legs.collection().len() > 1 {
            event_writer.write(SimulationEvent::AdvanceLeg(AdvanceLeg::new(element_id.0)));
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
    mut event_writer: MessageWriter<SimulationEvent>,
) {
    for hit in driver_hits.read() {
        event_writer.write(SimulationEvent::TrainMarkerHit(TrainMarkerHit::new(hit.train)));
    }
}
