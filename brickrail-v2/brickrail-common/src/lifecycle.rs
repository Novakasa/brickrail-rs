use std::marker::PhantomData;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Trait for types that participate in the layout lifecycle (spawn/register/despawn).
pub trait LayoutElement: Component + Clone {
    /// The ID type used to look up entities of this element type.
    type ID: Send + Sync + Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static;

    /// Extract the ID from this element's data.
    fn id(&self) -> Self::ID;
}

/// Per-type registry mapping element IDs to their ECS entities.
#[derive(Resource)]
pub struct Registry<T: LayoutElement> {
    map: HashMap<T::ID, Entity>,
}

impl<T: LayoutElement> Default for Registry<T> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl<T: LayoutElement> Registry<T> {
    pub fn insert(&mut self, id: T::ID, entity: Entity) {
        self.map.insert(id, entity);
    }

    pub fn remove(&mut self, id: &T::ID) -> Option<Entity> {
        self.map.remove(id)
    }

    pub fn get(&self, id: &T::ID) -> Option<Entity> {
        self.map.get(id).copied()
    }

    pub fn contains(&self, id: &T::ID) -> bool {
        self.map.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&T::ID, &Entity)> {
        self.map.iter()
    }
}

/// Generic spawn message. Send one of these to spawn an element.
#[derive(Message, Clone)]
pub struct SpawnElement<T: LayoutElement>(pub T);

/// Generic despawn message. Send one of these to despawn an element by ID.
#[derive(Message, Clone)]
pub struct DespawnElement<T: LayoutElement> {
    pub id: T::ID,
    _marker: PhantomData<T>,
}

impl<T: LayoutElement> DespawnElement<T> {
    pub fn new(id: T::ID) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }
}

/// Generic lifecycle plugin. Handles spawn/register/despawn for any `LayoutElement` type.
pub struct LifecyclePlugin<T: LayoutElement>(PhantomData<T>);

impl<T: LayoutElement> Default for LifecyclePlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: LayoutElement> LifecyclePlugin<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

fn spawn_element<T: LayoutElement>(
    mut commands: Commands,
    mut messages: MessageReader<SpawnElement<T>>,
    mut registry: ResMut<Registry<T>>,
) {
    for msg in messages.read() {
        let element = msg.0.clone();
        let id = element.id();
        let entity = commands.spawn(element).id();
        registry.insert(id, entity);
    }
}

fn despawn_element<T: LayoutElement>(
    mut commands: Commands,
    mut messages: MessageReader<DespawnElement<T>>,
    mut registry: ResMut<Registry<T>>,
) {
    for msg in messages.read() {
        if let Some(entity) = registry.remove(&msg.id) {
            commands.entity(entity).despawn();
        }
    }
}

impl<T: LayoutElement> Plugin for LifecyclePlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<Registry<T>>();
        app.add_message::<SpawnElement<T>>();
        app.add_message::<DespawnElement<T>>();
        app.add_systems(
            PostUpdate,
            (
                spawn_element::<T>.run_if(on_message::<SpawnElement<T>>),
                despawn_element::<T>.run_if(on_message::<DespawnElement<T>>),
            ),
        );
    }
}
