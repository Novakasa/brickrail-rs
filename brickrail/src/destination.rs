use crate::{
    editor::GenericID,
    layout::EntityMap,
    layout_primitives::{BlockDirection, BlockID, DestinationID, Facing},
    selectable::Selectable,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Message)]
pub struct SpawnDestinationMessage {
    pub dest: Destination,
    pub name: Option<String>,
}

#[derive(SystemParam)]
pub struct SpawnDestinationMessageQuery<'w, 's> {
    query: Query<'w, 's, (&'static Destination, &'static Name)>,
}
impl SpawnDestinationMessageQuery<'_, '_> {
    pub fn get(&self) -> Vec<SpawnDestinationMessage> {
        let mut result = self
            .query
            .iter()
            .map(|(dest, name)| SpawnDestinationMessage {
                dest: dest.clone(),
                name: Some(name.to_string()),
            })
            .collect::<Vec<_>>();
        result.sort_by_key(|d| d.dest.id);
        result
    }
}

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
    pub fn new(id: DestinationID) -> Self {
        Self { id, blocks: vec![] }
    }

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
    type SpawnMessage = SpawnDestinationMessage;
    type ID = DestinationID;

    fn get_type() -> crate::selectable::SelectableType {
        crate::selectable::SelectableType::Destination
    }

    fn generic_id(&self) -> GenericID {
        GenericID::Destination(self.id)
    }

    fn default_spawn_event(entity_map: &mut ResMut<EntityMap>) -> Option<Self::SpawnMessage> {
        Some(SpawnDestinationMessage {
            dest: Destination::new(entity_map.new_destination_id()),
            name: None,
        })
    }

    fn id(&self) -> Self::ID {
        self.id
    }
}

fn spawn_destination(
    mut commands: Commands,
    mut messages: MessageReader<SpawnDestinationMessage>,
    mut entity_map: ResMut<EntityMap>,
) {
    for spawn_dest in messages.read() {
        let name = Name::new(
            spawn_dest
                .name
                .clone()
                .unwrap_or(spawn_dest.dest.id.to_string()),
        );
        let entity = commands.spawn((name, spawn_dest.dest.clone())).id();
        entity_map.add_destination(spawn_dest.dest.id, entity);
    }
}

pub struct DestinationPlugin;

impl Plugin for DestinationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnDestinationMessage>();
        app.register_type::<BlockDirectionFilter>();
        app.add_systems(
            Update,
            spawn_destination.run_if(on_message::<SpawnDestinationMessage>),
        );
    }
}
