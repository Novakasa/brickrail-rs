mod app_command;
mod simulation_command;
mod sub_app;

pub use app_command::*;
pub use simulation_command::*;
pub use sub_app::*;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Unique command identifier for correlating requests with responses.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CommandId(pub u64);

/// Lifecycle state of a command entity.
#[derive(Component, Clone, Debug)]
pub enum CommandState {
    Pending,
    Completed,
    Failed(String),
}

/// Response from the simulation world, matched by CommandId.
#[derive(Message, Clone, Debug)]
pub struct CommandResponse {
    pub command_id: CommandId,
    pub result: Result<(), String>,
}

/// Generic envelope pairing a CommandId with a request payload.
/// Used both for transport (wrapping `SimulationCommand` enum) and after
/// fan-out (wrapping concrete request types like `SendTrainToBlockRequest`).
#[derive(Message, Clone, Debug)]
pub struct CommandEnvelope<T: Send + Sync + 'static> {
    pub command_id: CommandId,
    pub request: T,
}

// ---------------------------------------------------------------------------
// CommandRegistry
// ---------------------------------------------------------------------------

/// Registry mapping CommandId → Entity for response matching.
#[derive(Resource, Default)]
pub struct CommandRegistry {
    map: HashMap<CommandId, Entity>,
    next_id: u64,
}

impl CommandRegistry {
    pub fn next_id(&mut self) -> CommandId {
        let id = CommandId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn insert(&mut self, id: CommandId, entity: Entity) {
        self.map.insert(id, entity);
    }

    pub fn get(&self, id: &CommandId) -> Option<Entity> {
        self.map.get(id).copied()
    }

    pub fn remove(&mut self, id: &CommandId) -> Option<Entity> {
        self.map.remove(id)
    }

    /// Issue a simulation command using direct World access and immediately
    /// push it to the SubApp input queue. Useful in tests where you want to
    /// bypass `AppCommandQueue` but still need the command to reach the SubApp.
    pub fn issue_world(world: &mut World, command: SimulationCommand) -> Entity {
        world.resource_scope(|world, mut registry: Mut<CommandRegistry>| {
            let cmd_id = registry.next_id();
            let entity = world
                .spawn((cmd_id, CommandState::Pending))
                .id();
            registry.insert(cmd_id, entity);
            world.resource_mut::<SubAppCommandInputQueue>().0.push(CommandEnvelope {
                command_id: cmd_id,
                request: command,
            });
            entity
        })
    }
}

// ---------------------------------------------------------------------------
// CommandPlugin (client world, transport-agnostic)
// ---------------------------------------------------------------------------

/// Client-side command lifecycle plugin.
/// Registers core command types and handles response application.
pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandRegistry>();
        app.add_message::<CommandResponse>();
        app.add_systems(
            Update,
            apply_command_responses.run_if(on_message::<CommandResponse>),
        );
    }
}

/// Reads CommandResponse messages and updates the corresponding command entity's state.
fn apply_command_responses(
    mut responses: MessageReader<CommandResponse>,
    registry: Res<CommandRegistry>,
    mut commands: Commands,
) {
    for response in responses.read() {
        let Some(entity) = registry.get(&response.command_id) else {
            continue;
        };
        let new_state = match &response.result {
            Ok(()) => CommandState::Completed,
            Err(reason) => CommandState::Failed(reason.clone()),
        };
        commands.entity(entity).insert(new_state);
    }
}
