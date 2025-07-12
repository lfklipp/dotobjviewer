use anyhow::Result;
use tracing::{error, info};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::renderer::Renderer;
use crate::menu::Menu;

pub struct App {
    renderer: Option<Renderer>,
    menu: Menu,
    last_stats_display: Instant,
    stats_display_interval: Duration,
    show_detailed_stats: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            renderer: None,
            menu: Menu::new()?,
            last_stats_display: Instant::now(),
            stats_display_interval: Duration::from_secs(2), // Show stats every 2 seconds
            show_detailed_stats: false,
        })
    }

    pub fn run(mut self) -> Result<()> {
        let event_loop = EventLoop::new()?;
        let window = Rc::new(WindowBuilder::new()
            .with_title("DotObjViewer")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
            .with_resizable(true)
            .build(&event_loop)?);

       
        info!("Initializing renderer...");
        self.renderer = Some(pollster::block_on(Renderer::new(&window))?);

        let window_clone = window.clone();
        let mut app = self;
        event_loop.run(move |event, elwt| {
            if let Err(e) = app.handle_event(event, elwt, &window_clone) {
                error!("Error handling event: {}", e);
            }
        })?;

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: Event<()>,
        elwt: &winit::event_loop::EventLoopWindowTarget<()>,
        window: &Window,
    ) -> Result<()> {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if let Some(renderer) = &mut self.renderer {
                    // Pass event to egui
                    let _ = renderer.egui_winit_state.on_window_event(window, event);
                    renderer.handle_input(event);
                }

                match event {
                    WindowEvent::CloseRequested => {
                        info!("Window close requested");
                        elwt.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        if let Some(renderer) = &mut self.renderer {
                            renderer.resize(*physical_size);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(renderer) = &mut self.renderer {
                            match renderer.render(window) {
                                Ok(_) => {
                                    // Display performance stats periodically
                                    let now = Instant::now();
                                    if now.duration_since(self.last_stats_display) >= self.stats_display_interval {
                                        let stats = renderer.get_performance_stats();
                                        if self.show_detailed_stats {
                                            info!("Performance Stats - CPU: {:.1}%, RAM: {:.1}% ({:.0}MB/{:.0}MB), FPS: {:.1}, Frame: {:.1}ms, Frames: {}", 
                                                stats.cpu_usage, 
                                                stats.memory_usage, 
                                                stats.memory_used_mb, 
                                                stats.memory_total_mb,
                                                stats.fps,
                                                stats.frame_time_ms,
                                                stats.frame_count);
                                        } else {
                                            info!("FPS: {:.1}, CPU: {:.1}%, RAM: {:.1}%", 
                                                stats.fps, stats.cpu_usage, stats.memory_usage);
                                        }
                                        self.last_stats_display = now;
                                    }
                                }
                                Err(wgpu::SurfaceError::Lost) => {
                                    renderer.resize(window.inner_size());
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    elwt.exit();
                                }
                                Err(e) => {
                                    error!("Render error: {:?}", e);
                                }
                            }
                        }
                        window.request_redraw();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state == winit::event::ElementState::Pressed {
                            match event.logical_key.as_ref() {
                                winit::keyboard::Key::Character("o") | winit::keyboard::Key::Character("O") => {
                                    // Check for Ctrl modifier - we'll need to track this separately
                                    if let Ok(Some(path)) = self.menu.open_file() {
                                        if let Some(renderer) = &mut self.renderer {
                                            if let Err(e) = renderer.load_mesh(&path) {
                                                error!("Failed to load mesh: {}", e);
                                            } else {
                                                info!("Successfully loaded OBJ file: {:?}", path);
                                            }
                                        }
                                    }
                                }
                                winit::keyboard::Key::Character("w") | winit::keyboard::Key::Character("W") => {
                                    if let Some(renderer) = &mut self.renderer {
                                        renderer.toggle_wireframe();
                                    }
                                }
                                winit::keyboard::Key::Character("p") | winit::keyboard::Key::Character("P") => {
                                    self.show_detailed_stats = !self.show_detailed_stats;
                                    info!("Detailed performance stats: {}", self.show_detailed_stats);
                                }
                                winit::keyboard::Key::Character("q") | winit::keyboard::Key::Character("Q") => {
                                    info!("Window close requested");
                                    elwt.exit();
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent {
                event: winit::event::DeviceEvent::MouseMotion { .. },
                ..
            } => {
                window.request_redraw();
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }

        Ok(())
    }
} 