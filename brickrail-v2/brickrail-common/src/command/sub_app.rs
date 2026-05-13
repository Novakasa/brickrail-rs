use bevy::prelude::*;

use crate::simulation::SimulationSet;
use crate::simulation_event::{SimulationEvent, SimulationEventQueue};

use super::{
    CommandEnvelope, CommandId, CommandResponse, CommandState, Dispatched,
    SimulationCommand, SimulationCommandPayload,
};

// ---------------------------------------------------------------------------
// SubApp queue resources
// ---------------------------------------------------------------------------

/// Typed input queue: enveloped commands waiting to be sent to simulation.
/// Lives in the client world, drained by the extract bridge.
#[derive(Resource, Default)]
pub struct SubAppCommandInputQueue(pub Vec<CommandEnvelope<SimulationCommand>>);

/// Extraction resource: collects CommandResponses in the simulation SubApp
/// for forwarding to the client.
#[derive(Resource, Default)]
pub struct SubAppCommandResponseQueue(pub Vec<CommandResponse>);

// ---------------------------------------------------------------------------
// SubAppClientPlugin (client-side SubApp transport)
// ---------------------------------------------------------------------------

/// Client-side SubApp transport plugin.
/// Dispatches pending simulation commands to the SubAppCommandInputQueue
/// and sets up command infrastructure for the extract bridge.
pub struct SubAppClientPlugin;

impl Plugin for SubAppClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SubAppCommandInputQueue>();
        app.add_systems(Update, dispatch_simulation_commands);
    }
}

/// Dispatches pending SimulationCommandPayload entities to the SubApp input queue.
fn dispatch_simulation_commands(
    mut commands: Commands,
    query: Query<
        (Entity, &CommandId, &SimulationCommandPayload),
        (With<CommandState>, Without<Dispatched>),
    >,
    mut cmd_queue: ResMut<SubAppCommandInputQueue>,
) {
    for (entity, cmd_id, payload) in &query {
        cmd_queue.0.push(CommandEnvelope {
            command_id: *cmd_id,
            request: payload.0.clone(),
        });
        commands.entity(entity).insert(Dispatched);
    }
}

// ---------------------------------------------------------------------------
// SubAppServerPlugin (simulation-side SubApp transport)
// ---------------------------------------------------------------------------

/// Simulation-side SubApp transport plugin.
/// Collects SimulationEvent and CommandResponse messages into queues
/// for extraction by the client.
pub struct SubAppServerPlugin;

impl Plugin for SubAppServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationEventQueue>();
        app.init_resource::<SubAppCommandResponseQueue>();
        app.add_message::<CommandResponse>();
        app.add_systems(
            Update,
            (
                collect_simulation_events.run_if(on_message::<SimulationEvent>),
                collect_command_responses.run_if(on_message::<CommandResponse>),
            )
                .after(SimulationSet::Logic),
        );
    }
}

/// Collects SimulationEvent messages into the extraction queue.
fn collect_simulation_events(
    mut event_reader: MessageReader<SimulationEvent>,
    mut queue: ResMut<SimulationEventQueue>,
) {
    for event in event_reader.read() {
        queue.0.push(event.clone());
    }
}

/// Collects CommandResponse messages into the extraction queue.
fn collect_command_responses(
    mut response_reader: MessageReader<CommandResponse>,
    mut queue: ResMut<SubAppCommandResponseQueue>,
) {
    for response in response_reader.read() {
        queue.0.push(response.clone());
    }
}
