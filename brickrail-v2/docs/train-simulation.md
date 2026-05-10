# Train Simulation Domain Model

This document defines the domain model for markers, blocks, trains, and routes — the core types that drive the train simulation.

## Markers

A **marker** is a marked position on the layout that a sensor on a train can detect as it passes over. Markers are the only feedback mechanism from the physical layout — they form the basis for all train state transitions.

- One marker per track (at most).
- `MarkerID = TrackID` — a marker is identified by the track it sits on.
- Markers serve different logical roles depending on context (block boundary, progress indicator, etc.), but the marker itself doesn't know its role — that's determined during route resolution.

### Hardware Implementation

In the current hardware setup, markers are colored tiles placed on the track, and trains carry a color sensor. Each marker has a **color** (`MarkerColor`: `Red | Yellow | Green | Blue | Cyan | White | Black`). Matching the detected color against the expected color provides a safety check, but a marker can also be configured as a wildcard (any color matches). The simulation logic doesn't assign meaning to specific colors — color is purely for detection and validation at the hardware level.

## Blocks

A **block** is a user-defined continuous section of tracks that can safely contain a train. Everything outside the block can be unlocked for other trains. This aligns with how Rocrail models blocks, not with real-life dispatching block sections.

- A block is defined by its **section**: an ordered `Vec<DirectedTrackID>`.
- `BlockID = { track_a, track_b }` (normalized endpoints of the section) — a convenient unique identifier derived from the section.
- Block sections are **static** — stored in the layout, not computed from the connection graph.
- The section between a block's two endpoint markers must be long enough to fully contain any train.

### Endpoint Markers

When a user creates a block, two **endpoint markers** are automatically spawned — one at each end. These markers are owned by the block (created with it, removed with it) and signal to the user that physical markers should be placed at these positions.

### Blocks at Runtime

Blocks are the **lockable unit** of the simulation: they define which sections need to be locked and unlocked in response to marker events. Route legs are structured around blocks (start block → travel → target block), and the simulation uses block identity to determine:

- Which sections to lock ahead of the train
- When to release locks behind the train (only after the train has **entered the target block**, not merely when it has "left" the source block — because travel sections between blocks are not guaranteed to be long enough to contain the train)

### Per-Direction Configuration

Blocks have configuration that varies by **travel direction** (aligned or against the section direction). This is stored as two `DirectedBlockConfig` values on the block data.

Currently this includes **passthrough speed** — a target speed that applies when a train passes through the block without intending to stop. During route resolution, if a block is not the train's destination, the passthrough speed for the relevant travel direction is applied. This allows modeling speed adjustments for track features — slowing down for curves, or speeding up to climb ramps (where one direction needs fast and the other slow).

### Train-Block Position States

A train's relationship to a block has 4 states:

1. **Outside** — train is elsewhere.
2. **Entering** — the train's leading end has crossed the boundary, trailing end hasn't fully entered.
3. **Entered** — entire train is within the block.
4. **Exiting** — the train's leading end has crossed the far boundary, trailing end hasn't fully left.

These states are defined in terms of the train body relative to the block, not the sensor. Note that the marker role names (Entering, Entered, Exited) are related but distinct — they name the *event* that a marker triggers, not the ongoing train-block state. The Entered marker fires at the moment the train transitions from Entering to Entered state. The Exited marker fires in the *next leg's* context, signaling that the train has left the previous block.

### Canonical Enter Marker

Each block has two endpoint markers: **marker A** (on the first track of the section) and **marker B** (on the last track). The **canonical enter marker** — the marker whose detection signals the "entered" state transition — depends on both **travel direction** and **facing**:

| Travel direction       | Facing   | Enter marker |
|------------------------|----------|--------------|
| Aligned with section   | Forward  | B (far end)  |
| Aligned with section   | Backward | A (near end) |
| Against section        | Forward  | A (near end) |
| Against section        | Backward | B (far end)  |

The logic: the "entered" marker is the one the sensor crosses when the train body has fully entered the block. A forward-facing sensor is on the leading side, so it crosses the far boundary first. A backward-facing sensor is on the trailing side, so it crosses the near boundary only after the body has already moved deep into the block.

All other marker roles in a route leg are defined by their position **relative to** this canonical enter marker — not by a fixed index shift. Markers before the enter marker (in travel order) may signal "entering" or speed changes; markers after it may signal "leaving". If additional markers exist inside the block section (between A and B), they naturally fall into the correct role based on which side of the enter marker they sit on.

## Trains

A **train** is a physical or virtual train entity.

- `TrainID = u32` — a numeric identifier. The train's human-readable name is stored in data.
- Layout data includes the train name and hardware configuration (hub kind, channel, speed calibration — to be expanded later).
- Runtime state (position, speed, route assignment) belongs to the simulation tier, not the layout.

### Train Facing

A train has a **facing** relative to its travel direction — it determines whether the sensor is on the leading or trailing side of the train. Facing affects which marker serves as the canonical enter marker for a given block (see Canonical Enter Marker above).

