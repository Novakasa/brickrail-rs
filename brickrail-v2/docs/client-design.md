# Client Design

Design notes for the editor/renderer client.

## Abstraction Boundary

The client is split into two layers:

```
┌─────────────────────────────────────────────┐
│  Visual Layer (widgets, edit buffers,       │
│    rendering, selection, input mapping)     │
├─────────────────────────────────────────────┤
│  Headless Layer (commands, layout state,    │
│    simulation, mode state)                  │
└─────────────────────────────────────────────┘
```

Everything in the headless layer is testable without rendering or UI. Property tests and scripted sequences operate here directly. The visual layer is purely a presentation and input concern.

### Read/Write Rule

- **Reads are free**: widgets can query any ECS state (layout, simulation, mode) for display or preemptive validation.
- **Writes go through commands**: no widget ever modifies layout, simulation, or mode state directly.

The GUI may disable buttons or show warnings based on its own validation of ECS state, but the command handler is the authority — it always validates independently. GUI validation is UX polish, not a correctness guarantee.

### Commands

Commands take explicit arguments — `DeleteTrack(track_id)`, not `DeleteSelection`. The GUI resolves selection, hover state, or other UI context to concrete IDs before issuing a command. This keeps commands self-contained and testable without any UI state.

See [architecture.md](architecture.md) for command lifecycle (Pending → Completed/Failed) and category differences (layout vs. control).

### Application State vs. Interaction State

Two distinct concepts:

- **Application state** (editing vs. simulation running): lives in the headless layer. Transitions are commands (`EnterControlMode`, `ExitControlMode`). Command handlers check whether the current application state is valid for the command and fail with an error if not. The UI reads application state to adapt its presentation (grey out invalid actions), but enforcement lives in the command layer.

- **Interaction state** (active tool, selection, edit buffers): purely visual layer. Encompasses which tool is active (track building, block creation, selection), what is currently selected, and any in-progress edit buffers. Determines how mouse clicks and drags are interpreted. The command layer doesn't know about any of it. For example, the block creation tool holds intermediate picks (first track, second track) in interaction state and spawns a single `CreateBlock` command when complete. Selection determines what the user sees in the inspector and what gets passed as arguments when the user triggers a command, but commands never reference "the current selection."

## Data Flow

Widgets read from ECS state for display and write via commands for mutations. One-way data flow.

```
ECS State ──read──→ Widget Display
ECS State ──read──→ Preemptive Validation (UX polish)
Widget Interaction ──→ Edit Buffer ──commit──→ Command Entity
```

## Edit Buffers

Some widgets need to buffer edits before committing a command:

- **Sliders**: dragging a float slider shouldn't spawn a command per frame
- **Text fields**: commit on focus loss or enter, not per keystroke
- **Color pickers**: continuous interaction, commit on close/release
- **Multi-field forms**: commit all fields together as one command

The edit buffer is not just a copy of the data fields — it's the widget's working state that eventually produces a command. This includes:

- Copies of the fields being edited (for deferred commit)
- Transient UI state that doesn't exist on the data model (selected tab, validation feedback, intermediate selections)
- Partially constructed compound values (e.g. picking two tracks to define a block — the first pick is held in the buffer until the second completes it)

The buffer lives per inspector instance (or per active selection) and flushes as a single command on commit (mouse release, enter, focus loss).

For discrete interactions (clicking "add track", deleting a selection), no buffer is needed — the widget spawns a command immediately.

## Open Questions

- **Picking**: how to implement hover and click generically across element types
- **Inspector framework**: egui panel? bevy_ui? How does selection drive which buffer is active?
- **Rendering**: gizmos (current), bevy_prototype_lyon, or custom mesh generation?
- **Train interpolation**: smooth train position between marker hits for the client
