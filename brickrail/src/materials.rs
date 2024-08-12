use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin},
    utils::hashbrown::HashMap,
};

#[derive(Debug, Resource, Default)]
pub struct Materials {
    pub base_material: Handle<TrackBaseMaterial>,
    pub inner_materials: HashMap<TrackInnerMaterial, Handle<TrackInnerMaterial>>,
}

#[derive(Asset, Reflect, AsBindGroup, Debug, Clone)]
pub struct TrackBaseMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material2d for TrackBaseMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/track_base.wgsl".into()
    }
}

#[derive(Asset, Reflect, AsBindGroup, Debug, Clone)]
pub struct TrackInnerMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material2d for TrackInnerMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/track_inner.wgsl".into()
    }
}

#[derive(Asset, Reflect, AsBindGroup, Debug, Clone)]
pub struct TrackPathMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(1)]
    pub direction: i32,
}

impl Material2d for TrackPathMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/track_path.wgsl".into()
    }
}

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TrackBaseMaterial>();
        app.insert_resource(Materials::default());
        app.add_plugins(Material2dPlugin::<TrackBaseMaterial>::default());
        app.add_plugins(Material2dPlugin::<TrackInnerMaterial>::default());
        app.add_plugins(Material2dPlugin::<TrackPathMaterial>::default());
    }
}
