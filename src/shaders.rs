use wgpu::ShaderModule;

pub fn create_shader_module(device: &wgpu::Device, source: &str) -> ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
} 