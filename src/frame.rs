/// A Frame renders a BluePrint onto the App window
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::layout::{PlotOutput, PlotRegion, WindowSegment};
use crate::theme::Theme;
use crate::shape::{Element, HAlign, Rectangle, Text, TextRotation, VAlign};
use crate::transform::ContinuousNumericScale;

use std::collections::HashMap;
use wgpu_text::{
    glyph_brush::Section as TextSection,
    Matrix, TextBrush,
};

use glyph_brush::ab_glyph::FontRef;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// General-pipeline vertex: position + color.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}
impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// GPU vertex for tessellated polyline triangle strips.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub edge_dist: f32,
}
impl LineVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() + std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

/// A single quad vertex for the shared point quad (4 vertices drawn N times).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    /// Offset from center in [-1, 1] space; multiplied by half_size on GPU
    pub offset: [f32; 2],
    /// UV coordinates [0, 1] for SDF circle computation
    pub uv: [f32; 2],
}
impl QuadVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Per-point instance data uploaded to the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointInstance {
    /// NDC center of the point
    pub center: [f32; 2],
    /// NDC half-extents (half_width, half_height)
    pub half_size: [f32; 2],
    /// RGBA color
    pub color: [f32; 4],
}
impl PointInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PointInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

const RECT_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

fn rectangle_vertices(rect: &Rectangle, segment: &WindowSegment) -> Vec<Vertex> {
    let abs_position = [
        segment.abs_x(&rect.position[0]),
        segment.abs_y(&rect.position[1]),
    ];
    let abs_width = segment.abs_width(&rect.width);
    let abs_height = segment.abs_height(&rect.height);
    vec![
        Vertex { position: [abs_position[0] - abs_width / 2., abs_position[1] + abs_height / 2., 0.0], color: rect.color },
        Vertex { position: [abs_position[0] - abs_width / 2., abs_position[1] - abs_height / 2., 0.0], color: rect.color },
        Vertex { position: [abs_position[0] + abs_width / 2., abs_position[1] - abs_height / 2., 0.0], color: rect.color },
        Vertex { position: [abs_position[0] + abs_width / 2., abs_position[1] + abs_height / 2., 0.0], color: rect.color },
    ]
}

fn text_to_section<'a>(
    text: &'a Text,
    segment: &WindowSegment,
    window_width: f32,
    window_height: f32,
) -> wgpu_text::glyph_brush::Section<'a> {
    let h_align = match text.h_align {
        HAlign::Left => wgpu_text::glyph_brush::HorizontalAlign::Left,
        HAlign::Center => wgpu_text::glyph_brush::HorizontalAlign::Center,
        HAlign::Right => wgpu_text::glyph_brush::HorizontalAlign::Right,
    };
    let v_align = match text.v_align {
        VAlign::Top => wgpu_text::glyph_brush::VerticalAlign::Top,
        VAlign::Center => wgpu_text::glyph_brush::VerticalAlign::Center,
    };

    // Convert text position to pixels
    let x_ndc = segment.abs_x(&text.position.0);
    let x = segment.ndc_scale_x.map_position(&segment.pixel_scale_x, x_ndc.into());
    let y_ndc = segment.abs_y(&text.position.1);
    let flipped_pixel_y = ContinuousNumericScale {
        min: segment.pixel_scale_y.max,
        max: segment.pixel_scale_y.min,
    };
    let y = segment.ndc_scale_y.map_position(&flipped_pixel_y, y_ndc.into());
    let position = match text.rotation {
        TextRotation::None => (x as f32, y as f32),
        TextRotation::Ccw90 => (window_height - y as f32, x as f32),
        TextRotation::Cw90 => (y as f32, window_width - x as f32),
    };

    let layout = if text.wrap {
        wgpu_text::glyph_brush::Layout::default_wrap()
            .h_align(h_align)
            .v_align(v_align)
    } else {
        wgpu_text::glyph_brush::Layout::default_single_line()
            .h_align(h_align)
            .v_align(v_align)
    };
    let bounds = if text.wrap {
        match text.rotation {
            TextRotation::None => (segment.pixel_scale_x.span() as f32, segment.pixel_scale_y.span() as f32),
            TextRotation::Ccw90 | TextRotation::Cw90 => (segment.pixel_scale_y.span() as f32, segment.pixel_scale_x.span() as f32),
        }
    } else {
        (f32::INFINITY, f32::INFINITY)
    };
    wgpu_text::glyph_brush::Section::default()
        .with_screen_position(position)
        .with_bounds(bounds)
        .with_layout(layout)
        .add_text(wgpu_text::glyph_brush::Text::new(&text.value).with_scale(text.font_size))
}

