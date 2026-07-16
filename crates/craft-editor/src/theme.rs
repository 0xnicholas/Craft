use egui::Context;

pub fn apply(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    if let Some(font) = style.text_styles.get_mut(&egui::TextStyle::Body) {
        font.family = egui::FontFamily::Monospace;
    }
    ctx.set_style(style);
}
