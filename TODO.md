# TODO

High-level work items for brickrail-rs.

## Bugs

- [ ] File dialog crashes randomly — needs a more robust solution
- [ ] "New layout" button leaves the layout in virtual run state instead of edit state

## Features / Refactors

- [ ] Finish refactor of trains and routes to be more ECS-native
  - In-progress in `brickrail/src/route_modular.rs`
  - Route is split into fine-grained ECS entities (route legs, markers, positions) linked via relationships rather than a monolithic struct — much more ECS-native
  - Core logic (routing, leg advancement, marker tracking, train state) is working and somewhat tested
  - Not yet on parity with the old system; missing pieces are mostly:
    - Rendering (visual representation of trains/routes)
    - Hooking up with actual hardware (`ble_train`) for real hub communication

- [ ] Align inspector ECS logic with what was done in the ember-line project
  - Current approach in this project struggled with `Selectable` and making a generic inspector system — Rust generics and Bevy ECS don't compose well
  - Pattern discovered in ember-line: use a **generic plugin per type** for type-specific registration and per-type behaviour, but have it communicate with a **central non-generic plugin** via type-erased events/components
  - The central inspector plugin can then handle selection as a unified "currently selected thing" concept without needing to be generic itself, while new types can still be registered cleanly through the generic plugin
  - Essentially type-erasure at the plugin boundary: generic plugin translates type-specific operations into a common representation the inspector understands

- [ ] Add Raspberry Pi Zero support for broadcasting to stationary hubs
  - The existing broadcaster logic in `pybricks/programs/io_hub_unfrozen.py` (the `_IN_ID_BROADCAST_CMD` handler and BLE broadcast loop) is the reference implementation — port this to run on the Pi Zero via serial instead of the PoweredUp hub's USB/BLE stack
  - Pi Zero will likely also run MicroPython, so most of `io_hub_unfrozen.py` can be reused or adapted with minimal changes
  - Start with the same buffering logic as the hub (keep last 8 device IDs + states, rebroadcast on each command) — move buffering/prioritization to the CPU side only if testing reveals it's necessary

- [ ] Establish a clear communication boundary between layout/hardware execution and the front-end (rendering and layout editing)
  - **Definition of done:** at least one headless integration test passes that exercises core logic (e.g. train gets assigned a route and advances through its legs) without any front-end/renderer plugins loaded — the refactoring scope is whatever the friction of writing that test reveals
  - Primary motivation: allow the engine to act as a server for external clients that render/control the layout independently
  - Boundary is Bevy-event-based in-process — the engine exposes a set of events that cross the boundary in both directions
  - An out-of-process layer can be inserted later that simply forwards those events through a socket (WebSocket or similar), making the multi-client story possible without committing to it upfront
  - The event schema crossing the boundary needs to be serializable; versioning becomes important once external clients exist
  - The engine plugin group (routing, collision avoidance, scheduling, hardware comms) runs headlessly with `MinimalPlugins`; the front-end plugin group is optional and just subscribes to the same events
  - **Entity ownership:** backend and editor plugins must maintain separate ECS entities — backend owns logic entities, editor spawns its own rendering entities keyed by stable domain IDs (e.g. `BlockID`). The current `add_block_shape` observer that patches the backend entity is a temporary in-process shortcut, not the end state.
  - **Static layout vs. control state:** the communication boundary maps onto a clean data split:
    - *Static layout* — tracks, blocks, switches, markers, train definitions, destinations, schedules. Frozen at control time; serialized in the layout JSON. The client hands this to the server (as a serialized snapshot) when switching from edit → control mode.
    - *Control state* — track locks, train positions, current route legs, wait times, marker advance progress. Owned entirely by the core; only mutable via explicit commands. Not part of the layout format.
    - *Commands* — valid during control time: set train speed, assign destination, emergency stop, etc. Adding/removing tracks or trains is not a valid command — requires dropping back to edit mode.
    - Edit mode is entirely client-side; no server is needed until the user switches to control mode.
  - **Mixed static/runtime types to resolve:** several component types currently straddle the boundary using `#[serde(skip)]` as a band-aid. When the boundary is formalized these will need to be split into separate components or have their runtime fields freshly initialized on server startup:
    - `AssignedSchedule` — `schedule_id`/`offset` are static layout; `current_stop_index` is control state (already skipped)
    - `TrackLocks` — pure control state, but lives in `LayoutPlugin` alongside layout instantiation data
    - `WaitTime`, `QueuedDestination` — control state components attached to statically-spawned train entities
    - `BLEHub` — hub ID/address are static layout; `HubState`/`HubActive`/`HubReady` etc. are runtime connection state
    - `PulseMotor` — motor definition is static layout; in-progress pulse state is runtime

- [ ] Figure out testing
  - Prefer integration tests over unit tests
  - The communication boundary above is the key enabler: tests plug in at the event level, no front-end or renderer needed
  - Use Bevy's `App::update()` loop with `MinimalPlugins` for headless test runs; inject inputs via `world.send_event(...)`
  - Priority test targets: route/pathfinding logic, collision avoidance (block contention), schedule system
  - Hardware (BLE) and visual output are out of scope for automated tests — verified manually
