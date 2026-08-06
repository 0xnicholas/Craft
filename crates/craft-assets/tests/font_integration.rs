use craft_assets::{GLYPH_ORDER, builtin_font, font_metadata};

#[test]
fn charset_is_covered_by_metadata() {
    let (_png, json) = (builtin_font().expect("font"), font_metadata());
    let meta: serde_json::Value = serde_json::from_str(&json).expect("json");
    let glyphs = meta["glyphs"].as_object().expect("glyphs object");
    for ch in GLYPH_ORDER.chars() {
        assert!(
            glyphs.contains_key(&ch.to_string()),
            "missing glyph for {ch:?}"
        );
    }
    assert_eq!(
        glyphs.len(),
        GLYPH_ORDER.chars().count(),
        "exact charset, no extras"
    );
}

#[test]
fn font_png_decodes_at_expected_dimensions() {
    let png = builtin_font().expect("font");
    let img = image::load_from_memory(&png).expect("decode").to_rgba8();
    let glyphs = GLYPH_ORDER.chars().count();
    assert_eq!(img.dimensions(), ((glyphs * 5) as u32, 7));
    // 'A' is the first glyph: row 0 is "01110", so col 1 is the first lit pixel.
    let px = img.get_pixel(1, 0).0;
    assert_eq!(px[3], 255, "A's first ink pixel is opaque");
    let px = img.get_pixel(0, 0).0;
    assert_eq!(px[3], 0, "A's top-left pixel is transparent");
}

#[test]
fn glyph_rects_step_by_five_pixels() {
    let meta: serde_json::Value = serde_json::from_str(&font_metadata()).expect("json");
    let glyphs = meta["glyphs"].as_object().expect("glyphs");
    for (i, ch) in GLYPH_ORDER.chars().enumerate() {
        let rect = glyphs[&ch.to_string()].as_array().expect("rect");
        let x = rect[0].as_u64().expect("x");
        assert_eq!(x, (i * 5) as u64, "glyph {ch:?} x offset");
        assert_eq!(
            &serde_json::Value::Array(rect.clone()),
            &serde_json::json!([i * 5, 0, 5, 7])
        );
    }
}

#[test]
fn font_is_deterministic() {
    assert_eq!(builtin_font().expect("font"), builtin_font().expect("font"));
}

#[test]
fn every_glyph_has_ink() {
    // No all-blank glyphs (would render as invisible placeholders).
    for (ch, rows) in craft_assets::glyph_rows() {
        if ch == ' ' {
            continue; // space is deliberately blank
        }
        let ink = rows.iter().map(|r| r.count_ones()).sum::<u32>();
        assert!(ink > 0, "glyph {ch:?} is blank");
    }
}

#[test]
fn unknown_chars_are_absent() {
    let meta: serde_json::Value = serde_json::from_str(&font_metadata()).expect("json");
    let glyphs = meta["glyphs"].as_object().expect("glyphs");
    assert!(!glyphs.contains_key("~"), "tilde is not in minimal ASCII");
    assert!(
        !glyphs.contains_key("a"),
        "lowercase is not in minimal ASCII"
    );
}
