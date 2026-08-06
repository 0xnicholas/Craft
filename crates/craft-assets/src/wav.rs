use crate::pixel_map::AssetError;

pub const SAMPLE_RATE: u32 = 22050;
const AMPLITUDE: f64 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneKind {
    Jump,
    Coin,
    Win,
}

impl ToneKind {
    pub fn file_name(&self) -> &'static str {
        match self {
            ToneKind::Jump => "jump.wav",
            ToneKind::Coin => "coin.wav",
            ToneKind::Win => "win.wav",
        }
    }
}

/// A complete PCM16 WAV file for the tone.
pub fn wav_tone(kind: ToneKind) -> Result<Vec<u8>, AssetError> {
    let samples: Vec<i16> = match kind {
        ToneKind::Jump => sweep(0.22, 300.0, 900.0, Wave::Square),
        ToneKind::Coin => {
            let mut samples = sweep(0.08, 1318.51, 1318.51, Wave::Triangle);
            samples.extend(sweep(0.16, 2637.02, 2637.02, Wave::Triangle));
            samples
        }
        ToneKind::Win => {
            let mut samples = Vec::new();
            for f in [523.25, 659.26, 783.99, 1046.50] {
                samples.extend(sweep(0.12, f, f, Wave::Square));
            }
            samples
        }
    };
    Ok(wav_from_samples(&samples))
}

#[derive(Clone, Copy)]
enum Wave {
    Square,
    Triangle,
}

fn sweep(duration: f64, f0: f64, f1: f64, wave: Wave) -> Vec<i16> {
    let n = (duration * SAMPLE_RATE as f64) as usize;
    let mut samples = Vec::with_capacity(n);
    let mut phase = 0.0;
    for i in 0..n {
        let t = i as f64 / n as f64;
        let freq = f0 + (f1 - f0) * t;
        phase += freq / SAMPLE_RATE as f64;
        phase %= 1.0;
        let shape = match wave {
            Wave::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
        };
        let envelope = 1.0 - t * 0.9;
        let sample = shape * envelope * AMPLITUDE * i16::MAX as f64;
        samples.push(sample as i16);
    }
    samples
}

fn wav_from_samples(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}
