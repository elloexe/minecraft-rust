//! wgpu renderer: opaque terrain, translucent water, selection outline and HUD.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{DVec3, IVec3, Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::mesher::{MeshData, Vertex};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    cam_base: [i32; 4],
    cam_frac: [f32; 4],
    fog: [f32; 4],
    sky: [f32; 4],
    atlas: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HudVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

struct GpuMesh {
    origin: wgpu::Buffer,
    opaque: Option<(wgpu::Buffer, u32)>,
    water: Option<(wgpu::Buffer, u32)>,
    origin_world: IVec3,
}

pub struct Camera {
    pub eye: DVec3,
    pub yaw: f32,   // radians; 0 = facing +Z, increases turning toward -X
    pub pitch: f32, // radians; positive looks down
    pub fov_y: f32,
}

impl Camera {
    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, -sp, cy * cp)
    }
}

pub struct FrameParams<'a> {
    pub sky: [f32; 3],
    pub fog_end: f32,
    pub daylight: f32,
    pub outline: Option<IVec3>,
    pub hud: &'a [HudVertex],
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
    globals: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// Bind group used on the title screen, with the game logo bound in place of the atlas.
    menu_bind: Option<wgpu::BindGroup>,
    logo_aspect: Option<f32>,
    terrain: wgpu::RenderPipeline,
    water: wgpu::RenderPipeline,
    lines: wgpu::RenderPipeline,
    hud: wgpu::RenderPipeline,
    quad_index: wgpu::Buffer,
    quad_capacity: u32,
    line_buf: wgpu::Buffer,
    hud_buf: wgpu::Buffer,
    hud_capacity: usize,
    meshes: HashMap<(i32, i32), GpuMesh>,
    tiles_per_row: u32,
    atlas_size: (u32, u32),
    pub adapter_name: String,
    pub drawn_last_frame: usize,
}

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn quad_indices(quads: u32) -> Vec<u32> {
    let mut v = Vec::with_capacity(quads as usize * 6);
    for q in 0..quads {
        let b = q * 4;
        v.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    v
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        atlas: &[u8],
        atlas_w: u32,
        atlas_h: u32,
        vsync: bool,
        logo: Option<&crate::assets::Image>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;
        let adapter_name = adapter.get_info().name;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("claudium"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync },
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let atlas_tex = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("atlas"),
                size: wgpu::Extent3d { width: atlas_w, height: atlas_h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            atlas,
        );
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        // Optional title-screen logo texture, loaded from the player's own game files.
        let (menu_bind, logo_aspect) = if let Some(img) = logo {
            let tex = device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some("logo"),
                    size: wgpu::Extent3d { width: img.w, height: img.h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &img.rgba,
            );
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("menu"),
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            });
            (Some(bg), Some(img.w as f32 / img.h.max(1) as f32))
        } else {
            (None, None)
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("claudium"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let terrain_buffers = [
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Uint32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![1 => Sint32x4],
            },
        ];
        let alpha_blend = Some(wgpu::BlendState::ALPHA_BLENDING);
        let make = |label: &str,
                    vs: &str,
                    fs: &str,
                    buffers: &[wgpu::VertexBufferLayout],
                    topology: wgpu::PrimitiveTopology,
                    cull: Option<wgpu::Face>,
                    depth: Option<(bool, wgpu::CompareFunction)>,
                    blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some(vs), compilation_options: Default::default(), buffers },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState { topology, cull_mode: cull, ..Default::default() },
                depth_stencil: depth.map(|(write, cmp)| wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: write,
                    depth_compare: cmp,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let tri = wgpu::PrimitiveTopology::TriangleList;
        let terrain = make(
            "terrain",
            "vs_main",
            "fs_main",
            &terrain_buffers,
            tri,
            Some(wgpu::Face::Back),
            Some((true, wgpu::CompareFunction::Less)),
            None,
        );
        let water =
            make("water", "vs_main", "fs_water", &terrain_buffers, tri, None, Some((false, wgpu::CompareFunction::Less)), alpha_blend);
        let line_layout = [wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3],
        }];
        let lines = make(
            "outline",
            "vs_line",
            "fs_line",
            &line_layout,
            wgpu::PrimitiveTopology::LineList,
            None,
            Some((false, wgpu::CompareFunction::LessEqual)),
            alpha_blend,
        );
        let hud_layout = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<HudVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
        }];
        let hud = make("hud", "vs_hud", "fs_hud", &hud_layout, tri, None, None, alpha_blend);

        let quad_capacity = 65536;
        let quad_index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad indices"),
            contents: bytemuck::cast_slice(&quad_indices(quad_capacity)),
            usage: wgpu::BufferUsages::INDEX,
        });
        let line_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("outline"),
            size: 24 * 12,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hud_capacity = 65536;
        let hud_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud"),
            size: (hud_capacity * std::mem::size_of::<HudVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let depth = depth_view(&device, config.width, config.height);
        Ok(Renderer {
            surface,
            device,
            queue,
            config,
            depth,
            globals,
            bind_group,
            menu_bind,
            logo_aspect,
            terrain,
            water,
            lines,
            hud,
            quad_index,
            quad_capacity,
            line_buf,
            hud_buf,
            hud_capacity,
            meshes: HashMap::new(),
            tiles_per_row: atlas_w / crate::assets::TILE as u32,
            atlas_size: (atlas_w, atlas_h),
            adapter_name,
            drawn_last_frame: 0,
        })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth = depth_view(&self.device, w, h);
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    pub fn size(&self) -> (f32, f32) {
        (self.config.width.max(1) as f32, self.config.height.max(1) as f32)
    }

    /// Aspect ratio (w/h) of the game logo, if one was loaded from the player's files.
    pub fn logo_aspect(&self) -> Option<f32> {
        self.logo_aspect
    }

    /// Draws a HUD-only screen (the title menu): clears to `clear`, then draws the
    /// given 2D triangles. No world and no depth, so it never touches game rendering.
    pub fn render_menu(&mut self, hud: &[HudVertex], clear: [f32; 3]) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                eprintln!("surface error: {e}");
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let n = hud.len().min(self.hud_capacity);
        if n > 0 {
            self.queue.write_buffer(&self.hud_buf, 0, bytemuck::cast_slice(&hud[..n]));
        }
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("menu") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("menu"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: clear[0] as f64, g: clear[1] as f64, b: clear[2] as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if n > 0 {
                // Bind the logo texture (if we loaded one) so logo quads sample it; solid
                // quads ignore the bound texture (the shader returns vertex colour for uv < 0).
                pass.set_bind_group(0, self.menu_bind.as_ref().unwrap_or(&self.bind_group), &[]);
                pass.set_pipeline(&self.hud);
                pass.set_vertex_buffer(0, self.hud_buf.slice(..));
                pass.draw(0..n as u32, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
        frame.present();
    }

    /// UV rectangle of an atlas tile (for the HUD).
    pub fn tile_uv(&self, tile: u16) -> [f32; 4] {
        let tw = crate::assets::TILE as f32 / self.atlas_size.0 as f32;
        let th = crate::assets::TILE as f32 / self.atlas_size.1 as f32;
        let x = (tile as u32 % self.tiles_per_row) as f32 * tw;
        let y = (tile as u32 / self.tiles_per_row) as f32 * th;
        [x, y, x + tw, y + th]
    }

    fn ensure_quads(&mut self, quads: u32) {
        if quads <= self.quad_capacity {
            return;
        }
        let cap = quads.next_power_of_two();
        self.quad_index = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad indices"),
            contents: bytemuck::cast_slice(&quad_indices(cap)),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.quad_capacity = cap;
    }

    pub fn upload_mesh(&mut self, key: (i32, i32), min_y: i32, data: &MeshData) {
        let origin_world = IVec3::new(key.0 * 16, min_y, key.1 * 16);
        let mk = |verts: &[Vertex], label: &str| -> Option<(wgpu::Buffer, u32)> {
            if verts.is_empty() {
                return None;
            }
            let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            Some((buf, verts.len() as u32 / 4))
        };
        let opaque = mk(&data.opaque, "terrain");
        let water = mk(&data.water, "water");
        let max_quads = opaque.as_ref().map(|o| o.1).unwrap_or(0).max(water.as_ref().map(|w| w.1).unwrap_or(0));
        let origin = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("origin"),
            contents: bytemuck::cast_slice(&[origin_world.x, origin_world.y, origin_world.z, 0]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.ensure_quads(max_quads);
        self.meshes.insert(key, GpuMesh { origin, opaque, water, origin_world });
    }

    pub fn remove_mesh(&mut self, key: (i32, i32)) {
        self.meshes.remove(&key);
    }

    pub fn clear_meshes(&mut self) {
        self.meshes.clear();
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn render(&mut self, cam: &Camera, height: i32, p: &FrameParams) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                eprintln!("surface error: {e}");
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Camera-relative rendering keeps precision far from the origin.
        let base = cam.eye.floor();
        let base_i = IVec3::new(base.x as i32, base.y as i32, base.z as i32);
        let frac = (cam.eye - base).as_vec3();
        let fwd = cam.forward();
        let view_m = Mat4::look_to_rh(frac, fwd, Vec3::Y);
        let proj = Mat4::perspective_rh(cam.fov_y, self.aspect(), 0.05, p.fog_end * 1.5 + 64.0);
        let vp = proj * view_m;
        let g = Globals {
            view_proj: vp.to_cols_array_2d(),
            cam_base: [base_i.x, base_i.y, base_i.z, 0],
            cam_frac: [frac.x, frac.y, frac.z, p.daylight],
            fog: [p.fog_end * 0.6, p.fog_end, 0.0, 0.0],
            sky: [p.sky[0], p.sky[1], p.sky[2], 1.0],
            atlas: [
                self.tiles_per_row as f32,
                crate::assets::TILE as f32 / self.atlas_size.0 as f32,
                crate::assets::TILE as f32 / self.atlas_size.1 as f32,
                0.0,
            ],
        };
        self.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&g));

        // Frustum planes (wgpu depth range 0..1).
        let r = [vp.row(0), vp.row(1), vp.row(2), vp.row(3)];
        let planes: [Vec4; 6] = [r[3] + r[0], r[3] - r[0], r[3] + r[1], r[3] - r[1], r[2], r[3] - r[2]];
        let visible = |m: &GpuMesh| -> bool {
            let lo = (m.origin_world - base_i).as_vec3();
            let hi = lo + Vec3::new(16.0, height as f32, 16.0);
            planes.iter().all(|pl| {
                let px = if pl.x >= 0.0 { hi.x } else { lo.x };
                let py = if pl.y >= 0.0 { hi.y } else { lo.y };
                let pz = if pl.z >= 0.0 { hi.z } else { lo.z };
                pl.x * px + pl.y * py + pl.z * pz + pl.w >= 0.0
            })
        };
        let far2 = (p.fog_end + 24.0) * (p.fog_end + 24.0);
        let mut drawn: Vec<(&GpuMesh, f32)> = self
            .meshes
            .values()
            .filter_map(|m| {
                let c = (m.origin_world - base_i).as_vec3() + Vec3::new(8.0, 0.0, 8.0);
                let d2 = c.x * c.x + c.z * c.z;
                (d2 <= far2 && visible(m)).then_some((m, d2))
            })
            .collect();
        drawn.sort_by(|a, b| a.1.total_cmp(&b.1));
        self.drawn_last_frame = drawn.len();

        // Outline vertices
        let mut line_count = 0;
        if let Some(b) = p.outline {
            let o = (b - base_i).as_vec3() - Vec3::splat(0.002);
            let s = 1.004;
            let c = |x: f32, y: f32, z: f32| [o.x + x * s, o.y + y * s, o.z + z * s];
            let corners =
                [c(0., 0., 0.), c(1., 0., 0.), c(1., 0., 1.), c(0., 0., 1.), c(0., 1., 0.), c(1., 1., 0.), c(1., 1., 1.), c(0., 1., 1.)];
            let edges = [(0, 1), (1, 2), (2, 3), (3, 0), (4, 5), (5, 6), (6, 7), (7, 4), (0, 4), (1, 5), (2, 6), (3, 7)];
            let mut v: Vec<[f32; 3]> = Vec::with_capacity(24);
            for (a, b) in edges {
                v.push(corners[a]);
                v.push(corners[b]);
            }
            self.queue.write_buffer(&self.line_buf, 0, bytemuck::cast_slice(&v));
            line_count = v.len() as u32;
        }

        let hud_n = p.hud.len().min(self.hud_capacity);
        if hud_n > 0 {
            self.queue.write_buffer(&self.hud_buf, 0, bytemuck::cast_slice(&p.hud[..hud_n]));
        }

        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: p.sky[0] as f64, g: p.sky[1] as f64, b: p.sky[2] as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_index_buffer(self.quad_index.slice(..), wgpu::IndexFormat::Uint32);

            // Opaque terrain, near to far so early-z rejects hidden fragments.
            pass.set_pipeline(&self.terrain);
            for (m, _) in &drawn {
                if let Some((buf, quads)) = &m.opaque {
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.set_vertex_buffer(1, m.origin.slice(..));
                    pass.draw_indexed(0..quads * 6, 0, 0..1);
                }
            }
            // Water, far to near.
            pass.set_pipeline(&self.water);
            for (m, _) in drawn.iter().rev() {
                if let Some((buf, quads)) = &m.water {
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.set_vertex_buffer(1, m.origin.slice(..));
                    pass.draw_indexed(0..quads * 6, 0, 0..1);
                }
            }
            if line_count > 0 {
                pass.set_pipeline(&self.lines);
                pass.set_vertex_buffer(0, self.line_buf.slice(..));
                pass.draw(0..line_count, 0..1);
            }
            if hud_n > 0 {
                pass.set_pipeline(&self.hud);
                pass.set_vertex_buffer(0, self.hud_buf.slice(..));
                pass.draw(0..hud_n as u32, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
        frame.present();
    }
}
