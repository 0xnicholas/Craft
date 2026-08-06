use std::collections::HashSet;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use craft_kernel::serde_json::Value;
use rodio::Source;

#[derive(Debug, Clone, PartialEq)]
pub struct AudioRequest {
    pub path: PathBuf,
    pub volume: f32,
    pub loop_: bool,
}

/// Resolve a pending signal to a playable audio file. `volume` and `loop`
/// come from the signal payload (`{"volume": 0.5, "loop": true}`); a
/// payload without them behaves exactly like pre-payload signals.
pub fn audio_request(signal: &str, payload: &Value, asset_root: &Path) -> Option<AudioRequest> {
    let wav_path = asset_root.join(format!("{signal}.wav"));
    let ogg_path = asset_root.join(format!("{signal}.ogg"));
    let path = if wav_path.exists() {
        wav_path
    } else if ogg_path.exists() {
        ogg_path
    } else {
        return None;
    };
    let volume = payload
        .get("volume")
        .and_then(|v| v.as_f64())
        .map(|v| v.clamp(0.0, 1.0) as f32)
        .unwrap_or(1.0);
    let loop_ = payload
        .get("loop")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some(AudioRequest {
        path,
        volume,
        loop_,
    })
}

craft_kernel::craft_system!(AudioSystem, phase: PostTick, {
    let asset_root = std::env::var("CRAFT_ASSET_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let pending: Vec<(String, Value)> = ctx.pending_signals.clone();
    let mut played: HashSet<String> = HashSet::new();

    for (name, payload) in &pending {
        if played.contains(name) {
            continue;
        }
        played.insert(name.clone());

        let Some(req) = audio_request(name, payload, &asset_root) else {
            continue;
        };

        let file = match fs::File::open(&req.path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = BufReader::new(file);
        if let Ok(source) = rodio::Decoder::new(reader) {
            let output: Box<dyn Source<Item = i16> + Send> = if req.loop_ {
                Box::new(source.repeat_infinite())
            } else {
                Box::new(source)
            };
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default()
                && let Ok(sink) = rodio::Sink::try_new(&handle)
            {
                sink.set_volume(req.volume);
                sink.append(output);
                sink.detach();
            }
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
