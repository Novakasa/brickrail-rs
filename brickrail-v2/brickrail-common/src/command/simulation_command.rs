use std::collections::VecDeque;

use bevy::prelude::*;

use crate::layout::Layout;
use crate::layout_primitives::{LogicalBlockID, TrainID};

use super::{CommandEnvelope, CommandId, CommandResponse};

/// Pure domain data for control commands — no command infrastructure.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SimulationCommand {
    EnterControlMode(EnterControlModeRequest),
    PlaceTrainAtBlock(PlaceTrainAtBlockRequest),
    SendTrainToBlock(SendTrainToBlockRequest),
}

/// Domain request: enter control mode with a layout.
/// Spawns all layout elements + a VirtualDriver per train.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EnterControlModeRequest {
    pub layout: Layout,
}

/// Domain request: place a train at a block (creates idle leg).
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlaceTrainAtBlockRequest {
    pub train: TrainID,
    pub block: LogicalBlockID,
}

/// Domain request: send a train to a target block.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SendTrainToBlockRequest {
    pub train: TrainID,
    pub target_block: LogicalBlockID,
}

// ---------------------------------------------------------------------------
// SimulationCommandQueue
// ---------------------------------------------------------------------------

/// Sequential command queue: dispatches commands one at a time, waiting for
/// each to complete before starting the next. All commands enter through
/// `CommandEnvelope<SimulationCommand>` messages (from the extract bridge).
#[derive(Resource, Default)]
pub struct SimulationCommandQueue {
    queue: VecDeque<CommandEnvelope<SimulationCommand>>,
    in_flight: Option<CommandId>,
}

// ---------------------------------------------------------------------------
// SimulationCommandPlugin (simulation world, transport-agnostic)
// ---------------------------------------------------------------------------

/// Simulation-side command handling plugin.
/// Manages the command queue, sequential dispatch with fan-out,
/// and handler systems.
pub struct SimulationCommandPlugin;

impl Plugin for SimulationCommandPlugin {
    fn build(&self, app: &mut App) {
        use crate::simulation::{
            handle_enter_control_mode, handle_place_train_at_block,
            handle_send_train_to_block, SimulationSet,
        };

        app.init_resource::<SimulationCommandQueue>();
        app.add_message::<CommandEnvelope<SimulationCommand>>();
        app.add_message::<CommandEnvelope<EnterControlModeRequest>>();
        app.add_message::<CommandEnvelope<PlaceTrainAtBlockRequest>>();
        app.add_message::<CommandEnvelope<SendTrainToBlockRequest>>();
        app.add_systems(
            Update,
            (
                process_command_queue,
                (
                    handle_enter_control_mode
                        .run_if(on_message::<CommandEnvelope<EnterControlModeRequest>>),
                    handle_place_train_at_block
                        .run_if(on_message::<CommandEnvelope<PlaceTrainAtBlockRequest>>),
                    handle_send_train_to_block
                        .run_if(on_message::<CommandEnvelope<SendTrainToBlockRequest>>),
                ),
            )
                .chain()
                .in_set(SimulationSet::Logic),
        );
    }
}

/// Combined intake + dispatch: reads incoming command messages into the queue,
/// checks if the in-flight command completed, and dispatches the next one.
/// Runs as a single system to avoid ordering issues between intake and dispatch.
fn process_command_queue(
    mut incoming: MessageReader<CommandEnvelope<SimulationCommand>>,
    mut responses: MessageReader<CommandResponse>,
    mut queue: ResMut<SimulationCommandQueue>,
    mut enter_control_writer: MessageWriter<CommandEnvelope<EnterControlModeRequest>>,
    mut place_train_writer: MessageWriter<CommandEnvelope<PlaceTrainAtBlockRequest>>,
    mut send_train_writer: MessageWriter<CommandEnvelope<SendTrainToBlockRequest>>,
) {
    // Intake: queue incoming commands from the extract bridge,
    // preserving their client-assigned CommandIds.
    for envelope in incoming.read() {
        queue.queue.push_back(envelope.clone());
    }

    // Check if the in-flight command has completed.
    if let Some(in_flight_id) = queue.in_flight {
        let completed = responses.read().any(|r| r.command_id == in_flight_id);
        if completed {
            queue.in_flight = None;
        } else {
            return; // Still waiting.
        }
    }

    // Dispatch the next command with fan-out to typed envelope.
    if let Some(envelope) = queue.queue.pop_front() {
        queue.in_flight = Some(envelope.command_id);
        match envelope.request {
            SimulationCommand::EnterControlMode(request) => {
                enter_control_writer.write(CommandEnvelope {
                    command_id: envelope.command_id,
                    request,
                });
            }
            SimulationCommand::PlaceTrainAtBlock(request) => {
                place_train_writer.write(CommandEnvelope {
                    command_id: envelope.command_id,
                    request,
                });
            }
            SimulationCommand::SendTrainToBlock(request) => {
                send_train_writer.write(CommandEnvelope {
                    command_id: envelope.command_id,
                    request,
                });
            }
        }
    }
}
