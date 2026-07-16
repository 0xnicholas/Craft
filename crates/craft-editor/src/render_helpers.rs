use craft_kernel::Viewport;

pub fn glyph_for_type(type_name: &str) -> char {
    match type_name {
        "Player" | "BRPlayer" | "M7Player" | "HRPlayer" | "Probe" => '@',
        "Enemy" | "HREnemy" | "Mob" | "Bullet" => '*',
        "Tower" | "Spawner" => 'T',
        t if t.ends_with("Counter") => '#',
        t if t.ends_with("Enemy") => '*',
        t if t.ends_with("Player") => '@',
        _ => '.',
    }
}

pub fn project_to_screen(x: f64, y: f64, viewport: Viewport) -> Option<(usize, usize)> {
    let w = viewport.width as usize;
    let h = viewport.height as usize;
    let half_w = w as f64 / 2.0;
    let half_h = h as f64 / 2.0;
    let sx = (x + half_w).round() as i64;
    let sy = (-(y) + half_h).round() as i64;
    if sx < 0 || sy < 0 {
        return None;
    }
    let sx = sx as usize;
    let sy = sy as usize;
    if sx >= w || sy >= h {
        return None;
    }
    Some((sx, sy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_for_player_is_at() {
        assert_eq!(glyph_for_type("Player"), '@');
        assert_eq!(glyph_for_type("M7Player"), '@');
    }

    #[test]
    fn glyph_for_unknown_is_dot() {
        assert_eq!(glyph_for_type("Foobar"), '.');
    }

    #[test]
    fn project_origin_is_center() {
        let vp = Viewport::new(80, 24);
        let (x, y) = project_to_screen(0.0, 0.0, vp).unwrap();
        assert_eq!((x, y), (40, 12));
    }

    #[test]
    fn project_clamp_offscreen_returns_none() {
        let vp = Viewport::new(80, 24);
        assert!(project_to_screen(1000.0, 1000.0, vp).is_none());
    }
}