/// Build an orthographic projection matrix for a given text rotation.
pub fn text_projection_matrix(rotation: TextRotation, width: f32, height: f32) -> Matrix {
    match rotation {
        TextRotation::None => wgpu_text::ortho(width, height),
        TextRotation::Ccw90 => [
            [0.0, 2.0 / height, 0.0, 0.0],
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0, 1.0],
        ],
        TextRotation::Cw90 => [
            [0.0, -2.0 / height, 0.0, 0.0],
            [-2.0 / width, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0, 1.0],
        ],
    }
}

/// View transform uniform — identity matrix enables future pan/zoom support.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewUniform {
    transform: [[f32; 4]; 4],
}
impl ViewUniform {
    pub fn identity() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

pub struct Frame {
    view_bind_group: wgpu::BindGroup,

    // General pipeline: rectangles, axes, tick marks
    general_vertex_buffer: wgpu::Buffer,
    general_index_buffer: wgpu::Buffer,
    general_pipeline: wgpu::RenderPipeline,
    general_num_indices: u32,

    // Shared quad geometry for instanced pipelines (points + lines)
    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,

    // Point pipeline: SDF instanced circles
    point_instance_buffer: wgpu::Buffer,
    point_pipeline: wgpu::RenderPipeline,
    num_point_instances: u32,

    // Line pipeline: miter-join tessellated polylines
    line_vertex_buffer: wgpu::Buffer,
    line_index_buffer: wgpu::Buffer,
    line_pipeline: wgpu::RenderPipeline,
    line_num_indices: u32,
}

/// Static quad shared by all instanced pipelines (points and lines):
///   TL(-1, 1) → BL(-1,-1) → BR(1,-1) → TR(1,1)
/// UVs map (0,0) top-left to (1,1) bottom-right.
const QUAD_VERTICES: [QuadVertex; 4] = [
    QuadVertex { offset: [-1.0,  1.0], uv: [0.0, 0.0] }, // TL
    QuadVertex { offset: [-1.0, -1.0], uv: [0.0, 1.0] }, // BL
    QuadVertex { offset: [ 1.0, -1.0], uv: [1.0, 1.0] }, // BR
    QuadVertex { offset: [ 1.0,  1.0], uv: [1.0, 0.0] }, // TR
];
const QUAD_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// Helper: create an instance buffer from a vec, using a dummy element if empty
/// (wgpu requires non-zero buffer size).
fn create_instance_buffer<T: bytemuck::Pod + Default>(
    device: &wgpu::Device,
    label: &str,
    instances: &[T],
) -> wgpu::Buffer {
    let contents = if instances.is_empty() {
        bytemuck::cast_slice(&[T::default()]).to_vec()
    } else {
        bytemuck::cast_slice(instances).to_vec()
    };
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: &contents,
        usage: wgpu::BufferUsages::VERTEX,
    })
}

/// Helper: create a render pipeline with the instanced primitive state
fn create_instanced_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    config: &wgpu::SurfaceConfiguration,
    vs_entry: &str,
    fs_entry: &str,
    instance_desc: wgpu::VertexBufferLayout<'static>,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vs_entry),
            buffers: &[QuadVertex::desc(), instance_desc],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}

