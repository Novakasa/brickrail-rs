# Target Architecture

This document describes the target architecture for brickrail-rs, independent of the current implementation. It may inform a ground-up rewrite rather than an incremental refactor.

## Core Concepts

### Data Tiers

The system has three tiers of data with different lifetimes and ownership:

- **Static layout** — tracks, blocks, switches, markers, train definitions, destinations, schedules, home blocks. Serialized in the layout JSON format. Frozen during control mode. Cannot be modified while the simulation is running.
- **Persistent state** — survives across simulation runs and app modes, but is not part of the static layout. Primarily train block positions. Initialized from home blocks on first spawn, updated during control mode via simulation events, preserved on exit. Used by the client for rendering in edit mode (when no simulation is running). Cached to disk separately from the layout — more cache than save file.
- **Control state** — track locks, current route legs, wait times, marker advance progress. Owned entirely by the core simulation. Only mutable via explicit commands. Initialized fresh when entering control mode (seeded from persistent state where applicable, e.g. train block positions).

### Application Modes

- **Edit mode** — entirely client-side. The user manipulates the static layout. No server/simulation needed.
- **Control mode** — the static layout is serialized and handed to the core simulation. The simulation manages all control state. The client renders and sends commands.

## Communication Protocol

Both directions use ECS messages, aligned with the existing BLE hub command/event pattern. In-process these are Bevy messages; out-of-process a relay layer serializes and forwards them over a socket.

### Commands (client → server)

Commands are the only way the client influences control state. Structural changes (adding/removing tracks, blocks, trains) require dropping back to edit mode.

- Enter control mode (with serialized static layout + persistent state)
- Exit control mode
- Assign destination to train
- Emergency stop (per-train or global)
- (Future: manual switch override, other direct control)

### State Updates (server → client)

Simulation state is event-sourced. The server holds the authoritative simulation state and can only mutate it via state events. These same events fan out to clients, who maintain their own copy of the same state types and apply the same mutations.

State events describe **resulting state changes**, not causes. The server's simulation logic decides what happens (e.g. "train passed a marker, so it advances to the next route leg and releases the lock on the previous section"); the emitted events describe the outcome ("train is now in block Y", "track Z is now unlocked"). This keeps the mutation logic trivial — the client applies state deltas mechanically without understanding simulation logic.

- The simulation data types live in `brickrail-types` (shared)
- The event→state mutation logic also lives in `brickrail-types` — but it is trivial (set fields), not simulation logic
- The server's simulation plugins contain all domain logic and decide which events to emit
- **The server also applies state mutations via the shared event path** — simulation plugins never write to simulation state directly, they emit events. This guarantees the event stream is complete: if it's sufficient to keep the server's own state correct, it automatically keeps clients in sync
- Client rendering systems query the simulation state directly — same queries work on both sides
- The client does NOT receive continuous position data — train position within a block section is interpolated client-side from discrete route events

State events:
- Route events: train passed marker, train advanced to next route leg, route assigned/completed
- Block events: lock acquired/released, train entered/exited block
- Switch position changed
- Schedule stop advanced
- Train state changed (moving, waiting, stopped, error)
- Hardware events: hub connected/disconnected, sensor data (forwarded from BLE layer)

### Position Interpolation

Actual hardware trains do not report continuous position. The simulation emits discrete route events (passed marker, entered block, etc.) and the client interpolates train position between these events for smooth rendering. In virtual simulation mode, the server also simulates positions internally, but the client still only receives route events and does its own interpolation — keeping the interface consistent regardless of whether trains are real or virtual.

## Plugin Architecture

Each element type (e.g. Train, Block, Switch, Marker) has three plugin tiers:

### 1. Lifecycle Plugins

Two tiers:

**Generic lifecycle plugin** — parameterized by type, no type-specific code. Handles the universal spawn/register/despawn pattern shared by all element types.

Responsibilities:
- Register spawn/despawn messages
- Spawn entity with component on spawn message
- Register entity in `Registry<T>` and the type-erased `GenericRegistry`
- Despawn entity and unregister on despawn message

**Type-specific lifecycle plugin** — observes `Add<T>` / `Remove<T>` to perform structural side effects during spawn/despawn. Not every type needs one — only types whose existence affects layout structure.

Examples:
- **Track:** update `Connections` graph when a track is added/removed
- **Block:** establish relationships to associated markers/tracks

### Relationships Over Registries

Prefer Bevy ECS relationships to connect entities rather than maintaining lookup maps. For example, the current `MarkerMap` (which tracks marker-to-block associations) should be replaced by direct ECS relationships between marker and block entities. The `route_modular` refactor already uses this pattern — it should serve as the reference for how associations are expressed.

Registries (`Registry<T>` mapping `T::ID -> Entity`) are still needed for ID-based lookups (e.g. deserializing a layout where entities reference each other by ID), but the goal is to minimize their use at runtime. Once entities are spawned and relationships established, queries over relationships should replace registry lookups in simulation and rendering code.

A generic `Registry<T>` resource per element type replaces the current monolithic `EntityMap`. A type-erased `GenericRegistry` maps `GenericID -> Entity` for cross-type lookups where needed. The generic lifecycle plugin inserts into both.

