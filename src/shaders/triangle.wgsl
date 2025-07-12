struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct CameraUniforms {
    view_projection: mat4x4<f32>,
    view_matrix: mat4x4<f32>,
    camera_position: vec3<f32>,
}

struct LightUniforms {
    position: vec4<f32>,
    color: vec4<f32>,
    intensity: f32,
    ambient_strength: f32,
    diffuse_strength: f32,
    specular_strength: f32,
    shininess: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(1) @binding(0) var<uniform> light: LightUniforms;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.world_position = model.position;
    out.normal = model.normal;
    out.color = model.color;
    out.clip_position = camera.view_projection * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let light_dir = normalize(light.position.xyz - in.world_position);
    let view_dir = normalize(camera.camera_position - in.world_position);
    let reflect_dir = reflect(-light_dir, normal);
    
    // Ambient lighting
    let ambient = light.ambient_strength * light.color.xyz;
    
    // Diffuse lighting
    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = light.diffuse_strength * diff * light.color.xyz;
    
    // Specular lighting
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), light.shininess);
    let specular = light.specular_strength * spec * light.color.xyz;
    
    // Combine lighting
    let result = (ambient + diffuse + specular) * in.color;
    
    return vec4<f32>(result, 1.0);
} 