/// Tessellate a polyline into a triangle-list mesh with miter joins.
///
/// For each point, two vertices are emitted offset perpendicular to the line
/// direction. At interior points the offset follows the miter (bisector of
/// adjacent segment normals) so the strip covers the joint with no gaps.
///
/// `edge_dist` is the signed pixel distance from the line center: positive on
/// one side, negative on the other. The fragment shader uses this for a
/// fixed-width 1px anti-aliasing feather at the edges.
fn tessellate_polyline(
    points_ndc: &[[f32; 2]],
    colors: &[[f32; 4]],
    half_thickness_px: f32,
    px_per_ndc_x: f32,
    px_per_ndc_y: f32,
    vertices: &mut Vec<LineVertex>,
    indices: &mut Vec<u32>,
) {
    let n = points_ndc.len();
    if n < 2 {
        return;
    }

    // Extend the strip by 1px on each side for the AA feather.
    let total_half_px = half_thickness_px + 1.0;
    // edge_dist is normalized so ±1.0 = line body edge, values beyond 1.0
    // are in the AA feather zone. The strip physically extends to total_half_px
    // but edge_dist at the strip edge = total_half_px / half_thickness_px.
    let edge_dist_at_strip_edge = if half_thickness_px > 0.0 {
        total_half_px / half_thickness_px
    } else {
        1.0
    };

    let base = vertices.len() as u32;

    for i in 0..n {
        let cur_px = [points_ndc[i][0] * px_per_ndc_x, points_ndc[i][1] * px_per_ndc_y];

        // Compute incoming and outgoing directions in pixel space
        let d_in: Option<[f32; 2]> = if i > 0 {
            let prev_px = [points_ndc[i - 1][0] * px_per_ndc_x, points_ndc[i - 1][1] * px_per_ndc_y];
            let dx = cur_px[0] - prev_px[0];
            let dy = cur_px[1] - prev_px[1];
            let len = (dx * dx + dy * dy).sqrt();
            if len > 1e-6 { Some([dx / len, dy / len]) } else { None }
        } else {
            None
        };

        let d_out: Option<[f32; 2]> = if i < n - 1 {
            let next_px = [points_ndc[i + 1][0] * px_per_ndc_x, points_ndc[i + 1][1] * px_per_ndc_y];
            let dx = next_px[0] - cur_px[0];
            let dy = next_px[1] - cur_px[1];
            let len = (dx * dx + dy * dy).sqrt();
            if len > 1e-6 { Some([dx / len, dy / len]) } else { None }
        } else {
            None
        };

        // Use whichever direction(s) are available
        let (din, dout) = match (d_in, d_out) {
            (Some(a), Some(b)) => (a, b),
            (Some(a), None) => (a, a),
            (None, Some(b)) => (b, b),
            (None, None) => {
                // Degenerate: duplicate point, skip
                // Still emit vertices to keep indexing consistent
                vertices.push(LineVertex { position: points_ndc[i], color: colors[i], edge_dist: edge_dist_at_strip_edge });
                vertices.push(LineVertex { position: points_ndc[i], color: colors[i], edge_dist: -edge_dist_at_strip_edge });
                continue;
            }
        };

        // Perpendicular normals (rotated 90° CCW)
        let n_in = [-din[1], din[0]];
        let n_out = [-dout[1], dout[0]];

        // Miter direction = normalized sum of normals
        let mx = n_in[0] + n_out[0];
        let my = n_in[1] + n_out[1];
        let mlen = (mx * mx + my * my).sqrt();
        let (miter_x, miter_y) = if mlen > 1e-6 {
            (mx / mlen, my / mlen)
        } else {
            // Normals cancel out (180° turn) — use one of them
            (n_in[0], n_in[1])
        };

        // Miter length: total_half_px / dot(miter, n_in), clamped to avoid spikes
        let dot = miter_x * n_in[0] + miter_y * n_in[1];
        let miter_len = if dot.abs() > 1e-6 {
            (total_half_px / dot).min(total_half_px * 2.0).max(-total_half_px * 2.0)
        } else {
            total_half_px
        };

        // Convert offset back to NDC
        let offset_ndc = [
            miter_x * miter_len / px_per_ndc_x,
            miter_y * miter_len / px_per_ndc_y,
        ];

        // edge_dist: ±1.0 at body edge, ±edge_dist_at_strip_edge at strip edge
        vertices.push(LineVertex {
            position: [points_ndc[i][0] + offset_ndc[0], points_ndc[i][1] + offset_ndc[1]],
            color: colors[i],
            edge_dist: edge_dist_at_strip_edge,
        });
        vertices.push(LineVertex {
            position: [points_ndc[i][0] - offset_ndc[0], points_ndc[i][1] - offset_ndc[1]],
            color: colors[i],
            edge_dist: -edge_dist_at_strip_edge,
        });
    }

    // Emit triangle-list indices: two triangles per segment
    for i in 0..(n as u32 - 1) {
        let left_i = base + 2 * i;
        let right_i = base + 2 * i + 1;
        let left_j = base + 2 * (i + 1);
        let right_j = base + 2 * (i + 1) + 1;
        indices.extend_from_slice(&[left_i, right_i, left_j, right_i, right_j, left_j]);
    }
}

