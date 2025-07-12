use anyhow::Result;
use tracing::info;
use wgpu::{
    Backends, Device, Instance, Queue, SurfaceConfiguration,
};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::mesh::{Mesh, Vertex};
use crate::camera::Camera;
use crate::performance::PerformanceMonitor;
use egui_winit::State as EguiWinitState;
use egui_wgpu::Renderer as EguiRenderer;
use egui::Context as EguiContext;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniforms {
    view_projection: [[f32; 4]; 4],
    view_matrix: [[f32; 4]; 4],
    camera_position: [f32; 3],
    _padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniforms {
    position: [f32; 4],
    color: [f32; 4],
    intensity: f32,
    ambient_strength: f32,
    diffuse_strength: f32,
    specular_strength: f32,
    shininess: f32,
    _pad: [f32; 3], // Pad to 16-byte alignment
}

pub struct Renderer {
    instance: Instance,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    mesh: Mesh,
    has_mesh: bool,
    default_vertex_buffer: wgpu::Buffer,
    camera: Camera,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    light_uniform_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    wireframe_mode: bool,
    
    // Performance monitoring
    performance_monitor: PerformanceMonitor,
    // egui integration
    pub egui_winit_state: EguiWinitState,
    pub egui_ctx: EguiContext,
    egui_renderer: EguiRenderer,
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find an appropriate adapter"))?;

        // Check for POLYGON_MODE_LINE support
        let required_features = wgpu::Features::POLYGON_MODE_LINE;
        let adapter_features = adapter.features();
        let enable_wireframe = adapter_features.contains(required_features);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: if enable_wireframe { required_features } else { wgpu::Features::empty() },
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        if !enable_wireframe {
            tracing::warn!("Wireframe mode not supported on this device. The W key will have no effect.");
        }

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let camera = Camera::new(size.width as f32 / size.height as f32);

        let camera_uniforms = CameraUniforms {
            view_projection: (camera.projection_matrix() * camera.view_matrix()).to_cols_array_2d(),
            view_matrix: camera.view_matrix().to_cols_array_2d(),
            camera_position: [camera.position.x, camera.position.y, camera.position.z],
            _padding: 0.0,
        };

        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let light_uniforms = LightUniforms {
            position: [5.0, 5.0, 5.0, 0.0],
            color: [1.0, 1.0, 1.0, 0.0],
            intensity: 1.0,
            ambient_strength: 0.2,
            diffuse_strength: 0.7,
            specular_strength: 0.5,
            shininess: 32.0,
            _pad: [0.0; 3],
        };

        let light_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Uniform Buffer"),
            contents: bytemuck::cast_slice(&[light_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/triangle.wgsl").into()),
        });

        let wireframe_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wireframe Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wireframe.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let wireframe_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Wireframe Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &wireframe_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &wireframe_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Line,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertices = &[
            Vertex {
                position: [0.0, 0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0],
            },
        ];

        let default_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Default Triangle Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mesh = Mesh::new();

        let egui_ctx = EguiContext::default();
        let egui_winit_state = EguiWinitState::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            None,
            None,
        );
        let egui_renderer = EguiRenderer::new(&device, config.format, None, 1);

        info!("Renderer initialized successfully");
        Ok(Self {
            instance,
            device,
            queue,
            config,
            size,
            render_pipeline,
            wireframe_pipeline,
            mesh,
            has_mesh: false,
            default_vertex_buffer,
            camera,
            camera_uniform_buffer,
            camera_bind_group,
            light_uniform_buffer,
            light_bind_group,
            depth_texture,
            depth_texture_view,
            wireframe_mode: false,
            
            // Performance monitoring
            performance_monitor: PerformanceMonitor::new(),
            // egui integration
            egui_winit_state,
            egui_ctx,
            egui_renderer,
        })
    }

    pub fn load_mesh(&mut self, path: &std::path::Path) -> Result<()> {
        info!("Loading mesh from: {:?}", path);
        self.mesh.load_from_obj(path)?;
        self.mesh.create_buffers(&self.device);
        self.has_mesh = true;
        
        if !self.mesh.vertices.is_empty() {
            let mut min_pos = glam::Vec3::splat(f32::INFINITY);
            let mut max_pos = glam::Vec3::splat(f32::NEG_INFINITY);
            
            for vertex in &self.mesh.vertices {
                let pos = glam::Vec3::from_slice(&vertex.position);
                min_pos = min_pos.min(pos);
                max_pos = max_pos.max(pos);
            }
            
            self.camera.auto_fit_to_model((min_pos, max_pos));
        }
        
        info!("Mesh loaded successfully");
        Ok(())
    }

    pub fn handle_input(&mut self, event: &winit::event::WindowEvent) {
        self.camera.handle_input(event);
    }

    pub fn toggle_wireframe(&mut self) {
        self.wireframe_mode = !self.wireframe_mode;
        info!("Wireframe mode: {}", self.wireframe_mode);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.camera.aspect_ratio = new_size.width as f32 / new_size.height as f32;

            // Recreate depth texture
            self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: new_size.width,
                    height: new_size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_texture_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        }
    }

    pub fn render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        // Update performance monitor
        self.performance_monitor.update();

        // Begin egui frame
        let raw_input = self.egui_winit_state.take_egui_input(window);
        self.egui_ctx.begin_frame(raw_input);

        // Draw performance stats in egui
        let stats = self.performance_monitor.get_stats();
        egui::Window::new("Performance")
            .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
            .resizable(false)
            .collapsible(false)
            .show(&self.egui_ctx, |ui| {
                ui.label(format!("CPU: {:.1}%", stats.cpu_usage));
                ui.label(format!("RAM: {:.1}% ({:.0}MB/{:.0}MB)", stats.memory_usage, stats.memory_used_mb, stats.memory_total_mb));
                ui.label(format!("FPS: {:.1}", stats.fps));
                ui.label(format!("Frame: {:.1}ms", stats.frame_time_ms));
                ui.label(format!("Frames: {}", stats.frame_count));
            });
        let egui_output = self.egui_ctx.end_frame();
        let pixels_per_point = window.scale_factor() as f32;
        let paint_jobs = self.egui_ctx.tessellate(egui_output.shapes, pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point,
        };

        let surface = self.instance.create_surface(window).map_err(|_| wgpu::SurfaceError::Lost)?;
        surface.configure(&self.device, &self.config);
        
        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Update camera uniforms
        let camera_uniforms = CameraUniforms {
            view_projection: (self.camera.projection_matrix() * self.camera.view_matrix()).to_cols_array_2d(),
            view_matrix: self.camera.view_matrix().to_cols_array_2d(),
            camera_position: [self.camera.position.x, self.camera.position.y, self.camera.position.z],
            _padding: 0.0,
        };
        self.queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::cast_slice(&[camera_uniforms]));

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let pipeline = if self.wireframe_mode {
                &self.wireframe_pipeline
            } else {
                &self.render_pipeline
            };

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.light_bind_group, &[]);

            if self.has_mesh {
                if let Some(vertex_buffer) = self.mesh.get_vertex_buffer() {
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    
                    if let Some(index_buffer) = self.mesh.get_index_buffer() {
                        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        if self.wireframe_mode {
                            // For wireframe, draw edges
                            for i in (0..self.mesh.num_indices).step_by(3) {
                                if i + 2 < self.mesh.num_indices {
                                    render_pass.draw_indexed(i..i+3, 0, 0..1);
                                }
                            }
                        } else {
                            render_pass.draw_indexed(0..self.mesh.num_indices, 0, 0..1);
                        }
                    } else {
                        render_pass.draw(0..self.mesh.vertices.len() as u32, 0..1);
                    }
                }
            } else {
                render_pass.set_vertex_buffer(0, self.default_vertex_buffer.slice(..));
                render_pass.draw(0..3, 0..1);
            }
        }

        for (id, image_delta) in &egui_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &paint_jobs, &screen_descriptor);

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.egui_renderer.render(&mut rpass, &paint_jobs, &screen_descriptor);
        }

        for id in &egui_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
    
    pub fn get_performance_stats(&self) -> crate::performance::PerformanceStats {
        self.performance_monitor.get_stats()
    }
} 