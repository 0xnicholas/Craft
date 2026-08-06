use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error(
        "invalid palette color {0:?}: expected a single hex color like #6C9FFF (or null for transparent)"
    )]
    BadColor(String),
    #[error("palette key {0:?} must be a single character")]
    BadKey(String),
    #[error("row {row} of frame {frame} has {len} characters, expected {expected}")]
    RaggedRow {
        frame: usize,
        row: usize,
        len: usize,
        expected: usize,
    },
    #[error("character {ch:?} in frame {frame} is not defined in the palette")]
    UnknownChar { ch: char, frame: usize },
    #[error("scale must be at least 1, got {0}")]
    ZeroScale(u32),
    #[error("pixel map has no frames")]
    NoFrames,
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("png encode failed: {0}")]
    Png(String),
    #[error("atlas dimensions {width}x{height} overflow the pixel budget")]
    Overflow { width: u64, height: u64 },
}

#[derive(Debug, Deserialize)]
struct PixelMapFile {
    palette: BTreeMap<String, Option<String>>,
    frames: Vec<FrameFile>,
    scale: u32,
}

#[derive(Debug, Deserialize)]
struct FrameFile {
    rows: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PixelMap {
    palette: BTreeMap<char, Option<[u8; 3]>>,
    frames: Vec<Vec<String>>,
    scale: u32,
}

impl PixelMap {
    pub fn palette(&self) -> &BTreeMap<char, Option<[u8; 3]>> {
        &self.palette
    }

    pub fn frames(&self) -> &[Vec<String>] {
        &self.frames
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }
}

pub fn parse_pixel_map(json: &str) -> Result<PixelMap, AssetError> {
    let file: PixelMapFile = serde_json::from_str(json)?;
    if file.frames.is_empty() {
        return Err(AssetError::NoFrames);
    }
    if file.scale == 0 {
        return Err(AssetError::ZeroScale(file.scale));
    }
    let mut palette = BTreeMap::new();
    for (key, color) in &file.palette {
        let mut chars = key.chars();
        let (Some(ch), None) = (chars.next(), chars.next()) else {
            return Err(AssetError::BadKey(key.clone()));
        };
        palette.insert(ch, color.as_deref().map(parse_hex_color).transpose()?);
    }
    let frames = file
        .frames
        .iter()
        .map(|f| validate_rows(&f.rows, &palette))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PixelMap {
        palette,
        frames,
        scale: file.scale,
    })
}

fn parse_hex_color(s: &str) -> Result<[u8; 3], AssetError> {
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| AssetError::BadColor(s.to_string()))?;
    let parse = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16).map_err(|_| AssetError::BadColor(s.to_string()))
    };
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)
                .map_err(|_| AssetError::BadColor(s.to_string()))?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)
                .map_err(|_| AssetError::BadColor(s.to_string()))?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)
                .map_err(|_| AssetError::BadColor(s.to_string()))?;
            Ok([r, g, b])
        }
        6 => Ok([parse(0..2)?, parse(2..4)?, parse(4..6)?]),
        _ => Err(AssetError::BadColor(s.to_string())),
    }
}

fn validate_rows(
    rows: &[String],
    palette: &BTreeMap<char, Option<[u8; 3]>>,
) -> Result<Vec<String>, AssetError> {
    let mut validated: Vec<String> = Vec::with_capacity(rows.len());
    let width = rows
        .first()
        .ok_or(AssetError::RaggedRow {
            frame: 0,
            row: 0,
            len: 0,
            expected: 0,
        })?
        .len();
    for (i, row) in rows.iter().enumerate() {
        if row.len() != width {
            return Err(AssetError::RaggedRow {
                frame: 0,
                row: i,
                len: row.len(),
                expected: width,
            });
        }
        for ch in row.chars() {
            if !palette.contains_key(&ch) {
                return Err(AssetError::UnknownChar { ch, frame: 0 });
            }
        }
        validated.push(row.clone());
    }
    Ok(validated)
}

#[derive(Debug, Clone)]
pub struct Atlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub frame_rects: Vec<[u32; 4]>,
}

