use wgpu::util::DeviceExt;

pub const SPRITE_VERTEX_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct InstanceInput {
    @location(2) transform: mat4x4<f32>,
    @location(6) src_rect: vec4<f32>,
    @location(7) modulate: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) modulate: vec4<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    vert: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.view_proj * instance.transform * vec4<f32>(vert.position, 0.0, 1.0);
    out.tex_coord = instance.src_rect.xy + vert.tex_coord * instance.src_rect.zw;
    out.modulate = instance.modulate;
    return out;
}
"#;

pub const SPRITE_FRAGMENT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) modulate: vec4<f32>,
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coord) * in.modulate;
}
"#;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

pub fn create_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    let vertices: [Vertex; 4] = [
        Vertex {
            position: [-0.5, -0.5],
            tex_coord: [0.0, 0.0],
        },
        Vertex {
            position: [0.5, -0.5],
            tex_coord: [1.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.5],
            tex_coord: [1.0, 1.0],
        },
        Vertex {
            position: [-0.5, 0.5],
            tex_coord: [0.0, 1.0],
        },
    ];
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite quad vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub fn create_index_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite quad indices"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    })
}

pub fn create_sprite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sprite shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(format!(
            "{}\n{}",
            SPRITE_VERTEX_SHADER, SPRITE_FRAGMENT_SHADER
        ))),
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sprite pipeline layout"),
        bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sprite pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                },
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<super::InstanceData>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 3,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 32,
                            shader_location: 4,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 48,
                            shader_location: 5,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 64,
                            shader_location: 6,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 80,
                            shader_location: 7,
                        },
                    ],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}
