use std::path::PathBuf;

use craft_assets::{AssetError, build_atlas, generate_dir, parse_pixel_map};

const BASIC: &str = r####"{
  "palette": { ".": null, "#": "#6C9FFF", "e": "#E74C3C" },
  "frames": [
    { "rows": ["###", "#.#", "###"] }
  ],
  "scale": 4
}"####;

const TWO_FRAMES: &str = r####"{
  "palette": { ".": null, "#": "#6C9FFF" },
  "frames": [
    { "rows": ["###", "#.#", "###"] },
    { "rows": ["###", ".#.", "###"] }
  ],
  "scale": 4
}"####;

#[test]
fn parses_valid_pixel_map() {
    let pm = parse_pixel_map(BASIC).expect("valid pixel map");
    assert_eq!(pm.scale(), 4);
    assert_eq!(pm.frames().len(), 1);
    assert_eq!(pm.frames()[0].len(), 3);
    assert_eq!(pm.palette().get(&'#'), Some(&Some([0x6C, 0x9F, 0xFF])));
    assert_eq!(pm.palette().get(&'.'), Some(&None));
}

#[test]
fn accepts_three_digit_hex_colors() {
    let json = r####"{
      "palette": { "#": "#FFF", "r": "#f00" },
      "frames": [{ "rows": ["#r"] }],
      "scale": 1
    }"####;
    let pm = parse_pixel_map(json).expect("3-digit hex ok");
    assert_eq!(pm.palette().get(&'#'), Some(&Some([255, 255, 255])));
    assert_eq!(pm.palette().get(&'r'), Some(&Some([255, 0, 0])));
}

#[test]
fn rejects_bad_palette_hex() {
    let json = r####"{
      "palette": { "#": "#GGGGGG" },
      "frames": [{ "rows": ["#"] }],
      "scale": 1
    }"####;
    let err = parse_pixel_map(json).expect_err("bad hex");
    assert!(matches!(err, AssetError::BadColor(_)), "{err}");
}

#[test]
fn rejects_multi_char_palette_keys() {
    let json = r####"{
      "palette": { "##": "#000000" },
      "frames": [{ "rows": ["#"] }],
      "scale": 1
    }"####;
    let err = parse_pixel_map(json).expect_err("two-char key");
    assert!(matches!(err, AssetError::BadKey(_)), "{err}");
}

#[test]
fn rejects_ragged_rows() {
    let json = r####"{
      "palette": { ".": null, "#": "#000000" },
      "frames": [{ "rows": ["###", "##"] }],
      "scale": 1
    }"####;
    let err = parse_pixel_map(json).expect_err("ragged rows");
    assert!(matches!(err, AssetError::RaggedRow { .. }), "{err}");
}

#[test]
fn rejects_chars_missing_from_palette() {
    let json = r####"{
      "palette": { "#": "#000000" },
      "frames": [{ "rows": ["#x"] }],
      "scale": 1
    }"####;
    let err = parse_pixel_map(json).expect_err("x not in palette");
    assert!(
        matches!(err, AssetError::UnknownChar { ch: 'x', .. }),
        "{err}"
    );
}

#[test]
fn rejects_zero_scale() {
    let json = r####"{
      "palette": { "#": "#000000" },
      "frames": [{ "rows": ["#"] }],
      "scale": 0
    }"####;
    let err = parse_pixel_map(json).expect_err("scale 0");
    assert!(matches!(err, AssetError::ZeroScale(0)), "{err}");
}

#[test]
fn rejects_empty_frames() {
    let json = r####"{
      "palette": { "#": "#000000" },
      "frames": [],
      "scale": 1
    }"####;
    let err = parse_pixel_map(json).expect_err("no frames");
    assert!(matches!(err, AssetError::NoFrames), "{err}");
}

#[test]
fn builds_single_frame_atlas_with_scaled_pixels() {
    let pm = parse_pixel_map(BASIC).expect("valid");
    let atlas = build_atlas(&pm).expect("build");
    assert_eq!([atlas.width, atlas.height], [12, 12]);
    assert_eq!(atlas.frame_rects, vec![[0, 0, 12, 12]]);
    // Corner '#': #6C9FFF opaque.
    let px = |x: u32, y: u32| {
        let i = ((y * 12 + x) * 4) as usize;
        atlas.pixels[i..i + 4].to_vec()
    };
    assert_eq!(px(0, 0), vec![0x6C, 0x9F, 0xFF, 255]);
    assert_eq!(px(11, 11), vec![0x6C, 0x9F, 0xFF, 255]);
    // Center '.': transparent.
    assert_eq!(px(6, 6), vec![0, 0, 0, 0]);
}

