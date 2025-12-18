/// The window module owns the window, rendering loop, and base surface that all
/// other graphical elements are arranged on.
use std::vec;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{plot, shape};

use crate::shape::Vertex;
use crate::transform;

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
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
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
        println!("{:?}", window_segment);
        window_segment = window_segment.with_margin(theme.window_margin);
        println!("{:?}", window_segment);

        let mut vertices = vec![];
        let mut indices = vec![];
        for shape in bp.render(plot_data).expect("Could render plot").iter() {
            let base_index = vertices.len();
            let shape_vertices = shape.vertices(&window_segment);

            vertices.extend_from_slice(&shape_vertices);
            indices.extend(shape.indices().iter().map(|idx| idx + base_index as u16));
        }

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
            multiview: None,
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

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.vertex_buffer.size() == 0 {
            return;
        }
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}
