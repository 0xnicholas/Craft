mod shaders;
mod texture;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use wgpu::util::DeviceExt;

use craft_kernel::error::EngineResult;
use craft_kernel::render::{
    self, CameraInfo, ComponentView, Render, RenderCapabilities, SpriteDrawCommand, Viewport,
};
use craft_kernel::{Engine, EngineConfig, Scene};
use texture::TextureCache;
use winit::window::Window;

pub struct GameWindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub tick_hz: u32,
    pub seed: u64,
    pub asset_root: PathBuf,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    transform: [[f32; 4]; 4],
    src_rect: [f32; 4],
    modulate: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update(&mut self, camera: &CameraInfo, viewport_size: [f32; 2]) {
        let zoom = camera.zoom as f32;
        let pos = [camera.position.0 as f32, camera.position.1 as f32];
        let half_w = viewport_size[0] / (2.0 * zoom);
        let half_h = viewport_size[1] / (2.0 * zoom);
        let proj = glam::Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, -1.0, 1.0);
        let view = glam::Mat4::from_translation(glam::Vec3::new(-pos[0], -pos[1], 0.0));
        self.view_proj = (proj * view).to_cols_array_2d();
    }
}

pub struct GpuRenderer {
    viewport: Viewport,
    #[allow(dead_code)]
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    texture_cache: TextureCache,
    sprite_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    camera_uniform: CameraUniform,
}

impl GpuRenderer {
    pub async fn new(window: Arc<Window>, asset_root: PathBuf) -> EngineResult<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(Arc::clone(&window)).map_err(|e| {
            craft_kernel::error::EngineError::Internal(format!(
                "failed to create wgpu surface: {e}"
            ))
        })?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or_else(|| {
                craft_kernel::error::EngineError::Internal("no suitable GPU adapter found".into())
            })?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(|e| {
                craft_kernel::error::EngineError::Internal(format!(
                    "failed to create wgpu device: {e}"
                ))
            })?;
        let surface_caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let sprite_pipeline = shaders::create_sprite_pipeline(&device, config.format);

        let vertex_buffer = shaders::create_vertex_buffer(&device);
        let index_buffer = shaders::create_index_buffer(&device);
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite instance buffer"),
            size: (std::mem::size_of::<InstanceData>() * 1024) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniform"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let mut texture_cache = TextureCache::new(asset_root);
        texture_cache.init_placeholder(&device, &queue);

        Ok(Self {
            viewport: Viewport::new(size.width, size.height),
            window,
            surface,
            device,
            queue,
            config,
            size,
            texture_cache,
            sprite_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout,
            camera_uniform,
        })
    }

    fn group_into_batches(commands: &[SpriteDrawCommand]) -> Vec<(String, Vec<usize>)> {
        let mut seen = std::collections::HashSet::new();
        let mut order: Vec<&str> = Vec::new();
        for cmd in commands {
            if seen.insert(cmd.texture_id.as_str()) {
                order.push(&cmd.texture_id);
            }
        }
        order
            .into_iter()
            .map(|tid| {
                let indices: Vec<usize> = commands
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.texture_id == tid)
                    .map(|(i, _)| i)
                    .collect();
                (tid.to_string(), indices)
            })
            .collect()
    }
}

