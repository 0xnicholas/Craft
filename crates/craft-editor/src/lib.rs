use std::path::PathBuf;

pub mod app;
pub mod engine;
pub mod json_path;
pub mod error;
pub mod io;
pub mod lsp;
pub mod menu;
pub mod panels;
pub mod persist;
pub mod render_helpers;
pub mod renderer;
pub mod state;
pub mod theme;
pub mod watcher;

pub fn run(args: Vec<String>) -> eframe::Result<()> {
    let initial_project = args.get(1).map(PathBuf::from);
    let viewport = egui::ViewportBuilder::default().with_title("craft-editor");
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "craft-editor",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(crate::app::EditorApp::new(
                initial_project.clone(),
            )))
        }),
    )?;
    Ok(())
}
