# Architecture Layers

The system is organized into distinct layers, from low-level hardware interaction up to high-level user intent.

```
┌─────────────────────────────────────────────┐
│  Commands (layout editing + train control)  │
├─────────────────────────────────────────────┤
│  Layout State (tracks, connections,         │
│    blocks, markers, trains)                 │
├─────────────────────────────────────────────┤
│  Simulation State (train positions,         │
│    routes, legs, locks)                     │
├─────────────────────────────────────────────┤
│  Simulation Logic (leg advancement,         │
│    lock management, driver dispatch)        │
├─────────────────────────────────────────────┤
│  Driver Layer (virtual / BLE)               │
└─────────────────────────────────────────────┘
```

## Command Layer

The top-level abstraction. All state mutations — whether from GUI interactions, scripted sequences, or property tests — are expressed as commands. This is the canonical entry point for anything that changes the system.

Two categories:

- **Layout commands**: AddTrack, DeleteTrack, CreateBlock, ConnectTracks, SaveLayout, LoadLayout, etc. These modify the static layout data. The simulation does not need to be running.
- **Control commands**: SetRoute, StartTrain, EnterControlMode, ExitControlMode, etc. These cross into simulation territory.

The GUI maps user interactions (clicks, drags, menu selections) to commands. Property tests generate command sequences. Both exercise the same code path.

### Command Lifecycle

Commands are not fire-and-forget — they can fail (e.g. "send train to block" when no route exists). Each command is an ECS entity with a lifecycle:

1. Caller spawns a command entity with the command data (state: **Pending**)
2. A handler system processes it and transitions to **Completed** or **Failed(reason)**
3. The caller holds the entity and can poll or observe the state change

Commands resolve within 1–2 frames but not necessarily on the same frame they're issued, so they need trackable identity. A queue ensures well-defined ordering when multiple commands are issued in the same frame.

### Command Categories

Layout commands and control commands share the entity-per-command pattern and the Pending → Completed/Failed lifecycle, but differ in validity context and lifetime:

| | Layout Commands | Control Commands |
|---|---|---|
| **Examples** | AddTrack, DeleteBlock, SaveLayout | SetRoute, StartTrain |
| **Valid when** | Simulation stopped | Simulation running |
| **Undoable** | Yes | No |
| **Lifetime** | Preserved in undo history | Cleaned up after result is read |

Layout commands are preserved as long as the undo history lives — the command entity contains the data needed to reverse the operation (plus a stored inverse or state snapshot). Control commands have no meaningful undo, so they're despawned once the caller has read the result.

## Layout State

Static data that defines the physical layout: tracks, connections, markers, blocks, trains. Managed via the ECS lifecycle system (SpawnElement messages, registries). The layout is the same whether the simulation is running or not.

The layout is the serialization boundary — SaveLayout/LoadLayout operate here.

## Simulation State

Runtime state layered on top of the layout: train positions, route leg queues, leg states, locks. Mutated exclusively through `SimulationEvent` messages (event-sourced). Both server and client maintain their own copies of this state, kept in sync via the same event stream.

The simulation treats the layout as read-only. It reads block data and marker positions but never modifies them.

## Simulation Logic

Server-side systems that read simulation state and emit `SimulationEvent` messages: leg advancement when a train enters a target block, lock management, driver dispatch. These systems run only on the authoritative side (server or integrated SubApp), never on a pure client.

## Driver Layer

Bridges simulation logic and train execution (virtual or BLE hardware). Receives leg data, autonomously executes it, reports marker hits back. See [train-simulation.md](train-simulation.md) for details.

## Communication

The command layer and simulation event layer are the two communication boundaries:

- **Commands flow down**: `SimulationCommandEnvelope` from client → simulation.
- **SimulationEvents flow up**: `SimulationEvent` from simulation logic → clients.
- **Responses flow up**: `CommandResponse` from simulation → client (correlated by `CommandId`).

The transport is pluggable — SubApp extract or network — but the message types and the systems that produce/consume them are the same.

## Plugin Structure

Plugins are split into transport-agnostic core and transport-specific layers:

### Core plugins (transport-agnostic)

| Plugin | Side | Responsibility |
|---|---|---|
| `LayoutAppPlugin` | Both | Element lifecycle, registries, logical graph |
| `SimulationStatePlugin` | Both | `SimulationEvent` fan-out, state mutation handlers |
| `SimulationLogicPlugin` | Simulation | Leg advancement, driver dispatch, driver↔simulation translation |
| `CommandPlugin` | Client | `CommandId`/`CommandState` lifecycle, `CommandResponse` handling |
| `SimulationCommandPlugin` | Simulation | `SimulationCommandEnvelope` fan-out, command handler systems |
| `VirtualDriverPlugin` | Simulation | Virtual driver tick, marker hit generation |

These plugins consume and produce messages. They don't know how messages arrive or leave.

### SubApp transport

| Plugin | Side | Responsibility |
|---|---|---|
| `SubAppServerPlugin` | Simulation | Collects `SimulationEvent` + `CommandResponse` → queues for extraction |
| `SubAppClientPlugin` | Client | Extract bridge: drains all queues bidirectionally (commands in, events + responses out) |

### Future: Network transport

| Plugin | Side | Responsibility |
|---|---|---|
| `NetworkServerPlugin` | Simulation | Sends `SimulationEvent` + `CommandResponse` over wire |
| `NetworkClientPlugin` | Client | Network connection: sends commands, receives events + responses |

Core plugins stay identical — only the transport plugins are swapped.

## Binary Compositions

Different binaries compose these layers differently:

- **Integrated** (current test client): All layers in one process. Simulation runs in a SubApp with SubApp transport plugins. Extract bridge connects the worlds.
- **Dedicated server**: Core simulation plugins + network transport plugins. No rendering.
- **Editor-only client**: Core client plugins + network transport plugins. Receives SimulationEvents over the network, sends commands over the network.

Each binary picks core plugins + the appropriate transport plugins.