pub fn build_atlas(pm: &PixelMap) -> Result<Atlas, AssetError> {
    let mut frames: Vec<(u32, u32, Vec<u8>)> = Vec::with_capacity(pm.frames.len());
    for rows in &pm.frames {
        let w_chars = rows.first().map_or(0, |r| r.len());
        let h_chars = rows.len();
        let w = (w_chars as u64) * (pm.scale as u64);
        let h = (h_chars as u64) * (pm.scale as u64);
        let total = w * h * 4;
        if total > u32::MAX as u64 {
            return Err(AssetError::Overflow {
                width: w,
                height: h,
            });
        }
        frames.push((w as u32, h as u32, rasterize(rows, pm)?));
    }
    let width: u64 = frames.iter().map(|(w, _, _)| *w as u64).sum();
    let height: u64 = frames.iter().map(|(_, h, _)| *h as u64).max().unwrap_or(0);
    if width * height * 4 > u32::MAX as u64 {
        return Err(AssetError::Overflow { width, height });
    }
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut x = 0u32;
    let mut rects = Vec::with_capacity(frames.len());
    for (w, h, frame) in frames {
        for y in 0..h {
            let src = (y * w * 4) as usize;
            let dst = ((y as u64 * height + x as u64) * 4) as usize;
            pixels[dst..dst + (w * 4) as usize]
                .copy_from_slice(&frame[src..src + (w * 4) as usize]);
        }
        rects.push([x, 0, w, h]);
        x += w;
    }
    Ok(Atlas {
        width: width as u32,
        height: height as u32,
        pixels,
        frame_rects: rects,
    })
}

fn rasterize(rows: &[String], pm: &PixelMap) -> Result<Vec<u8>, AssetError> {
    let w = rows[0].len() as u32;
    let h = rows.len() as u32;
    let s = pm.scale;
    let mut out = vec![0u8; (w * h * s * s * 4) as usize];
    for (y, row) in rows.iter().enumerate() {
        for (cx, ch) in row.chars().enumerate() {
            let color = match pm.palette.get(&ch) {
                Some(Some(rgb)) => [rgb[0], rgb[1], rgb[2], 255],
                Some(None) => [0, 0, 0, 0],
                None => return Err(AssetError::UnknownChar { ch, frame: 0 }),
            };
            for dy in 0..s {
                for dx in 0..s {
                    let x = (cx as u32) * s + dx;
                    let y = (y as u32) * s + dy;
                    let i = ((y * w * s + x) * 4) as usize;
                    out[i..i + 4].copy_from_slice(&color);
                }
            }
        }
    }
    Ok(out)
}

impl Atlas {
    pub fn encode_png(&self) -> Result<Vec<u8>, AssetError> {
        let img = image::RgbaImage::from_raw(self.width, self.height, self.pixels.clone()).ok_or(
            AssetError::Overflow {
                width: self.width as u64,
                height: self.height as u64,
            },
        )?;
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| AssetError::Png(e.to_string()))?;
        Ok(buf)
    }

    pub fn atlas_metadata(&self) -> String {
        serde_json::json!({ "frames": self.frame_rects }).to_string()
    }
}

#[derive(Debug)]
pub struct GeneratedFiles {
    pub png_path: PathBuf,
    pub atlas_path: PathBuf,
}

pub fn generate_dir(input: &Path, out_dir: &Path) -> Result<GeneratedFiles, AssetError> {
    let json = std::fs::read_to_string(input)?;
    let pm = parse_pixel_map(&json)?;
    let atlas = build_atlas(&pm)?;
    let stem = input
        .file_stem()
        .ok_or_else(|| {
            AssetError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "input path has no file name",
            ))
        })?
        .to_string_lossy()
        .into_owned();
    let png_path = out_dir.join(format!("{stem}.png"));
    let atlas_path = out_dir.join(format!("{stem}.atlas.json"));
    std::fs::write(&png_path, atlas.encode_png()?)?;
    std::fs::write(&atlas_path, atlas.atlas_metadata())?;
    Ok(GeneratedFiles {
        png_path,
        atlas_path,
    })
}
