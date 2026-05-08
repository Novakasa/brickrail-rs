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

### Passthrough Speed

A block can have a **passthrough speed** — a target speed that applies when a train passes through the block without intending to stop. During route resolution, if a block is not the train's destination, the passthrough speed is applied to the markers within that block's section. This allows modeling speed adjustments for track features — slowing down for curves, or speeding up to climb ramps.

### Train-Block Position States

A train's relationship to a block has 4 states:

1. **Not in block** — train is elsewhere.
2. **Entering** — the train's leading end has crossed the boundary, trailing end hasn't fully entered.
3. **Entered** — entire train is within the block.
4. **Leaving** — the train's leading end has crossed the far boundary, trailing end hasn't fully left.

These states are defined in terms of the train body relative to the block, not the sensor. How the sensor detects these transitions depends on **facing**: whether the sensor is on the leading or trailing side of the train relative to travel direction.

- **Facing forward** (sensor on leading side): the sensor crosses a boundary *before* the train body fully crosses it. Sensor detection directly signals the leading-end transitions.
- **Facing backward** (sensor on trailing side): the sensor crosses a boundary *after* the train body has already started crossing. Sensor detection signals trailing-end transitions instead, so markers must be interpreted with a shift of -1 to infer leading-end events earlier.

## Trains

A **train** is a physical or virtual train entity.

- `TrainID = u32` — a numeric identifier. The train's human-readable name is stored in data.
- Layout data includes the train name and hardware configuration (hub kind, channel, speed calibration — to be expanded later).
- Runtime state (position, speed, route assignment) belongs to the simulation tier, not the layout.

### Train Facing

A train has a **facing** relative to its travel direction. Since trains typically have a sensor only at the front, a train going backwards needs to handle markers differently — it reacts to them later (the sensor is at the rear of the train body).

## Routes

A **route** describes a path a train takes from one block to another.

### Route Legs

A route is composed of **route legs**. Each route leg has three stages:

1. **Start block section** — the portion of the starting block the train traverses to exit.
2. **Travel section** — the tracks between the start and target blocks.
3. **Target block section** — the portion of the target block the train enters.

### Marker Collection and Roles

During route resolution, markers are collected from all three stages of each leg into an ordered list. Marker roles are assigned **relative to the "forward enter" marker** of the target block:

- **Forward-facing train**: roles assigned directly from the ordered marker list.
- **Backward-facing train**: roles shifted by **-1** (react one marker earlier, because the sensor is at the rear of the train body).

Markers in a route leg fall into three tiers, all defined relative to the **"entered" marker** of the target block:

1. **Primary**: the "entered" marker itself — anchors block state transitions.
2. **Secondary**: markers with roles relative to the entered marker (e.g. triggering lock release, speed changes). Their positions are defined by offset from the primary marker.
3. **Tertiary**: all remaining markers — used only for visual progress interpolation.

Marker roles are a **route leg concern**, not a property of the marker itself. The same physical marker can have different roles in different route legs — e.g. a marker in a shared travel section may be secondary in one leg (near the target block boundary) but tertiary in another (far from a different target).

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
