use craft_assets::{ToneKind, wav_tone};

#[test]
fn all_three_tones_generate() {
    for kind in [ToneKind::Jump, ToneKind::Coin, ToneKind::Win] {
        let wav = wav_tone(kind).expect("wav");
        assert!(!wav.is_empty(), "{kind:?} wav is non-empty");
        assert_eq!(
            wav_tone(kind).expect("again"),
            wav,
            "{kind:?} is deterministic"
        );
    }
}

#[test]
fn wav_header_is_valid_pcm16() {
    let wav = wav_tone(ToneKind::Jump).expect("wav");
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[12..16], b"fmt ");
    let audio_format = u16::from_le_bytes([wav[20], wav[21]]);
    assert_eq!(audio_format, 1, "PCM");
    let channels = u16::from_le_bytes([wav[22], wav[23]]);
    assert_eq!(channels, 1, "mono");
    let sample_rate = u32::from_le_bytes(wav[24..28].try_into().expect("slice"));
    assert_eq!(sample_rate, 22050);
    let bits = u16::from_le_bytes([wav[34], wav[35]]);
    assert_eq!(bits, 16);
    assert_eq!(&wav[36..40], b"data");
}

#[test]
fn wav_samples_are_audible_and_bounded() {
    let wav = wav_tone(ToneKind::Coin).expect("wav");
    let data_len = u32::from_le_bytes(wav[40..44].try_into().expect("slice")) as usize;
    let samples: Vec<i16> = wav[44..44 + data_len]
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    assert!(!samples.is_empty());
    let peak = samples
        .iter()
        .map(|s| s.unsigned_abs())
        .max()
        .expect("peak");
    assert!(peak > 1000, "audible amplitude, got {peak}");
    assert!(peak < 20000, "no clipping, got {peak}");
}

#[test]
fn wav_frequency_matches_expected_tone() {
    // Coin opens on E6 (~1318 Hz): zero-crossings per second should be
    // close to 2 * f for the first 60 ms.
    let wav = wav_tone(ToneKind::Coin).expect("wav");
    let data_len = u32::from_le_bytes(wav[40..44].try_into().expect("slice")) as usize;
    let samples: Vec<i16> = wav[44..44 + data_len]
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    let window = 22050 / 15; // ~67 ms of the first segment
    let crossings = samples[..window]
        .windows(2)
        .filter(|w| (w[0] < 0) != (w[1] < 0))
        .count();
    let freq = crossings as f64 * 22050.0 / (2.0 * window as f64);
    assert!(
        (1100.0..1600.0).contains(&freq),
        "expected ~1318 Hz, measured {freq:.0} Hz"
    );
}

#[test]
fn wav_lengths_are_short_ui_effects() {
    for kind in [ToneKind::Jump, ToneKind::Win] {
        let wav = wav_tone(kind).expect("wav");
        let data_len = u32::from_le_bytes(wav[40..44].try_into().expect("slice")) as usize;
        let seconds = data_len as f64 / 2.0 / 22050.0;
        assert!(
            seconds < 1.0,
            "{kind:?} is {seconds:.2}s — must be a short UI effect"
        );
    }
}
