#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

@group(2) @binding(0) var<uniform> material_color: vec4<f32>;
@group(2) @binding(1) var<uniform> direction: i32;

fn arrow_mask(uv: vec2<f32>, dir: i32) -> f32 {
    return floor((f32(dir) * uv.x + 2.0 * abs(uv.y - 0.5) - globals.time) % 1.0 + 1.8);
}    

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var test_uvs = vec4((20.0 * mesh.uv.x - globals.time * f32(direction)) % 1.0, (20.0 * mesh.uv.y) % 1.0, 0.0, 1.0);
    var mask = arrow_mask(mesh.uv, direction);
    if direction == 0 {
        mask = 1.0;
    }
    if direction == 2 {
        mask = arrow_mask(mesh.uv, 1) * arrow_mask(mesh.uv, -1);
    }
    return material_color * mask;
    // var val = (f32(direction) * mesh.uv.x - globals.time) % 1.0 + 0.8;
    // return vec4(val, -val, val - 1.0, 1.0);
}