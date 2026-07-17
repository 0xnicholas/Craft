use egui::{Color32, Context, CornerRadius};

pub fn apply(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = Color32::from_rgb(22, 33, 62);
    visuals.window_fill = Color32::from_rgb(26, 26, 46);
    visuals.override_text_color = Some(Color32::from_rgb(224, 224, 224));
    visuals.selection.bg_fill = Color32::from_rgb(15, 52, 96);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(22, 33, 62);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(15, 52, 96);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(108, 159, 255);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(15, 52, 96);
    visuals.widgets.active.bg_fill = Color32::from_rgb(108, 159, 255);
    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.menu_corner_radius = CornerRadius::same(4);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [2, 3],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(96),
    };
    visuals.indent_has_left_vline = false;
    visuals.striped = false;
    visuals.slider_trailing_fill = true;
    visuals.collapsing_header_frame = true;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.animation_time = 0.1;
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.indent = 16.0;
    style.spacing.scroll = egui::style::ScrollStyle {
        bar_width: 4.0,
        ..Default::default()
    };
    ctx.set_style(style);
}

pub fn node_type_icon(type_name: &str) -> &'static str {
    let lower = type_name.to_lowercase();
    if lower.contains("projectile") {
        "🟡"
    } else if lower.starts_with("enemy") {
        "🔴"
    } else if lower.starts_with("tower") {
        "🔵"
    } else if lower.starts_with("player") {
        "🟢"
    } else if lower.starts_with("resource") {
        "⚪"
    } else {
        "⬜"
    }
}

pub fn node_type_color(type_name: &str) -> Color32 {
    let lower = type_name.to_lowercase();
    if lower.contains("projectile") {
        Color32::from_rgb(255, 215, 0)
    } else if lower.starts_with("enemy") {
        Color32::from_rgb(233, 69, 96)
    } else if lower.starts_with("tower") {
        Color32::from_rgb(108, 159, 255)
    } else if lower.starts_with("player") {
        Color32::from_rgb(15, 155, 88)
    } else {
        Color32::from_rgb(160, 160, 160)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enemy_type_gets_red_icon() {
        assert_eq!(node_type_icon("Enemy_Basic"), "🔴");
        assert_eq!(node_type_icon("enemy_fast"), "🔴");
    }

    #[test]
    fn tower_type_gets_blue_icon() {
        assert_eq!(node_type_icon("Tower_Arrow"), "🔵");
    }

    #[test]
    fn unknown_type_gets_default_icon() {
        assert_eq!(node_type_icon("something_else"), "⬜");
    }

    #[test]
    fn projectile_gets_yellow() {
        assert_eq!(node_type_icon("enemy_projectile"), "🟡");
        assert_eq!(
            node_type_color("enemy_projectile"),
            Color32::from_rgb(255, 215, 0)
        );
    }
}
