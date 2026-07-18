use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    #[allow(dead_code)]
    pub size: [u32; 2],
}

pub struct TextureCache {
    textures: HashMap<String, GpuTexture>,
    missing: HashSet<String>,
    placeholder: Option<GpuTexture>,
    asset_root: PathBuf,
}

impl TextureCache {
    pub fn new(asset_root: PathBuf) -> Self {
        Self {
            textures: HashMap::new(),
            missing: HashSet::new(),
            placeholder: None,
            asset_root,
        }
    }

    pub fn ensure_loaded(&mut self, path: &str, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.textures.contains_key(path) || self.missing.contains(path) {
            return;
        }
        let full_path = self.asset_root.join(path);
        match load_texture(&full_path, device, queue) {
            Ok(tex) => {
                self.textures.insert(path.to_string(), tex);
            }
            Err(_) => {
                log::warn!("texture not found: {}", full_path.display());
                self.missing.insert(path.to_string());
            }
        }
    }

    pub fn get(&self, path: &str) -> &GpuTexture {
        self.textures.get(path).unwrap_or_else(|| {
            self.placeholder
                .as_ref()
                .expect("placeholder not initialized")
        })
    }

    pub fn init_placeholder(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.placeholder = Some(create_magenta_placeholder(device, queue));
    }

    pub fn clear(&mut self) {
        self.textures.clear();
        self.missing.clear();
        self.placeholder = None;
    }
}

fn load_texture(
    path: &Path,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<GpuTexture, String> {
    let img = image::open(path).map_err(|e| format!("{e}"))?;
    let rgba = img.to_rgba8();
    let dimensions = rgba.dimensions();
    let size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(path.to_str().unwrap_or("texture")),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    Ok(GpuTexture {
        texture,
        view,
        sampler,
        size: [dimensions.0, dimensions.1],
    })
}

fn create_magenta_placeholder(device: &wgpu::Device, queue: &wgpu::Queue) -> GpuTexture {
    let pixels: Vec<u8> = vec![
        255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255,
    ];
    let size = wgpu::Extent3d {
        width: 2,
        height: 2,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("magenta placeholder"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(2),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    GpuTexture {
        texture,
        view,
        sampler,
        size: [2, 2],
    }
}