### 2. Simulation State Plugin (shared, per-type)

Manages simulation state for a given type. Lives in `brickrail-types`. Included by both server and client. The state is event-controlled: it is only mutated by applying state events through shared mutation logic. Both server and client run this plugin — the difference is who produces the events.

Responsibilities:
- Define simulation data components for the type
- Define state event types
- Apply state events to simulation data (trivial field-level mutations)
- Initialize simulation state when entering control mode
- Clean up simulation state when exiting control mode

### 3. Simulation Logic Plugin (server-only, type-specific)

Contains the actual domain logic that drives the simulation. Reads simulation state, makes decisions, emits state events. Only runs on the server.

Examples:
- **Train:** route assignment, leg advancement, marker detection, speed control, waiting logic
- **Block:** lock acquisition/release decisions
- **Schedule:** time tracking, stop advancement, destination assignment
- **Switch:** position management, motor control

### 4. Editor Plugin (client-only, type-specific)

Manages rendering, input handling, property editing, selection, and inspector UI. Only relevant when there is a graphical client.

Responsibilities:
- Visual representation (meshes, gizmos, shapes)
- Selection and hover behavior
- Inspector UI for modifying layout properties
- Keyboard/mouse shortcuts for creating/deleting elements

## App Layer

Both server and client need an app-level plugin that manages mode transitions and controls which plugins are active. This replaces the current `EditorState` which conflates editor and simulation concerns.

**Server app plugin:**
- Receives "enter control mode" command with serialized layout
- Loads layout via lifecycle plugins
- Activates simulation state + simulation logic plugins
- On "exit control mode": persists persistent state, tears down simulation state
- Simpler state machine: idle → running → idle

**Client app plugin:**
- Manages the full edit ↔ control transition
- Edit mode: lifecycle + editor plugins active
- Enter control mode: serializes layout + persistent state, sends to server (or to local server plugins in combined binary), activates simulation state plugin (to receive state updates), deactivates editor mutation systems
- Exit control mode: tears down simulation state, re-enables editor mutation systems

The client keeps its layout entities throughout both modes — they're needed for rendering the static layout. On transition to control mode, the client serializes the layout and sends it to the server, disables editor mutation systems, and starts receiving simulation state events. The client's entities are not despawned, just frozen. Simulation state (train positions, lock colors, etc.) is applied to client entities via state events from the server.

The server spawns its own separate entities from the serialized layout. In the combined binary, both sets of entities exist in the same Bevy world but are distinct — client entities for rendering, server entities for simulation. The transition still goes through full serialization to ensure the round-trip is exercised every time. The only difference from out-of-process is the transport (direct message vs. network).

## Crate Structure

Four crates in a Cargo workspace:

**`brickrail-types`** (lib) — shared types and generic infrastructure. Both server and client depend on this.
- Layout data components
- Simulation data components, state event types, and event→state mutation logic (simulation state plugins)
- Command message types
- Layout format / serialization
- Generic lifecycle plugin
- Type-specific lifecycle plugins (structural side effects like `Connections`)
- Registry types

**`brickrail-server`** (lib) — server-side plugins. Depends on `brickrail-types`.
- Simulation plugins that decide which state events to emit (route logic, scheduling, block locking, train state machine)
- Hardware plugins (BLE, serial broadcaster)

**`brickrail-client`** (lib) — client-side plugins. Depends on `brickrail-types`.
- Editor plugins (rendering, selection, inspector, input handling)
- Position interpolation from route events
- Communication relay (for remote mode)

**`brickrail`** (bin) — binary targets that compose the above. Depends on all three libs.
- `combined` — client + server in-process (current default)
- `headless` — server only, no rendering
- `remote` — client only, connects to external server

## Separating Layout Data from Simulation Data

Each element's data should be split into two distinct component types:

- **Layout data component** — part of the layout format, serializable, managed by the lifecycle plugin. Freely editable by the client during edit mode. Read-only during control mode (enforced structurally: no simulation system writes to layout components).
- **Simulation data component** — created by the simulation plugin when entering control mode, removed when exiting. Not serialized as part of the layout. Owned entirely by the simulation.

The "freeze" during control mode is not a type-level guarantee — it's enforced by which systems run in which mode. Editor systems that mutate layout data only run in edit mode. Simulation systems only read layout data and write to their own simulation components.

Several current component types violate this split and would need to be separated:

- `AssignedSchedule` — `schedule_id`/`offset` are layout data; `current_stop_index` is simulation data
- `TrackLocks` — pure simulation data, should not be part of layout
- `WaitTime`, `QueuedDestination` — simulation data components on layout entities
- `BLEHub` — hub ID/address are layout data; connection state is simulation data
- `PulseMotor` — motor definition is layout data; pulse-in-progress is simulation data

No component should contain both `#[serde(skip)]` runtime fields and serialized layout fields.

## Open Questions

- Event schema and versioning for the client-server boundary
- Persistent state storage format and location — stored separately from the layout, serialized, auto-cached to disk to survive app restarts but not considered as permanent as the layout itself (more like a cache than a save file)
- How does the client handle reconnecting mid-simulation? (needs to reconstruct render state from a snapshot)
- Exact scope of "persistent state" — just train block positions, or anything else?
