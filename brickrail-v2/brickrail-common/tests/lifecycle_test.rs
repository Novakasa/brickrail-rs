use bevy::prelude::*;
use brickrail_common::layout::ServerLayout;
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::*;
use brickrail_common::track::Track;

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LayoutInstancePlugin::<ServerLayout>::new());
    app.add_plugins(LifecyclePlugin::<Track, ServerLayout>::new());
    app
}

#[test]
fn spawn_track_via_message() {
    let mut app = make_app();

    let track_id = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let track = Track::new(track_id);

    app.world_mut()
        .write_message(SpawnElement::<Track, ServerLayout>::new(track));
    app.update();

    // Entity should be registered
    let registry = app.world().resource::<Registry<Track, ServerLayout>>();
    assert_eq!(registry.len(), 1);
    let entity = registry.get(&track_id).expect("track should be in registry");

    // Entity should have the Track component
    let track_component = app
        .world()
        .get::<Track>(entity)
        .expect("entity should have Track");
    assert_eq!(track_component.id, track_id);
}

#[test]
fn despawn_track_via_entity_event() {
    let mut app = make_app();

    let track_id = TrackID::new(CellID::new(1, 2, 0), Orientation::NS);
    let track = Track::new(track_id);

    // Spawn
    app.world_mut()
        .write_message(SpawnElement::<Track, ServerLayout>::new(track));
    app.update();

    let entity = app
        .world()
        .resource::<Registry<Track, ServerLayout>>()
        .get(&track_id)
        .unwrap();

    // Despawn via entity event
    app.world_mut()
        .commands()
        .entity(entity)
        .trigger(|entity| DespawnElement { entity });
    app.world_mut().flush();
    app.update();

    // Registry should be empty
    let registry = app.world().resource::<Registry<Track, ServerLayout>>();
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
        app.world_mut()
            .write_message(SpawnElement::<Track, ServerLayout>::new(Track::new(*id)));
    }
    app.update();

    let registry = app.world().resource::<Registry<Track, ServerLayout>>();
    assert_eq!(registry.len(), 3);

    for id in &ids {
        assert!(registry.contains(id));
    }
}
