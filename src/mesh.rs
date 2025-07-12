use anyhow::Result;
use tobj::{load_obj, LoadOptions};
use std::path::Path;
use tracing::info;
use wgpu::util::DeviceExt;
use glam::Vec3;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
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
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
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
            
            // Load positions and normals
            let mut positions = Vec::new();
            let mut normals = Vec::new();
            
            for i in 0..mesh.positions.len() / 3 {
                let pos = [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ];
                positions.push(pos);
                
                // Use provided normals or default to up vector
                let normal = if i < mesh.normals.len() / 3 {
                    [
                        mesh.normals[i * 3],
                        mesh.normals[i * 3 + 1],
                        mesh.normals[i * 3 + 2],
                    ]
                } else {
                    [0.0, 1.0, 0.0]
                };
                normals.push(normal);
            }

            // Load indices
            if !mesh.indices.is_empty() {
                self.indices.extend(mesh.indices.iter().map(|&i| i as u32));
            } else {
                // Generate indices for triangle list
                for i in (0..positions.len()).step_by(3) {
                    if i + 2 < positions.len() {
                        self.indices.push(i as u32);
                        self.indices.push((i + 1) as u32);
                        self.indices.push((i + 2) as u32);
                    }
                }
            }

            // Create vertices with calculated normals if needed
            for i in 0..positions.len() {
                let mut normal = normals[i];
                
                // If no normals provided, calculate from geometry
                if mesh.normals.is_empty() {
                    normal = self.calculate_normal_for_vertex(i, &positions, &self.indices);
                }
                
                let color = [0.8, 0.8, 0.8]; // Default gray color
                
                self.vertices.push(Vertex {
                    position: positions[i],
                    normal,
                    color,
                });
            }
        }

        info!("Loaded mesh with {} vertices and {} indices", self.vertices.len(), self.indices.len());
        Ok(())
    }

    fn calculate_normal_for_vertex(&self, vertex_index: usize, positions: &[[f32; 3]], indices: &[u32]) -> [f32; 3] {
        let mut normal = Vec3::ZERO;
        let mut count = 0;
        
        // Find all triangles that use this vertex
        for i in (0..indices.len()).step_by(3) {
            if i + 2 < indices.len() {
                let idx1 = indices[i] as usize;
                let idx2 = indices[i + 1] as usize;
                let idx3 = indices[i + 2] as usize;
                
                if idx1 == vertex_index || idx2 == vertex_index || idx3 == vertex_index {
                    let v1 = Vec3::from_slice(&positions[idx1]);
                    let v2 = Vec3::from_slice(&positions[idx2]);
                    let v3 = Vec3::from_slice(&positions[idx3]);
                    
                    let edge1 = v2 - v1;
                    let edge2 = v3 - v1;
                    let face_normal = edge1.cross(edge2).normalize();
                    
                    normal += face_normal;
                    count += 1;
                }
            }
        }
        
        if count > 0 {
            normal = normal.normalize();
        } else {
            normal = Vec3::Y; // Default up vector
        }
        
        [normal.x, normal.y, normal.z]
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