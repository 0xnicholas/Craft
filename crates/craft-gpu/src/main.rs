use std::env;
use std::path::PathBuf;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: craft-gpu <scene.json> [--asset-root <path>]");
        std::process::exit(1);
    }
    let scene_path = PathBuf::from(&args[1]);

    let asset_root = args
        .iter()
        .position(|a| a == "--asset-root")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            scene_path
                .parent()
                .unwrap_or(&PathBuf::from("."))
                .join("assets")
        });

    let config = craft_gpu::GameWindowConfig {
        title: "Craft Game".into(),
        width: 960,
        height: 540,
        tick_hz: 60,
        seed: 0,
        asset_root,
    };

    if let Err(e) = craft_gpu::spawn_game_window(&scene_path, config) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