## Routes

A **route** describes a path a train takes from one block to another.

### Sensor Trajectory and Train Body

Pathfinding operates on the **sensor trajectory** — the path the train's sensor follows from one block's canonical enter marker to the next. This is the primary data for each route leg. Markers are collected along this trajectory, since the sensor is what detects them.

However, the train has non-zero length — its body extends behind the sensor. A route leg therefore also stores the **full block sections** (start and target) from block data, even though the sensor may only traverse part of them. These sections are needed for **locking**: the entire block must be reserved to ensure the train body fits, not just the tracks the sensor crosses.

### Route Legs

A route is composed of **route legs**. Each route leg stores:

1. **Start block section** — the full section of the starting block (for locking).
2. **Travel section** — the tracks between the start and target blocks (from the sensor trajectory).
3. **Target block section** — the full section of the target block (for locking).
4. **Markers** — collected along the sensor trajectory, not from the full block sections.

### Marker Collection and Roles

Markers are collected along the sensor trajectory (enter marker to enter marker) and assigned roles by position. Each leg has at most one marker of each role (or none if there aren't enough markers). The three marker roles align with the train-block states:

- **Exiting**: the first marker in the leg — the canonical enter marker of the start block. The train starts here and is exiting the start block.
- **Entering**: the marker immediately before the Entered marker in travel order — signals the train's leading end is crossing into the target block area.
- **Entered**: the last marker in the leg — the canonical enter marker of the target block. Signals the train has fully entered the target block. Anchors lock release.

A typical leg's marker sequence:

```
[Exiting] → ...no role... → [Entering] → [Entered]
```

The same physical marker is **Entered** at the end of one leg and **Exiting** at the start of the next — its role depends on which leg it belongs to.

All remaining markers (those between Exiting and Entering) have **no role**, corresponding to the **Outside** train-block state — the train is between blocks. These markers are used only for visual progress interpolation.

Marker roles are a **route leg concern**, not a property of the marker itself. The same physical marker can have different roles in different route legs.

### Marker Speed

Each marker in a route leg has a **baked-in speed** (`TrainSpeed`: `Slow | Cruise | Fast`) resolved at leg construction time. This is the target speed the train should travel at after passing this marker. The speed is determined by the marker's role:

- **Exiting** — passthrough speed of the **start block** (for the relevant travel direction).
- **Entering** / **Entered** — passthrough speed of the **target block** (for the relevant travel direction).
- **No role** — `Cruise` (default speed for travel sections between blocks).

Speeds are resolved once when the leg is built, not looked up dynamically at runtime. This keeps the leg self-contained — the driver (virtual or BLE) receives exactly the speeds it needs without consulting block metadata.

When a train intends to stop at the current leg (i.e. no next leg is queued), the driver overrides the baked-in speeds: it slows down after the **Entering** marker and stops after the **Entered** marker, regardless of the speeds assigned to those markers. The baked-in speeds only apply during pass-through.

### Travel Section Markers

- Travel sections can have **0 to many** markers.
- If a travel section has no markers near a block boundary, the adjacent block's boundary marker serves as the Exited or Entering marker instead. This is a **conservative fallback** — the train will be in transitional states for longer, but correctness is preserved.
- Extra markers in travel sections (beyond the minimum needed) help with showing visual progress but don't change block state logic.

### Locking

When a train travels a route leg, it locks all three stages:
- The **start block section**
- The **travel section**
- The **target block section**

Locks are **not** released progressively as the train's rear clears each section. Instead, a section is only unlocked once the train has **entered** the target block (i.e. the Entered marker fires). This is because travel sections between blocks are not guaranteed to be long enough to contain the train — only blocks provide that guarantee. Once the Entered marker fires, the start block and travel section can be released together.

## Route Resolution

1. Pathfinding (A* on the logical graph) produces a path of logical tracks — the **sensor trajectory** from the start block's enter marker to the target block's enter marker.
2. The path is split into **route legs** at canonical enter markers (using a logical track → logical block lookup).
3. Each leg resolves its full block sections from block data (for locking) and extracts travel tracks from the path slice.
4. Markers are collected along the sensor trajectory (the path slice, not the full block sections).
5. Marker roles are assigned by position within the leg.

## State Events

Simulation state is **event-sourced**. The server holds the authoritative state and can only mutate it by emitting state events. These same events are forwarded to clients, who apply the same mutations to their own state copies.

State events describe **resulting state changes**, not causes. The server's simulation logic decides what happens; the emitted events describe the outcome. This keeps mutation logic trivial — the client applies state deltas mechanically without understanding simulation logic.

State events use **domain IDs** (`TrainID`, `BlockID`, `TrackID`), not ECS entities. Both server and client resolve IDs to entities via their own registries. This allows state events to be serialized and sent over the network.

Examples of state events:
- `AppendLegs` — append pre-built route legs to a train's queue
- `LockAcquired` / `LockReleased` — block/track lock changes (future)
- `TrainStateChanged` — train movement state transitions (future)
- `MarkerPassed` — train sensor passed a marker (future)

## Train Control Hierarchy

Train behavior is driven by a layered abstraction, from low-level to high-level:

### Route Legs

The **route leg** is the atomic unit of train movement — one block-to-block traversal. A train is always assigned to a route leg. A stationary train has a single-block "idle" leg (no travel section, no target block — just occupying a block).

### Routes

A **route** is a mutable queue of route legs. The train executes legs in order, advancing to the next leg as it enters each target block. Routes are not immutable — upcoming legs can be removed or replaced while the train is in transit. Only the currently active leg (and any already-locked sections) are committed.

### Destinations

A **destination** is a target the train wants to reach. It can be:
- A single block
- A set of acceptable blocks (any one satisfies the destination)
- Optionally constrained by target direction and/or facing

A destination drives route computation: the system pathfinds from the train's current position to the destination and produces a route. If the current route becomes blocked (e.g. another train holds a lock), the system may recompute the route with different legs that reach the same destination via a free path. This is why routes are mutable queues — they serve the destination, not the other way around.

### Strategies

The highest level of control assigns **destinations** to trains over time. A **strategy** is a pluggable policy that produces destinations:
- A **schedule strategy** assigns a fixed sequence of destinations (e.g. stop A → stop B → stop C → repeat).
- A **random strategy** assigns arbitrary destinations periodically.
- Other strategies can be added (e.g. demand-driven, priority-based).

Strategies only produce destinations — they don't interact with routes or legs directly.

## Train Driver

The **train driver** is the abstraction that bridges the simulation logic and the physical (or simulated) train. It receives leg data and autonomously executes it, reporting events back to the simulation state layer.

### Why an Abstraction?

The simulation state layer (`TrainPosition`, `TrainLegState`, `TrainMarkerHit`, `AdvanceLeg`) is hardware-agnostic — it reacts to events without knowing what produced them. The driver is what produces those events. A consistent interface means the simulation logic works identically whether trains are physical BLE devices or virtual simulations.

### Interface

**Simulation logic → Driver:**
- Append a leg to the driver's queue (with marker data and facing)

**Driver → Simulation state:**
- `TrainMarkerHit` — the train passed a marker

The driver does not report leg advancement — the simulation logic infers leg transitions from marker events (specifically, when the Entered marker is hit and a next leg exists). The BLE train may advance legs locally for latency reasons, but the server derives its own state independently from marker hits. A debug assertion can verify they stay in sync.

### Driver Behavior

The driver maintains a queue of legs. Each leg contains a sequence of markers with metadata (role, speed, color/position) and a facing direction. Its behavior is simple:

1. Execute the current leg — advance through its markers in order.
2. Marker metadata determines speed behavior — e.g. slow down at Entering, stop at Entered.
3. When the current leg is complete (all markers passed):
   - **If another leg is queued** → advance to it immediately (pass through).
   - **If no next leg** → stop.
4. Report each marker hit back to the simulation.

The driver's leg data is a thin subset of a `RouteLeg` — just markers and facing. It doesn't include block sections, block IDs, or locking information. Those stay in the simulation layer.

This means the simulation logic controls when the train can proceed by controlling when it sends legs. In practice, the logic only sends a leg once the required locks are acquired. The driver doesn't know about locks — it just knows whether it has more legs to execute.

### BLE Hardware Driver

BLE trains run MicroPython and are semi-autonomous. Due to BLE latency (especially with many trains), the train must react to sensor detections locally without round-tripping to the control PC.

The control PC downloads legs to the BLE train's queue. Each leg encodes:
- Marker sequence (expected colors for sensor validation)
- Marker roles (which markers trigger speed changes)
- Facing/direction

The BLE train's onboard logic handles:
- Sensor reading and color matching
- Speed control (accelerate, cruise, slow down, stop) based on marker roles
- Autonomous leg advancement when the next leg is already queued
- Stopping when no next leg is available

The train reports events back to the control PC asynchronously:
- Marker passed (with index)
- Unexpected marker (color mismatch — safety event)

Only legs with acquired locks are sent to the train. This replaces the previous design where all legs were sent and each had a dynamic `intent_stop` flag that could be toggled remotely. The new approach is simpler: if a leg is on the train, it's safe to traverse.

### Virtual Driver

The virtual driver simulates the same behavior without hardware. It uses the leg's marker positions and a simulated speed to advance through markers. Marker positions reflect actual layout distances so that simulated travel times are consistent across legs of different lengths.

Each simulation tick:
1. Advance the train's continuous position by `speed × delta_time`.
2. If the position crosses the next marker's position, emit `TrainMarkerHit`.
3. If the current leg is complete and a next leg exists, advance to it.
4. If the current leg is complete and no next leg exists, stop.

The virtual driver receives the same leg data as the BLE driver and produces the same events. The simulation logic doesn't distinguish between the two.
