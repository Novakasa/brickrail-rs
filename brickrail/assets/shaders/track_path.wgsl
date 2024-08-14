#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

@group(2) @binding(0) var<uniform> material_color: vec4<f32>;
@group(2) @binding(1) var<uniform> direction: i32;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var test_uvs = vec4((20.0 * mesh.uv.x - globals.time * f32(direction)) % 1.0, (20.0 * mesh.uv.y) % 1.0, 0.0, 1.0);
    var mask = floor((f32(direction) * mesh.uv.x - globals.time) % 1.0 + 1.8);
    return material_color * mask;
    // var val = (f32(direction) * mesh.uv.x - globals.time) % 1.0 + 0.8;
    // return vec4(val, -val, val - 1.0, 1.0);
}