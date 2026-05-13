use bevy::app::{Main, MainSchedulePlugin};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use crate::layout::LayoutSubApp;
use crate::simulation::SimulationPlugin;
use crate::simulation_event::{SimulationEvent, SimulationEventQueue};

use super::{CommandEnvelope, CommandResponse, SimulationCommand, SimulationCommandPlugin};

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
/// Creates the simulation SubApp, wires up the bidirectional extract bridge,
/// and inserts it into the main app.
pub struct SubAppClientPlugin;

impl Plugin for SubAppClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SubAppCommandInputQueue>();

        let mut sub_app = SubApp::new();
        sub_app.update_schedule = Some(Main.intern());

        // Bootstrap SubApp with standard Bevy schedules and message plumbing.
        sub_app.add_plugins(MainSchedulePlugin);
        sub_app.add_systems(
            First,
            bevy::ecs::message::message_update_system
                .in_set(bevy::ecs::message::MessageUpdateSystems)
                .run_if(bevy::ecs::message::message_update_condition),
        );
        sub_app.init_resource::<bevy::ecs::reflect::AppTypeRegistry>();

        // Domain logic (communication-agnostic).
        sub_app.add_plugins(SimulationPlugin);

        // Command handling + response collection (transport-specific).
        sub_app.add_plugins(SimulationCommandPlugin);
        sub_app.add_plugins(SubAppServerPlugin);

        // Bidirectional extract bridge.
        sub_app.set_extract(|main_world, sub_world| {
            // Output: simulation events → main app.
            let mut event_queue = sub_world.resource_mut::<SimulationEventQueue>();
            let events: Vec<SimulationEvent> = event_queue.0.drain(..).collect();
            for event in events {
                main_world.write_message(event);
            }

            // Output: command responses → main app.
            let mut response_queue = sub_world.resource_mut::<SubAppCommandResponseQueue>();
            let responses: Vec<CommandResponse> = response_queue.0.drain(..).collect();
            for response in responses {
                main_world.write_message(response);
            }

            // Input: simulation commands → SubApp.
            let mut cmd_queue = main_world.resource_mut::<SubAppCommandInputQueue>();
            let commands: Vec<_> = cmd_queue.0.drain(..).collect();
            for cmd in commands {
                sub_world.write_message(cmd);
            }
        });

        app.insert_sub_app(LayoutSubApp, sub_app);
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
                .after(crate::simulation::SimulationSet::Logic),
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
