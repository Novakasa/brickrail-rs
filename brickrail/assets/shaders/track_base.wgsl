#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> material_color: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var uv_test = material_color * vec4(mesh.uv.x, mesh.uv.y, 1.0, 1.0);
    return material_color;
}