impl Frame {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        window: std::sync::Arc<Window>,
        queue: &wgpu::Queue,
        brushes: &mut HashMap<TextRotation, TextBrush<FontRef>>,
        plot_output: &PlotOutput,
        theme: &Theme,
        view_uniform: ViewUniform,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // View uniform buffer + bind group
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("View Uniform Buffer"),
            contents: bytemuck::cast_slice(&[view_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("View Bind Group Layout"),
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
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("View Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let window_width = window.inner_size().width;
        let window_height_u32 = window.inner_size().height;
        let root_segment = WindowSegment::new_root(window_width, window_height_u32);
        let margined = root_segment.with_margin(theme.window_margin);
        let mut segments = plot_output.layout.resolve(&margined);

        // For polar plots, square up DataArea segments so circles aren't distorted.
        if plot_output.is_polar {
            for (key, seg) in segments.iter_mut() {
                if key.region == PlotRegion::DataArea {
                    *seg = seg.squared();
                }
            }
        }
        let window_height = window_height_u32 as f32;

        let mut vertices: Vec<Vertex> = vec![];
        let mut indices: Vec<u32> = vec![];
        let mut point_instances: Vec<PointInstance> = vec![];
        let mut line_vertices: Vec<LineVertex> = vec![];
        let mut line_indices: Vec<u32> = vec![];
        let mut text_sections_by_rotation: HashMap<TextRotation, Vec<TextSection>> = HashMap::new();
        let window_width_f32 = window_width as f32;
        let px_per_ndc_x = window_width_f32 / 2.0;
        let px_per_ndc_y = window_height / 2.0;

        for (key, elements) in &plot_output.regions {
            let segment = match segments.get(key) {
                Some(s) => s,
                None => continue,
            };
            for element in elements.iter() {
                match element {
                    Element::Rect(r) => {
                        let base_index = vertices.len() as u32;
                        vertices.extend_from_slice(&rectangle_vertices(r, segment));
                        indices.extend(RECT_INDICES.iter().map(|idx| idx + base_index));
                    }
                    Element::Point(p) => {
                        let cx = segment.abs_x(&p.position[0]);
                        let cy = segment.abs_y(&p.position[1]);
                        let hw = segment.abs_width(&p.size) / 2.0;
                        let hh = segment.abs_height(&p.size) / 2.0;
                        point_instances.push(PointInstance {
                            center: [cx, cy],
                            half_size: [hw, hh],
                            color: p.color,
                        });
                    }
                    Element::Polyline(poly) => {
                        let points_ndc: Vec<[f32; 2]> = poly.points.iter()
                            .map(|p| [segment.abs_x(&p[0]), segment.abs_y(&p[1])])
                            .collect();
                        tessellate_polyline(
                            &points_ndc,
                            &poly.colors,
                            poly.thickness / 2.0,
                            px_per_ndc_x,
                            px_per_ndc_y,
                            &mut line_vertices,
                            &mut line_indices,
                        );
                    }
                    Element::Arc(arc) => {
                        let cx = segment.abs_x(&arc.center[0]);
                        let cy = segment.abs_y(&arc.center[1]);
                        // Use separate X/Y radii so arcs appear circular despite
                        // anisotropic clip space (1 clip unit ≠ same physical distance
                        // in X vs Y when the viewport isn't square).
                        let r_outer_x = segment.abs_width(&arc.outer_radius);
                        let r_outer_y = segment.abs_height(&arc.outer_radius);
                        let r_inner_x = segment.abs_width(&arc.inner_radius);
                        let r_inner_y = segment.abs_height(&arc.inner_radius);
                        // Tessellate arc into triangle fan/strip
                        let steps = ((arc.end_angle - arc.start_angle).abs() / 0.02).ceil() as usize;
                        let steps = steps.max(4);
                        let base = vertices.len() as u32;
                        if r_inner_x <= 0.001 {
                            // Pie wedge: triangle fan from center
                            vertices.push(Vertex { position: [cx, cy, 0.0], color: arc.color });
                            for i in 0..=steps {
                                let t = i as f32 / steps as f32;
                                let angle = arc.start_angle + t * (arc.end_angle - arc.start_angle);
                                let x = cx + r_outer_x * angle.cos();
                                let y = cy + r_outer_y * angle.sin();
                                vertices.push(Vertex { position: [x, y, 0.0], color: arc.color });
                            }
                            for i in 0..steps as u32 {
                                indices.push(base);           // center
                                indices.push(base + 1 + i);   // current edge
                                indices.push(base + 2 + i);   // next edge
                            }
                        } else {
                            // Annular wedge: quad strip between inner and outer
                            for i in 0..=steps {
                                let t = i as f32 / steps as f32;
                                let angle = arc.start_angle + t * (arc.end_angle - arc.start_angle);
                                let cos_a = angle.cos();
                                let sin_a = angle.sin();
                                vertices.push(Vertex {
                                    position: [cx + r_inner_x * cos_a, cy + r_inner_y * sin_a, 0.0],
                                    color: arc.color,
                                });
                                vertices.push(Vertex {
                                    position: [cx + r_outer_x * cos_a, cy + r_outer_y * sin_a, 0.0],
                                    color: arc.color,
                                });
                            }
                            for i in 0..steps as u32 {
                                let j = base + i * 2;
                                indices.extend_from_slice(&[j, j + 1, j + 3, j, j + 3, j + 2]);
                            }
                        }
                    }
                    Element::Text(t) => {
                        text_sections_by_rotation
                            .entry(t.rotation)
                            .or_default()
                            .push(text_to_section(t, segment, window_width_f32, window_height));
                    }
                }
            }
        }

        for (rotation, sections) in text_sections_by_rotation {
            if let Some(brush) = brushes.get_mut(&rotation) {
                brush.queue(device, queue, sections).unwrap();
            }
        }

        // General pipeline (rectangles, axes, tick marks)
        let general_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("General Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        // Instanced pipelines (points and lines share quad geometry)
        let point_pipeline = create_instanced_pipeline(
            device, &pipeline_layout, &shader, config,
            "vs_point_instanced", "fs_point",
            PointInstance::desc(), "Point Pipeline",
        );
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_line"),
                buffers: &[LineVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_line"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        // General geometry buffers
        let general_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("General Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let general_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("General Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Shared quad buffers (static, used by both point and line pipelines)
        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Index Buffer"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Per-element instance buffers
        let num_point_instances = point_instances.len() as u32;
        let point_instance_buffer =
            create_instance_buffer(device, "Point Instance Buffer", &point_instances);

        let line_num_indices = line_indices.len() as u32;
        let line_vertex_contents = if line_vertices.is_empty() {
            bytemuck::cast_slice(&[LineVertex { position: [0.0; 2], color: [0.0; 4], edge_dist: 0.0 }]).to_vec()
        } else {
            bytemuck::cast_slice(&line_vertices).to_vec()
        };
        let line_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Vertex Buffer"),
            contents: &line_vertex_contents,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let line_index_contents = if line_indices.is_empty() {
            bytemuck::cast_slice(&[0u32]).to_vec()
        } else {
            bytemuck::cast_slice(&line_indices).to_vec()
        };
        let line_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Index Buffer"),
            contents: &line_index_contents,
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            view_bind_group,
            general_vertex_buffer,
            general_index_buffer,
            general_pipeline,
            general_num_indices: indices.len() as u32,
            quad_vertex_buffer,
            quad_index_buffer,
            point_instance_buffer,
            point_pipeline,
            num_point_instances,
            line_vertex_buffer,
            line_index_buffer,
            line_pipeline,
            line_num_indices,
        }
    }

    pub fn render<'b>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'b>,
    ) {
        render_pass.set_bind_group(0, &self.view_bind_group, &[]);

        // 1. General pipeline (axes, tick marks, legend swatches)
        if self.general_vertex_buffer.size() > 0 && self.general_num_indices > 0 {
            render_pass.set_pipeline(&self.general_pipeline);
            render_pass.set_vertex_buffer(0, self.general_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.general_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.general_num_indices, 0, 0..1);
        }

        // 2. Line pipeline (miter-join tessellated polylines — drawn before points)
        if self.line_num_indices > 0 {
            render_pass.set_pipeline(&self.line_pipeline);
            render_pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.line_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.line_num_indices, 0, 0..1);
        }

        // 3. Point pipeline (SDF instanced circles — drawn on top)
        if self.num_point_instances > 0 {
            render_pass.set_pipeline(&self.point_pipeline);
            render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.point_instance_buffer.slice(..));
            render_pass.set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..6, 0, 0..self.num_point_instances);
        }
    }
}
