use anyhow::Result;
use native_dialog::{FileDialog, MessageDialog, MessageType};
use tracing::info;

pub struct Menu;

impl Menu {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn open_file(&self) -> Result<Option<std::path::PathBuf>> {
        info!("Opening file dialog...");
        
        let path = FileDialog::new()
            .set_title("Open OBJ File")
            .add_filter("OBJ Files", &["obj"])
            .add_filter("All Files", &["*"])
            .show_open_single_file()?;

        match path {
            Some(path) => {
                info!("Selected file: {:?}", path);
                Ok(Some(path))
            }
            None => {
                info!("No file selected");
                Ok(None)
            }
        }
    }

    pub fn save_file(&self) -> Result<()> {
        info!("Opening save file dialog...");
        
        let path = FileDialog::new()
            .set_title("Save File")
            .add_filter("All Files", &["*"])
            .show_save_single_file()?;

        match path {
            Some(path) => {
                info!("Save file path: {:?}", path);
                // TODO: Implement save functionality
                let _ = self.show_info("Save", "Save functionality not yet implemented");
            }
            None => {
                info!("No save path selected");
            }
        }

        Ok(())
    }

    pub fn show_about(&self) -> Result<()> {
        MessageDialog::new()
            .set_type(MessageType::Info)
            .set_title("About DotObjViewer")
            .set_text("DotObjViewer v0.1.0\n\nA cross-platform 3D OBJ file viewer written in Rust.")
            .show_alert()?;
        
        Ok(())
    }

    pub fn show_info(&self, title: &str, message: &str) -> Result<()> {
        MessageDialog::new()
            .set_type(MessageType::Info)
            .set_title(title)
            .set_text(message)
            .show_alert()?;
        
        Ok(())
    }

    pub fn show_error(&self, title: &str, message: &str) -> Result<()> {
        MessageDialog::new()
            .set_type(MessageType::Error)
            .set_title(title)
            .set_text(message)
            .show_alert()?;
        
        Ok(())
    }
} 