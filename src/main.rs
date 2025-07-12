use anyhow::Result;
use tracing::info;

use crate::app::App;

mod app;
mod camera;
mod menu;
mod mesh;
mod renderer;
mod shaders;
mod performance;
// mod overlay;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting DotObjViewer...");
    
    let app = App::new()?;
    app.run()?;
    
    Ok(())
}
