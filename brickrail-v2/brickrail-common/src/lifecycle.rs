use bevy::platform::collections::HashMap;
use bevy::prelude::*;

// --- Trait bounds ---

/// Trait for types that participate in the layout lifecycle (spawn/register/despawn).
/// Not a component itself — the lifecycle plugin wraps the ID and data in generic components.
pub trait LayoutElement: Send + Sync + 'static {
    /// The ID type used to look up entities of this element type.
    type ID: Send + Sync + Copy + Eq + std::hash::Hash + std::fmt::Debug + 'static;

    /// The layout data for this element type. Wrapped in `ElementData<T>` on the entity.
    type Data: Send + Sync + Clone + Default + std::fmt::Debug + 'static;

    /// Register type-specific lifecycle plugins (structural side effects).
    /// Called by `ElementPlugin` after the generic lifecycle plugin is added.
    /// Default: no additional plugins.
    fn build_lifecycle(_app: &mut App) {}
}

// --- Generic wrapper components ---

/// Component storing the typed ID for an element entity.
#[derive(Component, Clone, Debug)]
pub struct ElementId<T: LayoutElement>(pub T::ID);

/// Component storing the layout data for an element entity.
#[derive(Component, Clone, Debug, Deref, DerefMut)]
pub struct ElementData<T: LayoutElement>(pub T::Data);

// --- Non-generic relationships ---

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
/// Each `LifecyclePlugin<T>` observes this and handles type-specific cleanup.
#[derive(EntityEvent)]
pub struct DespawnElement {
    pub entity: Entity,
}

// --- Typed registry resource ---

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

/// Resource storing the registry entity for a given element type.
#[derive(Resource)]
pub struct RegistryEntity<T: LayoutElement> {
    pub entity: Entity,
    _marker: std::marker::PhantomData<T>,
}

// --- Serializable element entry ---

/// Generic serializable entry pairing an element's ID with its layout data.
/// Used in the `Layout` format and converts directly into `SpawnElement` messages.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T::ID: serde::Serialize, T::Data: serde::Serialize",
    deserialize = "T::ID: serde::Deserialize<'de>, T::Data: serde::Deserialize<'de>",
))]
pub struct ElementEntry<T: LayoutElement> {
    pub id: T::ID,
    #[serde(default)]
    pub data: T::Data,
}

impl<T: LayoutElement> ElementEntry<T> {
    pub fn new(id: T::ID, data: T::Data) -> Self {
        Self { id, data }
    }
}

// --- Typed messages ---

/// Generic spawn message. Send one of these to spawn an element.
#[derive(Message, Clone)]
pub struct SpawnElement<T: LayoutElement> {
    pub id: T::ID,
    pub data: T::Data,
}

impl<T: LayoutElement> SpawnElement<T> {
    pub fn new(id: T::ID, data: T::Data) -> Self {
        Self { id, data }
    }

    pub fn from_entry(entry: &ElementEntry<T>) -> Self {
        Self::new(entry.id, entry.data.clone())
    }
}

// --- Lifecycle plugin ---

/// Generic lifecycle plugin. Handles spawn/register/despawn for any `LayoutElement` type.
pub struct LifecyclePlugin<T: LayoutElement>(std::marker::PhantomData<T>);

impl<T: LayoutElement> Default for LifecyclePlugin<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
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
    registry_entity: Res<RegistryEntity<T>>,
) {
    for msg in messages.read() {
        let id = msg.id;
        let entity = commands
            .spawn((
                ElementId::<T>(id),
                ElementData::<T>(msg.data.clone()),
                RegisteredIn(registry_entity.entity),
            ))
            .id();
        registry.insert(id, entity);
    }
}

fn on_despawn_element<T: LayoutElement>(
    trigger: On<DespawnElement>,
    query: Query<&ElementId<T>>,
    mut registry: ResMut<Registry<T>>,
    mut commands: Commands,
) {
    let entity = trigger.event().entity;
    if let Ok(element_id) = query.get(entity) {
        registry.remove(&element_id.0);
        commands.entity(entity).despawn();
    }
}

impl<T: LayoutElement> Plugin for LifecyclePlugin<T> {
    fn build(&self, app: &mut App) {
        let registry_entity = app.world_mut().spawn_empty().id();
        app.insert_resource(RegistryEntity::<T> {
            entity: registry_entity,
            _marker: std::marker::PhantomData,
        });

        app.init_resource::<Registry<T>>();
        app.add_message::<SpawnElement<T>>();
        app.add_systems(
            PostUpdate,
            spawn_element::<T>.run_if(on_message::<SpawnElement<T>>),
        );
        app.add_observer(on_despawn_element::<T>);
    }
}

// --- Element plugin (unified entry point) ---

/// Unified plugin for a layout element type. Adds the generic lifecycle plugin
/// and calls `T::build_lifecycle` for type-specific structural side effects.
pub struct ElementPlugin<T: LayoutElement>(std::marker::PhantomData<T>);

impl<T: LayoutElement> Default for ElementPlugin<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: LayoutElement> ElementPlugin<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: LayoutElement> Plugin for ElementPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugins(LifecyclePlugin::<T>::new());
        T::build_lifecycle(app);
    }
}

/// Despawn all registered element entities.
/// Iterates all registry entities and triggers `DespawnElement` on each element.
pub fn despawn_all_elements(registries: &Query<&RegisteredEntities>, commands: &mut Commands) {
    for registered in registries.iter() {
        for &element_entity in registered.0.iter() {
            commands
                .entity(element_entity)
                .trigger(|entity| DespawnElement { entity });
        }
    }
}