impl Render for GpuRenderer {
    fn render(&mut self, components: &[ComponentView], _tick: u64) {
        let camera_info = render::extract_camera(components).unwrap_or(CameraInfo {
            position: (0.0, 0.0),
            zoom: 1.0,
            follow: None,
        });
        self.camera_uniform.update(
            &camera_info,
            [self.size.width as f32, self.size.height as f32],
        );
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        let mut commands: Vec<SpriteDrawCommand> = components
            .iter()
            .filter_map(|v| v.sprite_draw_command())
            .collect();
        commands.sort_by_key(|c| c.z_index);

        let mut instances: Vec<InstanceData> = commands
            .iter()
            .map(|cmd| {
                let mut src_rect = cmd.src_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]);
                // Normalize texel coordinates to UV if needed.
                // If src_rect components are > 1.0, treat as texels and divide by texture size.
                if src_rect[2] > 1.0 || src_rect[3] > 1.0 {
                    if let Some(tex) = self.texture_cache.get(&cmd.texture_id) {
                        let tw = tex.size[0] as f32;
                        let th = tex.size[1] as f32;
                        if tw > 0.0 && th > 0.0 {
                            src_rect = [
                                src_rect[0] / tw,
                                src_rect[1] / th,
                                src_rect[2] / tw,
                                src_rect[3] / th,
                            ];
                        }
                    }
                }
                let transform = glam::Mat4::from_scale_rotation_translation(
                    glam::Vec3::new(cmd.scale[0], cmd.scale[1], 1.0),
                    glam::Quat::from_rotation_z(cmd.rotation),
                    glam::Vec3::new(cmd.position[0], cmd.position[1], 0.0),
                );
                InstanceData {
                    transform: transform.to_cols_array_2d(),
                    src_rect,
                    modulate: cmd.modulate,
                }
            })
            .collect();

        if instances.len() > 1024 {
            log::warn!(
                "too many sprites ({}), truncating to 1024; increase instance buffer size",
                instances.len()
            );
            instances.truncate(1024);
        }

        self.queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(_) => return,
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let batches = Self::group_into_batches(&commands);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("craft render"),
            });
        {
            // Pre-load all needed textures before creating the render pass
            for cmd in &commands {
                self.texture_cache
                    .ensure_loaded(&cmd.texture_id, &self.device, &self.queue);
            }

            // Pre-create bind groups for each texture batch
            let batch_bind_groups: Vec<(Vec<usize>, wgpu::BindGroup)> = batches
                .iter()
                .filter_map(|(tid, indices)| {
                    self.texture_cache.get(tid).map(|tex| {
                    let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("texture bind group"),
                        layout: &self.texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&tex.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&tex.sampler),
                            },
                        ],
                    });
                    (indices.clone(), bg)
                    })
                })
                .collect();

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.sprite_pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            for (indices, bind_group) in &batch_bind_groups {
                pass.set_bind_group(1, bind_group, &[]);
                for &idx in indices {
                    pass.draw_indexed(0..6, 0, idx as u32..(idx as u32 + 1));
                }
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn viewport(&self) -> Viewport {
        self.viewport
    }

    fn resize(&mut self, viewport: Viewport) {
        self.viewport = viewport;
        self.size = winit::dpi::PhysicalSize::new(viewport.width, viewport.height);
        self.config.width = self.size.width;
        self.config.height = self.size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn shutdown(&mut self) {
        self.texture_cache.clear();
    }

    fn capabilities(&self) -> RenderCapabilities {
        RenderCapabilities::TEXT | RenderCapabilities::SPRITE
    }
}

use std::time::{Duration, Instant};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;

#[allow(deprecated)]
pub fn spawn_game_window(scene_path: &Path, config: GameWindowConfig) -> EngineResult<()> {
    let event_loop = EventLoop::new().map_err(|e| {
        craft_kernel::error::EngineError::Internal(format!("failed to create event loop: {e}"))
    })?;
    let window = Arc::new(
        event_loop
            .create_window(winit::window::WindowAttributes::default())
            .map_err(|e| {
                craft_kernel::error::EngineError::Internal(format!("failed to create window: {e}"))
            })?,
    );
    window.set_title(&config.title);
    let _ = window.request_inner_size(winit::dpi::LogicalSize::new(config.width, config.height));

    let gpu = pollster::block_on(GpuRenderer::new(
        Arc::clone(&window),
        config.asset_root.clone(),
    ))?;
    let mut engine = Engine::with_config(EngineConfig {
        seed: config.seed,
        tick_hz: config.tick_hz,
    });
    let scene = Scene::load(scene_path, &engine.nodes)?;
    engine.load_scene(scene);
    engine.set_renderer(Box::new(gpu));
    engine.enable_rendering(true);

    let tick_duration = Duration::from_secs_f64(1.0 / config.tick_hz as f64);
    let last_tick = std::sync::atomic::AtomicU64::new(Instant::now().elapsed().as_nanos() as u64);
    let window_ref = Arc::clone(&window);
    let key_state = std::sync::Arc::new(std::sync::Mutex::new((0i8, 0i8, false)));
    let key_state_ref = key_state.clone();

    event_loop
        .run(move |event, target| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                engine.renderer_mut().shutdown();
                target.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    event: key_event, ..
                },
                ..
            } => {
                use winit::keyboard::{KeyCode, PhysicalKey};
                let pressed = key_event.state.is_pressed();
                let mut ks = key_state.lock().unwrap();
                if let PhysicalKey::Code(code) = key_event.physical_key {
                    match code {
                        KeyCode::KeyW | KeyCode::ArrowUp => ks.1 = if pressed { 1 } else { 0 },
                        KeyCode::KeyS | KeyCode::ArrowDown => ks.1 = if pressed { -1 } else { 0 },
                        KeyCode::KeyA | KeyCode::ArrowLeft => ks.0 = if pressed { -1 } else { 0 },
                        KeyCode::KeyD | KeyCode::ArrowRight => ks.0 = if pressed { 1 } else { 0 },
                        KeyCode::Space => ks.2 = pressed,
                        _ => {}
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let (dx, dy, action) = *key_state_ref.lock().unwrap();
                engine.set_input_direction(dx as f64, dy as f64);
                engine.set_input_action(action);
                engine.tick();
                engine.render_now();
            }
            Event::AboutToWait => {
                let now = Instant::now().elapsed().as_nanos() as u64;
                let last = last_tick.load(std::sync::atomic::Ordering::Relaxed);
                if (now - last) >= tick_duration.as_nanos() as u64 {
                    last_tick.store(now, std::sync::atomic::Ordering::Relaxed);
                    window_ref.request_redraw();
                }
            }
            _ => {}
        })
        .map_err(|e| {
            craft_kernel::error::EngineError::Internal(format!("event loop error: {e}"))
        })?;

    #[allow(unreachable_code)]
    Ok(())
}
