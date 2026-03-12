/// A Frame renders a BluePrint onto the App window
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::layout::{PlotOutput, WindowSegment};
use crate::theme::Theme;
use crate::shape::{Element, Vertex};

use wgpu_text::{
    glyph_brush::Section as TextSection,
    Matrix, TextBrush,
};

use glyph_brush::ab_glyph::FontRef;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Orthographic projection matrix with 90° CCW rotation.
/// In this coordinate system, (rx, ry) maps to screen position where:
/// rx controls vertical position (0=bottom, h=top) and ry controls horizontal (0=left, w=right).
pub fn ortho_rotated_ccw(width: f32, height: f32) -> Matrix {
    [
        [0.0, 2.0 / height, 0.0, 0.0],
        [2.0 / width, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-1.0, -1.0, 0.0, 1.0],
    ]
}

pub struct Frame {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
}
impl Frame {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        window: std::sync::Arc<Window>,
        queue: &wgpu::Queue,
        brush: &mut TextBrush<FontRef>,
        brush_rotated: &mut TextBrush<FontRef>,
        plot_output: &PlotOutput,
        theme: &Theme,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let root_segment = WindowSegment::new_root(window.clone());
        let margined = root_segment.with_margin(theme.window_margin);
        let segments = plot_output.layout.resolve(&margined);
        let window_height = window.inner_size().height as f32;

        let mut vertices = vec![];
        let mut indices: Vec<u16> = vec![];
        let mut text_sections: Vec<TextSection> = vec![];
        let mut rotated_text_sections: Vec<TextSection> = vec![];

        for (region, elements) in &plot_output.regions {
            let segment = match segments.get(region) {
                Some(s) => s,
                None => continue,
            };
            for element in elements.iter() {
                match element {
                    Element::Shape(s) => {
                        let base_index = vertices.len();
                        let shape_vertices = s.vertices(segment);
                        if shape_vertices.is_empty() {
                            continue;
                        }
                        vertices.extend_from_slice(&shape_vertices);
                        indices.extend(s.indices().iter().map(|idx| idx + base_index as u16));
                    }
                    Element::Text(t) => {
                        if t.rotated {
                            rotated_text_sections.push(t.as_section(segment, window_height));
                        } else {
                            text_sections.push(t.as_section(segment, window_height));
                        }
                    }
                }
            }
        }

        brush.queue(device, queue, text_sections).unwrap();
        brush_rotated
            .queue(device, queue, rotated_text_sections)
            .unwrap();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
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
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            render_pipeline,
            num_indices: indices.len() as u32,
        }
    }

    pub fn render<'b>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'b>,
    ) {
        if self.vertex_buffer.size() == 0 {
            return;
        }
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}
