use anyhow::Result;
use tobj::{load_obj, LoadOptions};
use std::path::Path;
use tracing::info;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_indices: u32,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            num_indices: 0,
        }
    }

    pub fn load_from_obj<P: AsRef<Path> + std::fmt::Debug>(&mut self, path: P) -> Result<()> {
        info!("Loading OBJ file: {:?}", path.as_ref());
        
        let (models, _materials) = load_obj(
            path,
            &LoadOptions::default(),
        )?;

        self.vertices.clear();
        self.indices.clear();

        for model in &models {
            let mesh = &model.mesh;
            
            // Process vertices
            for i in 0..mesh.positions.len() / 3 {
                let pos = [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ];
                
                // Use a simple color based on position for now
                let color = [0.8, 0.8, 0.8]; // Default gray color
                
                self.vertices.push(Vertex {
                    position: pos,
                    color,
                });
            }

            // Process indices
            if !mesh.indices.is_empty() {
                self.indices.extend(mesh.indices.iter().map(|&i| i as u32));
            } else {
                // If no indices, create them from vertex order
                for i in (0..self.vertices.len()).step_by(3) {
                    if i + 2 < self.vertices.len() {
                        self.indices.push(i as u32);
                        self.indices.push((i + 1) as u32);
                        self.indices.push((i + 2) as u32);
                    }
                }
            }
        }

        info!("Loaded mesh with {} vertices and {} indices", self.vertices.len(), self.indices.len());
        Ok(())
    }

    pub fn create_buffers(&mut self, device: &wgpu::Device) {
        if !self.vertices.is_empty() {
            self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Vertex Buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }));
        }

        if !self.indices.is_empty() {
            self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Index Buffer"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            }));
            self.num_indices = self.indices.len() as u32;
        }
    }

    pub fn get_vertex_buffer(&self) -> Option<&wgpu::Buffer> {
        self.vertex_buffer.as_ref()
    }

    pub fn get_index_buffer(&self) -> Option<&wgpu::Buffer> {
        self.index_buffer.as_ref()
    }
} 