# Command Layer Design

## Context

The system needs a formal command layer — the canonical entry point for all state mutations from GUI, tests, and scripts. Currently, mutations happen ad-hoc: `SpawnElement` messages for layout changes, `SimulationEvent` for simulation state, and direct `write_message` calls into the SubApp for control (e.g. `EnterControlMode`).

The command layer introduces:
- Trackable lifecycle per command (Pending → Completed/Failed)
- A formal input channel from client → simulation (control commands)
- Headless testability (property tests issue commands directly)

We'll implement with a concrete first command — `SendTrainToBlock` — to validate the pattern.

---

## Design: Command entities in client only, typed enum for simulation requests

Command entities live only in the client world. Control commands that need simulation processing use a `SimulationCommand` enum (mirroring `SimulationEvent`) carried via a typed queue resource. The simulation fans it out to individual message handlers, processes them, and writes `CommandResponse` to a response queue. Extract carries both directions.

A `Dispatched` marker component (not a CommandState variant) is added to control command entities after their request is queued, preventing re-dispatch on subsequent frames. Layout commands don't get this marker — they resolve immediately in the client world.

---

## Step 1: Core command types in `brickrail-common/src/command.rs`

```rust
/// Unique command identifier for correlating requests with responses.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CommandId(pub u64);

/// Lifecycle state of a command entity.
#[derive(Component, Clone, Debug)]
pub enum CommandState {
    Pending,
    Completed,
    Failed(String),
}

/// Marker: this control command's request has been queued for the simulation.
#[derive(Component)]
pub struct Dispatched;

/// Response from the simulation world, matched by CommandId.
#[derive(Message, Clone, Debug)]
pub struct CommandResponse {
    pub command_id: CommandId,
    pub result: Result<(), String>,
}

/// Extraction resource: collects responses for forwarding to client.
#[derive(Resource, Default)]
pub struct SubAppCommandResponseQueue(pub Vec<CommandResponse>);

/// Envelope: pairs a CommandId with a SimulationCommand for correlation.
#[derive(Clone, Debug)]
pub struct SimulationCommandEnvelope {
    pub command_id: CommandId,
    pub command: SimulationCommand,
}

/// Typed input queue: enveloped commands waiting to be sent to simulation.
/// Mirroring SimulationEventQueue pattern.
#[derive(Resource, Default)]
pub struct SubAppCommandInputQueue(pub Vec<SimulationCommandEnvelope>);

/// Registry mapping CommandId → Entity for response matching.
#[derive(Resource, Default)]
pub struct CommandRegistry {
    map: HashMap<CommandId, Entity>,
    next_id: u64,
}
```

## Step 2: `SimulationCommand` enum, `CommandEnvelope<T>`, and fan-out

The enum wraps concrete request types. Each request type is pure domain data — no `CommandId`:

```rust
/// Pure domain data for control commands — no command infrastructure.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SimulationCommand {
    SendTrainToBlock(SendTrainToBlockRequest),
}

/// Domain request: send a train to a target block.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SendTrainToBlockRequest {
    pub train: TrainID,
    pub target_block: LogicalBlockID,
}
```

The fan-out produces typed `CommandEnvelope<T>` messages — a generic envelope that pairs any request with a `CommandId`:

```rust
/// Generic envelope: pairs a CommandId with a typed request.
/// Used as a Message so handlers receive both the request and the correlation ID.
#[derive(Message, Clone, Debug)]
pub struct CommandEnvelope<T: Send + Sync + 'static> {
    pub command_id: CommandId,
    pub request: T,
}
```

Fan-out system in simulation world:

```rust
fn fan_out_simulation_commands(
    mut reader: MessageReader<SimulationCommandEnvelope>,
    mut send_train_writer: MessageWriter<CommandEnvelope<SendTrainToBlockRequest>>,
) {
    for envelope in reader.read() {
        match envelope.command {
            SimulationCommand::SendTrainToBlock(request) => {
                send_train_writer.write(CommandEnvelope {
                    command_id: envelope.command_id,
                    request,
                });
            }
        }
    }
}
```

## Step 3: Plugins

