use glam::{Mat4, Vec3};
use winit::event::{MouseButton, WindowEvent};
use winit::dpi::PhysicalPosition;

pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    
    // Orbit controls
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub is_orbiting: bool,
    pub last_mouse_pos: Option<PhysicalPosition<f64>>,
}

impl Camera {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 45.0_f32.to_radians(),
            aspect_ratio,
            near: 0.1,
            far: 1000.0,
            
            distance: 5.0,
            yaw: 0.0,
            pitch: 0.0,
            is_orbiting: false,
            last_mouse_pos: None,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    pub fn update_position(&mut self) {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        
        self.position = Vec3::new(x, y, z);
    }

    pub fn handle_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: winit::event::ElementState::Pressed,
                ..
            } => {
                self.is_orbiting = true;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: winit::event::ElementState::Released,
                ..
            } => {
                self.is_orbiting = false;
                self.last_mouse_pos = None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.is_orbiting {
                    if let Some(last_pos) = self.last_mouse_pos {
                        let delta_x = position.x - last_pos.x;
                        let delta_y = position.y - last_pos.y;
                        
                        self.yaw += delta_x as f32 * 0.01;
                        self.pitch += delta_y as f32 * 0.01;
                        
                        // Clamp pitch to prevent gimbal lock
                        self.pitch = self.pitch.clamp(-1.5, 1.5);
                        
                        self.update_position();
                    }
                    self.last_mouse_pos = Some(*position);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        self.distance -= y * 0.5;
                        self.distance = self.distance.clamp(0.1, 100.0);
                        self.update_position();
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.distance -= pos.y as f32 * 0.01;
                        self.distance = self.distance.clamp(0.1, 100.0);
                        self.update_position();
                    }
                }
            }
            WindowEvent::Resized(physical_size) => {
                self.aspect_ratio = physical_size.width as f32 / physical_size.height as f32;
            }
            _ => {}
        }
    }

    pub fn auto_fit_to_model(&mut self, model_bounds: (Vec3, Vec3)) {
        let (min, max) = model_bounds;
        let center = (min + max) * 0.5;
        let size = (max - min).length();
        
        self.target = center;
        self.distance = size * 2.0;
        self.update_position();
    }
} 