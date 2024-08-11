use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin},
};

#[derive(Debug, Resource, Default)]
pub struct Materials {
    pub white: Option<Handle<ColorMaterial>>,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TrackBaseMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material2d for TrackBaseMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/track_base.wgsl".into()
    }
}

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Materials::default());
        app.add_plugins(Material2dPlugin::<TrackBaseMaterial>::default());
    }
}
