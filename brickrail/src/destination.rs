use crate::{
    editor::{GenericID, Selectable},
    layout::EntityMap,
    layout_primitives::{BlockDirection, BlockID, DestinationID, Facing},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Event)]
pub struct SpawnDestinationEvent(pub Destination);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BlockDirectionFilter {
    Any,
    Aligned,
    Opposite,
}

impl BlockDirectionFilter {
    pub fn iter_directions(&self) -> impl Iterator<Item = &BlockDirection> {
        match self {
            BlockDirectionFilter::Any => [BlockDirection::Aligned, BlockDirection::Opposite].iter(),
            BlockDirectionFilter::Aligned => [BlockDirection::Aligned].iter(),
            BlockDirectionFilter::Opposite => [BlockDirection::Opposite].iter(),
        }
    }
}

#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Destination {
    pub id: DestinationID,
    pub blocks: Vec<(BlockID, BlockDirectionFilter, Option<Facing>)>,
}

impl Destination {
    pub fn contains_block(&self, block_id: BlockID) -> bool {
        self.blocks.iter().any(|(id, _, _)| *id == block_id)
    }

    pub fn get_block_filter(&self, block_id: BlockID) -> Option<BlockDirectionFilter> {
        self.blocks
            .iter()
            .find(|(id, _, _)| *id == block_id)
            .map(|(_, filter, _)| filter.clone())
    }

    pub fn remove_block(&mut self, block_id: BlockID) {
        self.blocks.retain(|(id, _, _)| *id != block_id);
    }

    pub fn add_block(
        &mut self,
        block_id: BlockID,
        direction: BlockDirectionFilter,
        facing: Option<Facing>,
    ) {
        self.blocks.push((block_id, direction, facing));
    }

    pub fn change_filter(&mut self, block_id: BlockID, direction: BlockDirectionFilter) {
        if let Some((_, filter, _)) = self.blocks.iter_mut().find(|(id, _, _)| *id == block_id) {
            *filter = direction;
        }
    }
}

impl Selectable for Destination {
    type SpawnEvent = SpawnDestinationEvent;
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
        let name = Name::new(dest.id.to_string());
        let entity = commands.spawn((name, dest.clone())).id();
        entity_map.add_destination(dest.id, entity);
    }
}

pub struct DestinationPlugin;

impl Plugin for DestinationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnDestinationEvent>();
        app.register_type::<BlockDirectionFilter>();
        app.add_systems(
            Update,
            spawn_destination.run_if(on_event::<SpawnDestinationEvent>()),
        );
    }
}