### Transport-agnostic (core logic)

**`CommandPlugin`** — added to the client world:
- Inits `CommandRegistry` resource
- Registers `CommandResponse` message
- System: `apply_command_responses` — reads `CommandResponse` messages, looks up command entity via `CommandRegistry`, updates `CommandState`

**`SimulationCommandPlugin`** — added to the simulation world (SubApp or server):
- Registers `SimulationCommandEnvelope` message, `CommandEnvelope<T>` messages for each request type
- System: `fan_out_simulation_commands` (before handler systems)
- Individual handler systems (e.g. `handle_send_train_to_block`)

These plugins work regardless of transport. They consume/produce messages — they don't know how messages arrive or leave.

### SubApp transport

**`SubAppServerPlugin`** — added to the simulation SubApp:
- Inits `SubAppCommandResponseQueue` and `SimulationEventQueue` resources
- Systems: collects `CommandResponse` and `SimulationEvent` messages into their respective queues

**`SubAppClientPlugin`** — added to the client world:
- Inits `SubAppCommandInputQueue` resource
- System: `dispatch_simulation_commands` — reads pending `SimulationCommandPayload` entities, pushes envelopes to queue, adds `Dispatched`
- Sets up the extract bridge that drains all queues bidirectionally

### Future: Network transport

**`NetworkServerPlugin`** — replaces `SubAppServerPlugin` on simulation side. Sends `SimulationEvent` + `CommandResponse` over wire.

**`NetworkClientPlugin`** — replaces `SubAppClientPlugin` on client side. Sends commands over wire, receives events + responses from wire.

The core plugins (`CommandPlugin`, `SimulationCommandPlugin`, `SimulationLogicPlugin`) stay identical.

## Step 4: Bidirectional extract bridge

```rust
sub_app.set_extract(|main_world, sub_world| {
    // Output: simulation events (existing)
    let mut event_queue = sub_world.resource_mut::<SimulationEventQueue>();
    for event in event_queue.0.drain(..) {
        main_world.write_message(event);
    }

    // Output: command responses (new)
    let mut response_queue = sub_world.resource_mut::<SubAppCommandResponseQueue>();
    for response in response_queue.0.drain(..) {
        main_world.write_message(response);
    }

    // Input: simulation command envelopes (new)
    let mut cmd_queue = main_world.resource_mut::<SubAppCommandInputQueue>();
    let envelopes: Vec<_> = cmd_queue.0.drain(..).collect();
    for envelope in envelopes {
        sub_world.write_message(envelope);
    }
});
```

## Step 5: Implement `SendTrainToBlock`

**Client side — generic dispatch handler (handles all simulation commands):**

A single `SimulationCommandPayload` component wraps the `SimulationCommand` enum. One dispatch system forwards all of them:

```rust
/// Component: wraps a SimulationCommand for dispatch.
#[derive(Component, Clone, Debug)]
pub struct SimulationCommandPayload(pub SimulationCommand);

/// Single dispatch system for all simulation commands.
fn dispatch_simulation_commands(
    mut commands: Commands,
    query: Query<(Entity, &CommandId, &SimulationCommandPayload), Without<Dispatched>>,
    mut cmd_queue: ResMut<SubAppCommandInputQueue>,
) {
    for (entity, cmd_id, payload) in &query {
        cmd_queue.0.push(SimulationCommandEnvelope {
            command_id: *cmd_id,
            command: payload.0.clone(),
        });
        commands.entity(entity).insert(Dispatched);
    }
}
```

**`Commands` extension for issuing commands:**

```rust
pub trait CommandsExt {
    fn issue_command(
        &mut self,
        registry: &mut CommandRegistry,
        command: SimulationCommand,
    ) -> Entity;
}

impl CommandsExt for Commands<'_, '_> {
    fn issue_command(
        &mut self,
        registry: &mut CommandRegistry,
        command: SimulationCommand,
    ) -> Entity {
        let cmd_id = registry.next_id();
        let entity = self.spawn((
            cmd_id,
            CommandState::Pending,
            SimulationCommandPayload(command),
        )).id();
        registry.insert(cmd_id, entity);
        entity
    }
}
```

