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
use lyon_tessellation::{
    path::Path, BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex,
    StrokeVertexConstructor, VertexBuffers,
};

use crate::track::LAYOUT_SCALE;

#[derive(Resource)]
pub struct MeshCache<T: MeshType> {
    pub meshes: HashMap<T::ID, Mesh2dHandle>,
}

impl<T: MeshType> MeshCache<T> {
    pub fn insert(&mut self, mesh: &T, assets: &mut Assets<Mesh>) {
        let generated_mesh = mesh.build_mesh();
        self.meshes
            .try_insert(mesh.id(), Mesh2dHandle(assets.add(generated_mesh)))
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

    fn path(&self) -> Path;

    fn base_transform(&self) -> Transform;

    fn interpolate(&self, dist: f32) -> Vec2;

    fn build_mesh(&self) -> Mesh {
        let mut stroke_tesselator = StrokeTessellator::new();
        let mut buffers = VertexBuffers::new();
        let mut builder = BuffersBuilder::new(
            &mut buffers,
            VertexConstructor {
                color: Color::WHITE,
            },
        );
        stroke_tesselator
            .tessellate_path(&self.path(), &Self::stroke(), &mut builder)
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
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            buffers
                .vertices
                .iter()
                .map(|v| {
                    let pos = self.interpolate(v.dist);
                    let pos2 = self.interpolate(v.dist + 0.001);
                    [
                        v.dist,
                        ((pos2 - pos)
                            .normalize()
                            .perp_dot(Vec2::from(v.position) / LAYOUT_SCALE - pos))
                            + Self::stroke().line_width / 2.0,
                    ]
                })
                .collect::<Vec<[f32; 2]>>(),
        );
        mesh
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub dist: f32,
}

pub struct VertexConstructor {
    pub color: Color,
}

impl StrokeVertexConstructor<Vertex> for VertexConstructor {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> Vertex {
        Vertex {
            position: [vertex.position().x, vertex.position().y],
            color: self.color.to_linear().to_f32_array(),
            dist: vertex.interpolated_attributes()[0],
        }
    }
}

fn add_meshes<T: MeshType>(
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_cache: ResMut<MeshCache<T>>,
    query: Query<(Entity, &T), Without<Mesh2dHandle>>,
    mut commands: Commands,
) {
    for (entity, mesh) in query.iter() {
        if !mesh_cache.meshes.contains_key(&mesh.id()) {
            mesh_cache.insert(mesh, &mut meshes);
        }
        commands.entity(entity).insert((
            SpatialBundle {
                transform: mesh.base_transform(),
                ..Default::default()
            },
            mesh_cache.meshes[&mesh.id()].clone(),
        ));
    }
}

pub struct TrackMeshPlugin<T: MeshType> {
    pub marker: PhantomData<T>,
}

impl<T: MeshType> Plugin for TrackMeshPlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(MeshCache::<T>::default());
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
