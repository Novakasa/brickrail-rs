use bevy::prelude::*;

use crate::simulation::SimulationSet;
use crate::simulation_event::{SimulationEvent, SimulationEventQueue};

use super::{CommandEnvelope, CommandResponse, SimulationCommand};

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
/// Sets up the SubAppCommandInputQueue for the extract bridge.
/// Command dispatch is handled by `AppCommandPlugin`.
pub struct SubAppClientPlugin;

impl Plugin for SubAppClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SubAppCommandInputQueue>();
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
