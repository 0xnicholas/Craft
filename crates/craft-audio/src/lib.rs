use std::collections::HashSet;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;

craft_kernel::craft_system!(AudioSystem, phase: PostTick, {
    let asset_root = std::env::var("CRAFT_ASSET_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let signal_names = ctx.bus.pending_signal_names();
    let mut played: HashSet<String> = HashSet::new();

    for name in &signal_names {
        if played.contains(name) {
            continue;
        }
        played.insert(name.clone());

        let wav_path = asset_root.join(format!("{name}.wav"));
        let ogg_path = asset_root.join(format!("{name}.ogg"));
        let audio_path = if wav_path.exists() {
            wav_path
        } else if ogg_path.exists() {
            ogg_path
        } else {
            continue;
        };

        let file = match fs::File::open(&audio_path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = BufReader::new(file);
        if let Ok(source) = rodio::Decoder::new(reader)
            && let Ok((_stream, handle)) = rodio::OutputStream::try_default()
            && let Ok(sink) = rodio::Sink::try_new(&handle)
        {
            sink.append(source);
            sink.detach();
        }
    }
});

#[cfg(test)]
mod tests {
    #[test]
    fn system_is_registered() {
        let engine = craft_kernel::Engine::new();
        let names: Vec<&str> = engine.list_systems().iter().map(|s| s.name).collect();
        assert!(
            names.contains(&"AudioSystem"),
            "AudioSystem must be registered. Found: {names:?}"
        );
    }
}
