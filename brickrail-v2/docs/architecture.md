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

- **Commands flow down**: from user/test → layout state and simulation.
- **SimulationEvents flow up**: from simulation logic → clients.

In an integrated binary, both directions use SubApp extract (same process). In a networked setup, both become network messages. The layers above and below the boundary don't change — only the transport.

## Binary Compositions

Different binaries compose these layers differently:

- **Integrated** (current test client): All layers in one process. Simulation runs in a SubApp, events are extracted to the main app for rendering.
- **Dedicated server**: Layout state + simulation + network server. No rendering.
- **Editor-only client**: Layout state + rendering + network client. Receives SimulationEvents over the network, sends commands over the network.

Each binary picks the plugins it needs. The communication layer (SubApp extract vs. network) is the interchangeable piece.
