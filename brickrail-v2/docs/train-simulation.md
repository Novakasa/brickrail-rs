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

1. **Not in block** — train is elsewhere.
2. **Entering** — the train's leading end has crossed the boundary, trailing end hasn't fully entered.
3. **Entered** — entire train is within the block.
4. **Leaving** — the train's leading end has crossed the far boundary, trailing end hasn't fully left.

These states are defined in terms of the train body relative to the block, not the sensor.

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

### Route Legs

A route is composed of **route legs**. Each route leg has three stages:

1. **Start block section** — the portion of the starting block the train traverses to exit.
2. **Travel section** — the tracks between the start and target blocks.
3. **Target block section** — the portion of the target block the train enters.

### Marker Collection and Roles

During route resolution, markers are collected from all three stages of each leg into an ordered list. The **canonical enter marker** of the target block (determined by travel direction and facing, see above) anchors the role assignment.

Markers in a route leg fall into three tiers, all defined relative to the canonical enter marker:

1. **Primary**: the canonical enter marker itself — anchors block state transitions.
2. **Secondary**: markers with specific roles based on their position relative to the enter marker (e.g. triggering lock release, speed changes).
3. **Tertiary**: all remaining markers — used only for visual progress interpolation.

Marker roles are a **route leg concern**, not a property of the marker itself. The same physical marker can have different roles in different route legs — e.g. a marker in a shared travel section may be secondary in one leg (near the target block boundary) but tertiary in another (far from a different target). The same marker can even serve as the canonical enter marker for one route leg but be tertiary in another.

### Travel Section Markers

- Travel sections can have **0 to many** markers.
- If a travel section has no markers near a block boundary, the adjacent block's own boundary marker is used instead. This is a **conservative fallback** — the train will be in the "entering" and "leaving" states for longer, but correctness is preserved.
- Extra markers in travel sections (beyond the minimum needed) help with showing visual progress but don't change block state logic.

### Locking

When a train travels a route leg, it locks all three stages:
- The **start block section**
- The **travel section**
- The **target block section**

Locks are **not** released progressively as the train's rear clears each section. Instead, a section is only unlocked once the train has **entered the target block**. This is because travel sections between blocks are not guaranteed to be long enough to contain the train — only blocks provide that guarantee. Once the train has entered the target block, the start block and travel section can be released together.

## Route Resolution

1. A route is defined by pathfinding between specific markers (or blocks).
2. The resulting path of tracks is split into **route legs** at block boundaries.
3. Each leg resolves its three stages (start block, travel, target block).
4. Markers are collected from each leg's tracks (via marker-track relationships).
5. Marker roles are assigned based on the train's facing and travel direction.

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
