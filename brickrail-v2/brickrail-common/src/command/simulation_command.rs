use bevy::prelude::*;

use crate::layout_primitives::{LogicalBlockID, TrainID};

use super::CommandEnvelope;

/// Pure domain data for control commands — no command infrastructure.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SimulationCommand {
    SendTrainToBlock(SendTrainToBlockRequest),
}

/// Domain request: send a train to a target block.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SendTrainToBlockRequest {
    pub train: TrainID,
    pub target_block: LogicalBlockID,
}

/// Component: wraps a SimulationCommand for dispatch on the client side.
#[derive(Component, Clone, Debug)]
pub struct SimulationCommandPayload(pub SimulationCommand);

// ---------------------------------------------------------------------------
// SimulationCommandPlugin (simulation world, transport-agnostic)
// ---------------------------------------------------------------------------

/// Simulation-side command handling plugin.
/// Registers `CommandEnvelope<SimulationCommand>` message, fans out to typed envelopes,
/// and contains handler systems.
pub struct SimulationCommandPlugin;

impl Plugin for SimulationCommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CommandEnvelope<SimulationCommand>>();
        app.add_message::<CommandEnvelope<SendTrainToBlockRequest>>();
        app.add_systems(
            Update,
            fan_out_simulation_commands.run_if(on_message::<CommandEnvelope<SimulationCommand>>),
        );
    }
}

/// Fan-out: unwraps `CommandEnvelope<SimulationCommand>` into typed `CommandEnvelope<T>` messages.
fn fan_out_simulation_commands(
    mut reader: MessageReader<CommandEnvelope<SimulationCommand>>,
    mut send_train_writer: MessageWriter<CommandEnvelope<SendTrainToBlockRequest>>,
) {
    for envelope in reader.read() {
        match envelope.request.clone() {
            SimulationCommand::SendTrainToBlock(request) => {
                send_train_writer.write(CommandEnvelope {
                    command_id: envelope.command_id,
                    request,
                });
            }
        }
    }
}
