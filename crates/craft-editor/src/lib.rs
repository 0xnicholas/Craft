pub mod app;
pub mod error;
pub mod io;
pub mod panels;
pub mod state;
pub mod watcher;

pub fn run(_args: Vec<String>) -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default().with_title("craft-editor");
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "craft-editor",
        options,
        Box::new(|_cc| Ok(Box::new(crate::app::EditorApp::new()))),
    )?;
    Ok(())
}
