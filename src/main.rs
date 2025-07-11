use anyhow::Result;
use tracing::{info, error};
use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
};
use std::rc::Rc;
use std::cell::RefCell;

mod app;
mod camera;
mod menu;
mod mesh;
mod renderer;
mod shaders;

use app::App;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting DotObjViewer...");

    let app = App::new()?;
    app.run()?;

    Ok(())
}