#[test]
fn packs_frames_horizontally() {
    let pm = parse_pixel_map(TWO_FRAMES).expect("valid");
    let atlas = build_atlas(&pm).expect("build");
    assert_eq!([atlas.width, atlas.height], [24, 12]);
    assert_eq!(atlas.frame_rects, vec![[0, 0, 12, 12], [12, 0, 12, 12]]);
    // Second frame's left edge pixel is its top-left '#'.
    let i = (12 * 4) as usize;
    assert_eq!(atlas.pixels[i..i + 4], [0x6C, 0x9F, 0xFF, 255]);
}

#[test]
fn packs_uneven_frame_sizes() {
    let json = r####"{
      "palette": { "#": "#000000" },
      "frames": [
        { "rows": ["##", "##"] },
        { "rows": ["#", "#", "#"] }
      ],
      "scale": 2
    }"####;
    let pm = parse_pixel_map(json).expect("valid");
    let atlas = build_atlas(&pm).expect("build");
    assert_eq!([atlas.width, atlas.height], [6, 6]);
    assert_eq!(atlas.frame_rects, vec![[0, 0, 4, 4], [4, 0, 2, 6]]);
}

#[test]
fn png_encoding_is_deterministic() {
    let pm = parse_pixel_map(TWO_FRAMES).expect("valid");
    let atlas = build_atlas(&pm).expect("build");
    let a = atlas.encode_png().expect("png");
    let b = atlas.encode_png().expect("png");
    assert_eq!(a, b, "two encodes must be byte-identical");
    assert!(!a.is_empty());
}

#[test]
fn png_decodes_at_expected_dimensions() {
    let pm = parse_pixel_map(BASIC).expect("valid");
    let atlas = build_atlas(&pm).expect("build");
    let png = atlas.encode_png().expect("png");
    let img = image::load_from_memory(&png).expect("decode").to_rgba8();
    assert_eq!(img.dimensions(), (12, 12));
    let px = img.get_pixel(6, 6).0;
    assert_eq!(
        px,
        [0, 0, 0, 0],
        "transparent center survives the round-trip"
    );
    let px = img.get_pixel(0, 0).0;
    assert_eq!(px, [0x6C, 0x9F, 0xFF, 255]);
}

#[test]
fn atlas_metadata_emits_frame_rects() {
    let pm = parse_pixel_map(TWO_FRAMES).expect("valid");
    let atlas = build_atlas(&pm).expect("build");
    let meta: serde_json::Value = serde_json::from_str(&atlas.atlas_metadata()).expect("json");
    assert_eq!(
        meta["frames"],
        serde_json::json!([[0, 0, 12, 12], [12, 0, 12, 12]])
    );
}

#[test]
fn generate_dir_writes_png_and_atlas() {
    let dir = std::env::temp_dir().join(format!("craft-assets-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let input = dir.join("hero.json");
    std::fs::write(&input, TWO_FRAMES).expect("write input");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("out dir");
    let files = generate_dir(&input, &out).expect("generate");
    assert!(files.png_path.exists());
    assert!(files.atlas_path.exists());
    let png = std::fs::read(&files.png_path).expect("read png");
    let img = image::load_from_memory(&png).expect("decode").to_rgba8();
    assert_eq!(img.dimensions(), (24, 12));
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&files.atlas_path).expect("read meta"))
            .expect("meta json");
    assert_eq!(
        meta["frames"],
        serde_json::json!([[0, 0, 12, 12], [12, 0, 12, 12]])
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn generate_dir_rejects_invalid_input_without_panicking() {
    let dir = std::env::temp_dir().join(format!("craft-assets-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let input = dir.join("bad.json");
    std::fs::write(
        &input,
        r####"{"palette": {"#": "#GG"}, "frames": [{"rows": ["#"]}], "scale": 1}"####,
    )
    .expect("write");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("out dir");
    let err = generate_dir(&input, &out).expect_err("bad input must error");
    assert!(matches!(err, AssetError::BadColor(_)), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unknown_asset_path_errors() {
    let dir = PathBuf::from("/nonexistent/craft-assets-input.json");
    let err = generate_dir(&dir, &dir).expect_err("missing file");
    assert!(matches!(err, AssetError::Io(_)), "{err}");
}
