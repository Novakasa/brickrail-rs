use std::{fmt::Debug, hash::Hash, marker::PhantomData};

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    sprite::Mesh2dHandle,
    utils::hashbrown::HashMap,
};
use bevy_prototype_lyon::prelude::tess::{
    path::Path, BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex,
    StrokeVertexConstructor, VertexBuffers,
};

#[derive(Resource)]
pub struct MeshCache<T: MeshType> {
    pub meshes: HashMap<T::ID, Mesh2dHandle>,
}

impl<T: MeshType> MeshCache<T> {
    pub fn insert(&mut self, id: T::ID, assets: &mut Assets<Mesh>) {
        let mesh = T::build_mesh(&id);
        self.meshes
            .try_insert(id, Mesh2dHandle(assets.add(mesh)))
            .unwrap();
    }
}

impl<T: MeshType> Default for MeshCache<T> {
    fn default() -> Self {
        Self {
            meshes: HashMap::default(),
        }
    }
}

pub trait MeshType: Component {
    type ID: Eq + Hash + Send + Sync + Clone + Debug;

    fn id(&self) -> Self::ID;

    fn stroke() -> StrokeOptions;

    fn path(id: &Self::ID) -> Path;

    fn base_transform(&self) -> Transform;

    fn build_mesh(id: &Self::ID) -> Mesh {
        let mut stroke_tesselator = StrokeTessellator::new();
        let mut buffers = VertexBuffers::new();
        let mut builder = BuffersBuilder::new(
            &mut buffers,
            VertexConstructor {
                color: Color::WHITE,
            },
        );
        stroke_tesselator
            .tessellate_path(&Self::path(id), &Self::stroke(), &mut builder)
            .unwrap();

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_indices(Indices::U32(buffers.indices.clone()));
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            buffers
                .vertices
                .iter()
                .map(|v| [v.position[0], v.position[1], 0.0])
                .collect::<Vec<[f32; 3]>>(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            buffers
                .vertices
                .iter()
                .map(|v| v.color)
                .collect::<Vec<[f32; 4]>>(),
        );
        mesh
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

pub struct VertexConstructor {
    pub color: Color,
}

impl StrokeVertexConstructor<Vertex> for VertexConstructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex {
        Vertex {
            position: [vertex.position().x, vertex.position().y],
            color: self.color.to_linear().to_f32_array(),
        }
    }
}

fn add_meshes<T: MeshType>(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut mesh_cache: ResMut<MeshCache<T>>,
    query: Query<(Entity, &T), Without<Mesh2dHandle>>,
    mut commands: Commands,
    mut material_handles: ResMut<Materials>,
) {
    for (entity, id) in query.iter() {
        if !mesh_cache.meshes.contains_key(&id.id()) {
            mesh_cache.insert(id.id(), &mut meshes);
        }
        if material_handles.white.is_none() {
            material_handles.white = Some(materials.add(ColorMaterial::from(Color::WHITE)));
        }
        commands.entity(entity).insert((
            SpatialBundle {
                transform: id.base_transform(),
                ..Default::default()
            },
            mesh_cache.meshes[&id.id()].clone(),
            material_handles.white.clone().unwrap(),
        ));
        println!("Number of meshes: {:?}", mesh_cache.meshes.len());
    }
}

pub struct TrackMeshPlugin<T: MeshType> {
    pub marker: PhantomData<T>,
}

#[derive(Debug, Resource, Default)]
pub struct Materials {
    pub white: Option<Handle<ColorMaterial>>,
}

impl<T: MeshType> Plugin for TrackMeshPlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(MeshCache::<T>::default());
        app.insert_resource(Materials::default());
        app.add_systems(Update, add_meshes::<T>);
    }
}

impl<T: MeshType> Default for TrackMeshPlugin<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}
