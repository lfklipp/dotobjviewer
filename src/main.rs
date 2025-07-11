use anyhow::Result;
use tracing::{info, error};
use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
};
use std::rc::Rc;
use std::cell::RefCell;

mod app;
mod menu;
mod renderer;
mod shaders;
mod mesh;

use app::App;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("Starting DotObjViewer...");

    // Create the event loop
    let event_loop = EventLoop::new()?;

    // Create the window
    let window = Rc::new(WindowBuilder::new()
        .with_title("DotObjViewer")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
        .with_resizable(true)
        .build(&event_loop)?);

    // Create the application
    let app = Rc::new(RefCell::new(App::new()?));

    // Initialize the renderer synchronously
    {
        let mut app_borrow = app.borrow_mut();
        pollster::block_on(app_borrow.init_renderer(&window))?;
    }

    // Run the event loop
    let window_clone = window.clone();
    let app_clone = app.clone();
    event_loop.run(move |event, elwt| {
        let mut app = app_clone.borrow_mut();
        let window = window_clone.as_ref();
        if let Err(e) = app.handle_event(event, elwt, window) {
            error!("Error handling event: {}", e);
        }
    })?;

    Ok(())
}
