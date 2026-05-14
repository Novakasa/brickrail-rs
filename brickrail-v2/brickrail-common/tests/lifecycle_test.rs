use bevy::prelude::*;
use brickrail_common::layout::track::{Track, TrackData};
use brickrail_common::lifecycle::*;
use brickrail_common::primitives::*;

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ElementPlugin::<Track>::new());
    app
}

#[test]
fn spawn_track_via_commands() {
    let mut app = make_app();

    let track_id = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);

    app.world_mut().spawn_element::<Track>(track_id, TrackData);

    // Entity should be registered
    let registry = app.world().resource::<Registry<Track>>();
    assert_eq!(registry.len(), 1);
    let entity = registry
        .get(&track_id)
        .expect("track should be in registry");

    // Entity should have the ElementId component
    let element_id = app
        .world()
        .get::<ElementId<Track>>(entity)
        .expect("entity should have ElementId<Track>");
    assert_eq!(element_id.0, track_id);
}

#[test]
fn despawn_track_via_entity_event() {
    let mut app = make_app();

    let track_id = TrackID::new(CellID::new(1, 2, 0), Orientation::NS);

    // Spawn
    let entity = app.world_mut().spawn_element::<Track>(track_id, TrackData);

    // Despawn via entity event
    app.world_mut()
        .commands()
        .entity(entity)
        .trigger(|entity| DespawnElement { entity });
    app.world_mut().flush();
    app.update();

    // Registry should be empty
    let registry = app.world().resource::<Registry<Track>>();
    assert!(registry.is_empty());

    // Entity should be gone
    assert!(app.world().get_entity(entity).is_err());
}

#[test]
fn spawn_multiple_tracks() {
    let mut app = make_app();

    let ids = [
        TrackID::new(CellID::new(0, 0, 0), Orientation::EW),
        TrackID::new(CellID::new(1, 0, 0), Orientation::EW),
        TrackID::new(CellID::new(0, 1, 0), Orientation::NS),
    ];

    for id in &ids {
        app.world_mut().spawn_element::<Track>(*id, TrackData);
    }

    let registry = app.world().resource::<Registry<Track>>();
    assert_eq!(registry.len(), 3);

    for id in &ids {
        assert!(registry.contains(id));
    }
}