Issuing a control command from UI or test:

```rust
let entity = commands.issue_command(
    &mut registry,
    SimulationCommand::SendTrainToBlock {
        train: TrainID(0),
        target_block: logical_b,
    },
);
```

**Simulation side — handler receives `CommandEnvelope<SendTrainToBlockRequest>`:**

```rust
fn handle_send_train_to_block(
    mut messages: MessageReader<CommandEnvelope<SendTrainToBlockRequest>>,
    logical_graph: Res<LogicalGraph>,
    block_registry: Res<Registry<Block>>,
    block_data_query: Query<&ElementData<Block>>,
    marker_registry: Res<Registry<Marker>>,
    marker_data_query: Query<&ElementData<Marker>>,
    train_registry: Res<Registry<Train>>,
    train_query: Query<(&TrainPosition, &TrainLegs)>,
    leg_query: Query<&RouteLeg>,
    mut event_writer: MessageWriter<SimulationEvent>,
    mut response_queue: ResMut<SubAppCommandResponseQueue>,
) {
    for envelope in messages.read() {
        let request = &envelope.request;
        // 1. Get train's current block from its position + current leg
        // 2. A* pathfind from current block to target block
        // 3. Build route legs + trailing idle
        // 4. Write SimulationEvent::AppendLegs
        // 5. Push CommandResponse with Ok or Err

        response_queue.0.push(CommandResponse {
            command_id: envelope.command_id,
            result,  // Ok(()) or Err("no path found".into())
        });
    }
}
```

## Step 6: Layout commands (example)

Layout commands share `CommandId` and `CommandState` but resolve immediately in the client world — no `Dispatched`, no simulation bridge:

```rust
#[derive(Component, Clone, Debug)]
pub struct AddTrack {
    pub track_id: TrackID,
}

fn handle_add_track(
    mut commands: Commands,
    query: Query<(Entity, &AddTrack, &CommandState)>,
    mut spawn_writer: MessageWriter<SpawnElement<Track>>,
) {
    for (entity, payload, state) in &query {
        if !matches!(state, CommandState::Pending) { continue; }
        spawn_writer.write(SpawnElement::new(payload.track_id, Default::default()));
        commands.entity(entity).insert(CommandState::Completed);
    }
}
```

## Frame-by-frame flow for `SendTrainToBlock`

```
Frame N (client):
  dispatch_simulation_commands → wraps payload + CommandId into envelope, pushes to SubAppCommandInputQueue, adds Dispatched

Frame N (extract):
  drains SubAppCommandInputQueue → writes SimulationCommandEnvelope messages into SubApp

Frame N+1 (simulation):
  fan_out → unwraps SimulationCommandEnvelope, writes CommandEnvelope<SendTrainToBlockRequest> message
  handle_send_train_to_block → reads envelope, pathfinds, writes AppendLegs event, pushes CommandResponse

Frame N+1 (extract):
  drains SubAppCommandResponseQueue → writes CommandResponse messages into client

Frame N+2 (client):
  apply_command_responses → updates CommandState to Completed/Failed
```

---

## Files to create/modify

- **Create**: `brickrail-common/src/command.rs` — CommandId, CommandState, Dispatched, CommandResponse, SubAppCommandResponseQueue, SubAppCommandInputQueue, SimulationCommand, CommandRegistry, CommandPlugin, SimulationCommandPlugin, SendTrainToBlockRequest
- **Modify**: `brickrail-common/src/lib.rs` — add `pub mod command`
- **Modify**: `brickrail-client/src/main.rs` — add CommandPlugin to main app, SimulationCommandPlugin to SubApp, extend extract bridge, refactor init_simulation to use SendTrainToBlock command
- **May create**: handler module for SendTrainToBlock simulation-side logic (or add to simulation.rs)

## Verification

1. `cargo build && cargo test` — existing tests pass
2. Write a test: issue `SendTrainToBlock`, tick a few frames, verify command reaches Completed and train gets route legs
3. Refactor client's `init_simulation` to use the command pattern instead of hardcoded route building
4. `cargo run -p brickrail-client` — train still moves through the layout
