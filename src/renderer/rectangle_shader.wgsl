struct VertexOutput {
  [[builtin(position)]] out_pos: vec4<f32>;
  [[location(0)]] out_color: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main([[location(0)]] in_pos: vec2<f32>, [[location(1)]] in_color: vec3<f32>) -> VertexOutput {
  return VertexOutput(vec4<f32>(in_pos, 0.0, 1.0), in_color);
}

[[stage(fragment)]]
fn fs_main([[location(0)]] in_color: vec3<f32>) -> [[location(0)]] vec4<f32> {
  return vec4<f32>(in_color, 1.0);
}
