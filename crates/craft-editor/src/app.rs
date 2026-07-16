use eframe::egui;

pub struct EditorApp;

impl Default for EditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorApp {
    pub fn new() -> Self {
        Self
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("craft-editor");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_app_constructs() {
        let _app = EditorApp::new();
    }
}
