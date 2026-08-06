use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = r####"{
  "palette": { ".": null, "#": "#6C9FFF", "e": "#E74C3C" },
  "frames": [
    { "rows": ["###", "#.#", "###"] },
    { "rows": ["###", ".#.", "###"] },
    { "rows": ["###", "###", "###"] }
  ],
  "scale": 4
}"####;

fn run_cli(input: &PathBuf, out: &PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_craft-assets"))
        .arg(input)
        .arg(out)
        .output()
        .expect("cli runs")
}

#[test]
fn cli_generates_atlas_and_metadata() {
    let dir = std::env::temp_dir().join(format!("craft-assets-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");
    let input = dir.join("hero.json");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("out");
    std::fs::write(&input, INPUT).expect("input");

    let run = run_cli(&input, &out);
    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(out.join("hero.png").exists());
    assert!(out.join("hero.atlas.json").exists());

    let png = std::fs::read(out.join("hero.png")).expect("png");
    let img = image::load_from_memory(&png).expect("decode").to_rgba8();
    assert_eq!(
        img.dimensions(),
        (36, 12),
        "three 12x12 frames packed horizontally"
    );

    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("hero.atlas.json")).expect("meta"))
            .expect("json");
    assert_eq!(
        meta["frames"],
        serde_json::json!([[0, 0, 12, 12], [12, 0, 12, 12], [24, 0, 12, 12]])
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_output_is_byte_identical_across_runs() {
    let dir = std::env::temp_dir().join(format!("craft-assets-det-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");
    let input = dir.join("hero.json");
    std::fs::create_dir_all(dir.join("a")).expect("a");
    std::fs::create_dir_all(dir.join("b")).expect("b");
    std::fs::write(&input, INPUT).expect("input");

    let first = run_cli(&input, &dir.join("a"));
    let second = run_cli(&input, &dir.join("b"));
    assert!(first.status.success() && second.status.success());

    for name in ["hero.png", "hero.atlas.json"] {
        let a = std::fs::read(dir.join("a").join(name)).expect("a file");
        let b = std::fs::read(dir.join("b").join(name)).expect("b file");
        assert_eq!(a, b, "{name} must be byte-identical across runs");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_errors_on_invalid_input_without_panicking() {
    let dir = std::env::temp_dir().join(format!("craft-assets-cli-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");
    let input = dir.join("bad.json");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("out");
    std::fs::write(
        &input,
        r####"{"palette": {"#": "#ZZZZZZ"}, "frames": [{"rows": ["#"]}], "scale": 1}"####,
    )
    .expect("input");

    let run = run_cli(&input, &out);
    assert!(!run.status.success(), "invalid input must fail");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("craft-assets:"),
        "stderr mentions the tool: {stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_font_subcommand_emits_font_assets() {
    let dir = std::env::temp_dir().join(format!("craft-assets-cli-font-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");

    let run = Command::new(env!("CARGO_BIN_EXE_craft-assets"))
        .arg("font")
        .arg(&dir)
        .output()
        .expect("cli runs");
    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let png = std::fs::read(dir.join("font.png")).expect("font.png");
    let img = image::load_from_memory(&png).expect("decode").to_rgba8();
    assert_eq!(img.dimensions(), (270, 7), "54 glyphs x 5px");
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("font.json")).expect("font.json"))
            .expect("json");
    assert_eq!(meta["glyphs"]["A"], serde_json::json!([0, 0, 5, 7]));
    assert_eq!(meta["glyphs"]["0"], serde_json::json!([130, 0, 5, 7]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_wav_subcommand_emits_three_tones() {
    let dir = std::env::temp_dir().join(format!("craft-assets-cli-wav-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");

    let run = Command::new(env!("CARGO_BIN_EXE_craft-assets"))
        .arg("wav")
        .arg(&dir)
        .output()
        .expect("cli runs");
    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    for name in ["jump.wav", "coin.wav", "win.wav"] {
        let wav = std::fs::read(dir.join(name)).expect(name);
        assert_eq!(&wav[0..4], b"RIFF", "{name} header");
        assert_eq!(&wav[8..12], b"WAVE", "{name} format");
        let data_len = u32::from_le_bytes(wav[40..44].try_into().expect("len"));
        assert!(data_len > 1000, "{name} has samples");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
