use std::path::PathBuf;

use craft_audio::audio_request;
use craft_kernel::serde_json::{Value, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

#[test]
fn payload_volume_and_loop_are_parsed() {
    let req = audio_request("jump", &json!({ "volume": 0.5, "loop": true }), &root())
        .expect("jump.wav exists in testdata");
    assert_eq!(req.path.file_name().unwrap(), "jump.wav");
    assert_eq!(req.volume, 0.5);
    assert!(req.loop_);
}

#[test]
fn no_payload_defaults_to_full_volume_no_loop() {
    let req = audio_request("jump", &Value::Null, &root()).expect("defaults");
    assert_eq!(req.volume, 1.0);
    assert!(!req.loop_);
}

#[test]
fn empty_object_payload_is_legacy_behavior() {
    let req = audio_request("jump", &json!({}), &root()).expect("empty payload");
    assert_eq!(req.volume, 1.0);
    assert!(!req.loop_);
}

#[test]
fn missing_audio_file_returns_none() {
    assert_eq!(
        audio_request("nonexistent_signal", &json!({}), &root()),
        None
    );
}

#[test]
fn ogg_files_are_discovered_too() {
    let req = audio_request("coin", &json!({}), &root()).expect("coin.ogg exists");
    assert_eq!(req.path.file_name().unwrap(), "coin.ogg");
}

#[test]
fn payload_volume_clamped_to_range() {
    let req = audio_request("jump", &json!({ "volume": 3.0 }), &root()).expect("clamped");
    assert_eq!(req.volume, 1.0);
    let req = audio_request("jump", &json!({ "volume": -1.0 }), &root()).expect("clamped");
    assert_eq!(req.volume, 0.0);
}
