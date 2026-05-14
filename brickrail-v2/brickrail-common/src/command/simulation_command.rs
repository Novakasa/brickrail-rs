use std::collections::VecDeque;

use bevy::prelude::*;

use crate::layout::Layout;
use crate::primitives::{LogicalBlockID, TrainID};

use super::{CommandEnvelope, CommandId, CommandResponse};

/// Pure domain data for control commands — no command infrastructure.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SimulationCommand {
    EnterControlMode(EnterControlModeRequest),
    SendTrainToBlock(SendTrainToBlockRequest),
    ExitControlMode(ExitControlModeRequest),
}

/// Domain request: enter control mode with a layout and optional cached train positions.
/// The handler stores this data and transitions to `Entering` state; dedicated
/// state-driven systems handle spawning and train placement.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EnterControlModeRequest {
    pub layout: Layout,
    #[serde(default)]
    pub train_positions: Vec<(TrainID, LogicalBlockID)>,
}

/// Domain request: send a train to a target block.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SendTrainToBlockRequest {
    pub train: TrainID,
    pub target_block: LogicalBlockID,
}

/// Domain request: exit control mode by despawning all layout elements.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ExitControlModeRequest;

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
            SimulationSet, SimulationState, handle_enter_control_mode, handle_exit_control_mode,
            handle_send_train_to_block,
        };

        app.init_resource::<SimulationCommandQueue>();
        app.add_message::<CommandEnvelope<SimulationCommand>>();
        app.add_message::<CommandEnvelope<EnterControlModeRequest>>();
        app.add_message::<CommandEnvelope<SendTrainToBlockRequest>>();
        app.add_message::<CommandEnvelope<ExitControlModeRequest>>();
        app.add_systems(
            Update,
            (
                intake_commands.run_if(on_message::<CommandEnvelope<SimulationCommand>>),
                // Response clearing must always run so responses aren't lost
                // while dispatch is gated during Entering state.
                clear_completed_commands.run_if(on_message::<CommandResponse>),
                dispatch_commands
                    .run_if(in_state(SimulationState::Idle).or(in_state(SimulationState::Running))),
                (
                    handle_enter_control_mode
                        .run_if(on_message::<CommandEnvelope<EnterControlModeRequest>>),
                    handle_send_train_to_block
                        .run_if(on_message::<CommandEnvelope<SendTrainToBlockRequest>>),
                    handle_exit_control_mode
                        .run_if(on_message::<CommandEnvelope<ExitControlModeRequest>>),
                ),
            )
                .chain()
                .in_set(SimulationSet::Logic),
        );
    }
}

/// Intake: reads incoming command messages into the queue,
/// preserving their client-assigned CommandIds.
fn intake_commands(
    mut incoming: MessageReader<CommandEnvelope<SimulationCommand>>,
    mut queue: ResMut<SimulationCommandQueue>,
) {
    for envelope in incoming.read() {
        queue.queue.push_back(envelope.clone());
    }
}

/// Clears the in-flight command when its response arrives.
/// Runs unconditionally so responses aren't lost while dispatch is gated
/// during transitional states like Entering.
fn clear_completed_commands(
    mut responses: MessageReader<CommandResponse>,
    mut queue: ResMut<SimulationCommandQueue>,
) {
    if let Some(in_flight_id) = queue.in_flight {
        if responses.read().any(|r| r.command_id == in_flight_id) {
            queue.in_flight = None;
        }
    }
}

/// Dispatch: pops the next command from the queue and fans out to typed
/// envelopes. Gated on SimulationState (only runs in Idle or Running).
fn dispatch_commands(
    mut queue: ResMut<SimulationCommandQueue>,
    mut enter_control_writer: MessageWriter<CommandEnvelope<EnterControlModeRequest>>,
    mut send_train_writer: MessageWriter<CommandEnvelope<SendTrainToBlockRequest>>,
    mut exit_control_writer: MessageWriter<CommandEnvelope<ExitControlModeRequest>>,
) {
    // Wait for in-flight command to complete.
    if queue.in_flight.is_some() {
        return;
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
            SimulationCommand::SendTrainToBlock(request) => {
                send_train_writer.write(CommandEnvelope {
                    command_id: envelope.command_id,
                    request,
                });
            }
            SimulationCommand::ExitControlMode(request) => {
                exit_control_writer.write(CommandEnvelope {
                    command_id: envelope.command_id,
                    request,
                });
            }
        }
    }
}
