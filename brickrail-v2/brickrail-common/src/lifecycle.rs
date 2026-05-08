use std::marker::PhantomData;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

// --- Trait bounds ---

/// Trait for types that participate in the layout lifecycle (spawn/register/despawn).
pub trait LayoutElement: Component + Clone {
    /// The ID type used to look up entities of this element type.
    type ID: Send + Sync + Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static;

    /// Extract the ID from this element's data.
    fn id(&self) -> Self::ID;
}

/// Marker trait for layout instance types (e.g. `ServerLayout`, `ClientLayout`).
pub trait LayoutType: Component + Default {}

// --- Non-generic relationships ---

/// Relationship: a per-type registry entity belongs to a layout instance entity.
/// Registry entity → Layout instance entity.
#[derive(Component)]
#[relationship(relationship_target = Registries)]
pub struct RegistryOf(pub Entity);

/// Relationship target: a layout instance entity has many registry entities.
#[derive(Component)]
#[relationship_target(relationship = RegistryOf)]
pub struct Registries(Vec<Entity>);

/// Relationship: an element entity is registered in a per-type registry entity.
/// Element entity → Registry entity.
#[derive(Component)]
#[relationship(relationship_target = RegisteredEntities)]
pub struct RegisteredIn(pub Entity);

/// Relationship target: a registry entity has many element entities.
#[derive(Component)]
#[relationship_target(relationship = RegisteredIn)]
pub struct RegisteredEntities(Vec<Entity>);

// --- Non-generic entity event ---

/// Entity event: trigger on an element entity to despawn it.
/// Non-generic — any code can trigger this without knowing the element type.
/// Each `LifecyclePlugin<T, L>` observes this and handles type-specific cleanup.
#[derive(EntityEvent)]
pub struct DespawnElement {
    pub entity: Entity,
}

// --- Typed registry resource ---

/// Per-type, per-layout registry mapping element IDs to their ECS entities.
#[derive(Resource)]
pub struct Registry<T: LayoutElement, L: LayoutType> {
    map: HashMap<T::ID, Entity>,
    _marker: PhantomData<L>,
}

impl<T: LayoutElement, L: LayoutType> Default for Registry<T, L> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            _marker: PhantomData,
        }
    }
}

impl<T: LayoutElement, L: LayoutType> Registry<T, L> {
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

/// Resource storing the registry entity for a given (T, L) pair.
#[derive(Resource)]
pub struct RegistryEntity<T: LayoutElement, L: LayoutType> {
    pub entity: Entity,
    _marker: PhantomData<(T, L)>,
}

// --- Typed messages ---

/// Generic spawn message. Send one of these to spawn an element.
#[derive(Message, Clone)]
pub struct SpawnElement<T: LayoutElement, L: LayoutType> {
    pub element: T,
    _marker: PhantomData<L>,
}

impl<T: LayoutElement, L: LayoutType> SpawnElement<T, L> {
    pub fn new(element: T) -> Self {
        Self {
            element,
            _marker: PhantomData,
        }
    }
}

// --- Layout instance plugin ---

/// Plugin that creates a layout instance entity with the `L` marker component.
/// Add this once per layout type (e.g. `LayoutInstancePlugin::<ServerLayout>`).
pub struct LayoutInstancePlugin<L: LayoutType>(PhantomData<L>);

impl<L: LayoutType> Default for LayoutInstancePlugin<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<L: LayoutType> LayoutInstancePlugin<L> {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Resource storing the layout instance entity for a given layout type.
#[derive(Resource)]
pub struct LayoutInstance<L: LayoutType> {
    pub entity: Entity,
    _marker: PhantomData<L>,
}

impl<L: LayoutType> Plugin for LayoutInstancePlugin<L> {
    fn build(&self, app: &mut App) {
        let entity = app.world_mut().spawn(L::default()).id();
        app.insert_resource(LayoutInstance::<L> {
            entity,
            _marker: PhantomData,
        });
    }
}

// --- Lifecycle plugin ---

/// Generic lifecycle plugin. Handles spawn/register/despawn for any `LayoutElement` type
/// within a specific layout type.
pub struct LifecyclePlugin<T: LayoutElement, L: LayoutType>(PhantomData<(T, L)>);

impl<T: LayoutElement, L: LayoutType> Default for LifecyclePlugin<T, L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: LayoutElement, L: LayoutType> LifecyclePlugin<T, L> {
    pub fn new() -> Self {
        Self::default()
    }
}

fn spawn_element<T: LayoutElement, L: LayoutType>(
    mut commands: Commands,
    mut messages: MessageReader<SpawnElement<T, L>>,
    mut registry: ResMut<Registry<T, L>>,
    registry_entity: Res<RegistryEntity<T, L>>,
) {
    for msg in messages.read() {
        let element = msg.element.clone();
        let id = element.id();
        let entity = commands
            .spawn((element, RegisteredIn(registry_entity.entity)))
            .id();
        registry.insert(id, entity);
    }
}

fn on_despawn_element<T: LayoutElement, L: LayoutType>(
    trigger: On<DespawnElement>,
    query: Query<&T>,
    mut registry: ResMut<Registry<T, L>>,
    mut commands: Commands,
) {
    let entity = trigger.event().entity;
    if let Ok(element) = query.get(entity) {
        registry.remove(&element.id());
        commands.entity(entity).despawn();
    }
}

impl<T: LayoutElement, L: LayoutType> Plugin for LifecyclePlugin<T, L> {
    fn build(&self, app: &mut App) {
        // Create the per-type registry entity, linked to the layout instance
        let layout_entity = app.world().resource::<LayoutInstance<L>>().entity;
        let registry_entity = app
            .world_mut()
            .spawn(RegistryOf(layout_entity))
            .id();
        app.insert_resource(RegistryEntity::<T, L> {
            entity: registry_entity,
            _marker: PhantomData,
        });

        app.init_resource::<Registry<T, L>>();
        app.add_message::<SpawnElement<T, L>>();
        app.add_systems(
            PostUpdate,
            spawn_element::<T, L>.run_if(on_message::<SpawnElement<T, L>>),
        );
        app.add_observer(on_despawn_element::<T, L>);
    }
}

/// Despawn all entities registered under a layout instance.
/// Walks the relationship tree and triggers `DespawnElement` on each leaf entity.
pub fn despawn_all_in_layout(
    layout_entity: Entity,
    registries: &Query<&RegisteredEntities>,
    layout_registries: &Query<&Registries>,
    commands: &mut Commands,
) {
    if let Ok(regs) = layout_registries.get(layout_entity) {
        for &registry_entity in regs.0.iter() {
            if let Ok(registered) = registries.get(registry_entity) {
                for &element_entity in registered.0.iter() {
                    commands.entity(element_entity).trigger(|entity| DespawnElement { entity });
                }
            }
        }
    }
}
