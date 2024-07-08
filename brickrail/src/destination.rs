use crate::{
    editor::{GenericID, Selectable},
    layout::EntityMap,
    layout_primitives::{BlockDirection, BlockID, DestinationID, Facing},
};
use bevy::prelude::*;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Event)]
pub struct SpawnDestinationEvent(pub Destination);

#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Destination {
    pub id: DestinationID,
    pub blocks: Vec<(BlockID, Option<BlockDirection>, Option<Facing>)>,
}

impl Selectable for Destination {
    fn get_id(&self) -> GenericID {
        GenericID::Destination(self.id)
    }
}

fn spawn_destination(
    mut commands: Commands,
    mut events: EventReader<SpawnDestinationEvent>,
    mut entity_map: ResMut<EntityMap>,
) {
    for SpawnDestinationEvent(dest) in events.read() {
        let entity = commands.spawn(dest.clone()).id();
        entity_map.add_destination(dest.id, entity);
    }
}

pub struct DestinationPlugin;

impl Plugin for DestinationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnDestinationEvent>();
        app.register_component_as::<dyn Selectable, Destination>();
        app.add_systems(
            Update,
            (spawn_destination.run_if(on_event::<SpawnDestinationEvent>())),
        );
    }
}
