/// A Frame renders a BluePrint onto the App window
use std::vec;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{plot, shape};

use crate::shape::{Element, Vertex};

use wgpu_text::{
    glyph_brush::{Section as TextSection, Text},
    BrushBuilder, TextBrush,
};

use glyph_brush::ab_glyph::{Font, FontArc, FontRef, InvalidFont, Rect};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        // <hack msg="Demo purposes only, this will eventually be passed in">
        let layer = plot::Layer::new(
            Box::new(plot::GeomPoint {}),
            vec![plot::Mapping::X("x".into()), plot::Mapping::Y("y".into())],
            Box::new(plot::IdentityTransform {}),
            Box::new(plot::IdentityTransform {}),
        );
        let theme = plot::Theme::default();
        let mut bp = plot::Blueprint::new(&theme)
            .with_layer(layer)
            .with_scale(Box::new(plot::ScaleXContinuous::new()))
            .with_scale(Box::new(plot::ScaleYContinuous::new()));

        let mut plot_data = plot::PlotData::new();
        plot_data.insert(
            "x".into(),
            plot::PlotParameter::FloatArray(vec![0.0, 0.5, 1.0]),
        );
        plot_data.insert(
            "y".into(),
            plot::PlotParameter::FloatArray(vec![2.0, 0.0, 2.0]),
        );
        // </hack>

        let mut window_segment = shape::WindowSegment::new_root(window.clone());
        window_segment = window_segment.with_margin(theme.window_margin);

        let mut vertices = vec![];
        let mut indices = vec![];
        let mut text = vec![];
        for element in bp.render(plot_data).expect("Could render plot").iter() {
            match element {
                Element::Shape(s) => {
                    let base_index = vertices.len();
                    let shape_vertices = s.vertices(&window_segment);

                    vertices.extend_from_slice(&shape_vertices);
                    indices.extend(s.indices().iter().map(|idx| idx + base_index as u16));
                }
                Element::Text(t) => {
                    // Buffer text elements for bulk queuing below
                    text.push(t.clone());
                }
            }
        }
        let text_sections: Vec<TextSection> = text.iter().map(|t| t.as_section()).collect();
        brush.queue(device, queue, text_sections).unwrap();

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

        // Create some text for testing
        let section = TextSection::default().add_text(Text::new("Hello World").with_scale(72.0));
        brush.queue(device, queue, [&section]).unwrap();

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
        brush: &TextBrush<FontRef>,
    ) {
        if self.vertex_buffer.size() == 0 {
            return;
        }
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        brush.draw(render_pass);
    }
}
