use anyhow::Result;
use tracing::{info, error};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopWindowTarget,
    window::Window,
    keyboard::{Key, ModifiersState},
};

use crate::menu::Menu;
use crate::renderer::Renderer;

pub struct App {
    menu: Menu,
    modifiers: ModifiersState,
    renderer: Option<Renderer>,
}

impl App {
    pub fn new() -> Result<Self> {
        let menu = Menu::new()?;
        Ok(Self {
            menu,
            modifiers: ModifiersState::default(),
            renderer: None,
        })
    }

    pub async fn init_renderer(&mut self, window: &Window) -> Result<()> {
        info!("Initializing renderer...");
        let renderer = Renderer::new(window).await?;
        self.renderer = Some(renderer);
        Ok(())
    }

    pub fn handle_event(
        &mut self,
        event: Event<()>,
        elwt: &EventLoopWindowTarget<()>,
        window: &Window,
    ) -> Result<()> {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("Window close requested");
                        elwt.exit();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        self.handle_keyboard_input(event)?;
                    }
                    WindowEvent::ModifiersChanged(modifiers) => {
                        self.modifiers = modifiers.state();
                    }
                    WindowEvent::Resized(physical_size) => {
                        if let Some(renderer) = &mut self.renderer {
                            renderer.resize(physical_size);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(renderer) = &mut self.renderer {
                            match renderer.render(window) {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => {
                                    if let Some(renderer) = &mut self.renderer {
                                        renderer.resize(window.inner_size());
                                    }
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    elwt.exit();
                                }
                                Err(e) => {
                                    eprintln!("{:?}", e);
                                }
                            }
                        }
                        window.request_redraw();
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent { .. } => {}
            Event::UserEvent(_) => {}
            Event::Suspended => {
                info!("Application suspended");
            }
            Event::Resumed => {
                info!("Application resumed");
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            Event::LoopExiting => {
                info!("Event loop exiting");
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_keyboard_input(&mut self, event: winit::event::KeyEvent) -> Result<()> {
        if event.state == winit::event::ElementState::Pressed {
            match event.logical_key {
                Key::Character(ref c) if c == "o" || c == "O" => {
                    if self.modifiers.control_key() {
                        self.load_obj_file()?;
                    }
                }
                Key::Character(ref c) if c == "q" || c == "Q" => {
                    if self.modifiers.control_key() {
                        info!("Quit requested via Ctrl+Q");
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn load_obj_file(&mut self) -> Result<()> {
        if let Some(renderer) = &mut self.renderer {
            if let Some(path) = self.menu.open_file()? {
                match renderer.load_mesh(&path) {
                    Ok(_) => {
                        info!("Successfully loaded OBJ file: {:?}", path);
                        let _ = self.menu.show_info(
                            "File Loaded", 
                            &format!("Successfully loaded: {}", path.display())
                        );
                    }
                    Err(e) => {
                        error!("Failed to load OBJ file: {}", e);
                        let _ = self.menu.show_error(
                            "Load Error", 
                            &format!("Failed to load file: {}", e)
                        );
                    }
                }
            }
        }
        Ok(())
    }
} 