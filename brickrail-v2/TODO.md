# brickrail-v2 TODO

## Done

- [x] ECS lifecycle (spawn/despawn via messages, registries)
- [x] Logical graph for facing-aware pathfinding
- [x] Route building with A* and leg construction
- [x] Train position state (idle legs, marker hits, leg advancement)
- [x] Driver interface layer and virtual driver with simulation
- [x] SimulationEvent sync pipeline (fan-out, collector, SubApp extract)
- [x] Minimal visualization client with gizmo-based track/block/train rendering

## In Progress

- [ ] Proper system scheduling: LogicalGraph rebuild currently runs in `Last` schedule. Should use explicit ordering or a dedicated schedule to avoid brittle implicit ordering.

## Future

- [ ] High-level plugin composition (simulation plugin, client plugin, communication layer between them)
- [ ] Networking: replace SubApp extract with network transport for SimulationEvent
- [ ] Editor→simulation command channel (start train, set route, etc.)
- [ ] Train interpolation: smooth position between marker hits
- [ ] Client picking (hover/click on layout elements)
- [ ] Inspector panel (egui or bevy_ui)
- [ ] Property-based testing (proptest/quickcheck for layout, route, simulation invariants)
