#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::layout::{Unit, WindowSegment};
use crate::transform::ContinuousNumericScale;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// The position of the vertex (x,y,z)
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

/// The geometry of a shape is defined in relative units and then projected
/// into a specific segment of the window at render time. This supports defining
/// compositions of shapes that can be scaled and positioned on the window,
/// decoupling the rendering of the shape from the layout of the window.
pub trait Shape {
    /// Construct the vertices of the shape, to be sent to GPU shader.
    ///
    /// This method takes a WindowSegment that describes the portion of the
    /// screen that the shape is defined relative to.
    fn vertices(&self, segment: &WindowSegment) -> Vec<Vertex>;

    /// Indices of vertex triplets for drawing triangles
    fn indices(&self) -> &[u32];
}

pub struct Rectangle {
    /// X,Y position
    position: [Unit; 2],
    /// Dimensions of the rectangle either in relative NDC units or in Pixels
    width: Unit,
    height: Unit,
    /// Rectangle fill color (RGBA)
    color: [f32; 4],
}
impl Rectangle {
    /// Create a new rectangle
    ///
    /// Args:
    /// - position: X,Y position of center of the rectangle
    /// - width: The width of the rectange
    /// - height: The height of the rectange
    /// - color: The fill color of the rectangles (RGBA)
    pub fn new(position: [Unit; 2], width: Unit, height: Unit, color: [f32; 4]) -> Self {
        Self {
            position,
            width,
            height,
            color,
        }
    }
}
impl Shape for Rectangle {
    /// Project shape onto a specific portion of the screen
    fn vertices(&self, segment: &WindowSegment) -> Vec<Vertex> {
        // Convert position from relative to absolute units
        let abs_position = [
            segment.abs_x(&self.position[0]),
            segment.abs_y(&self.position[1]),
        ];
        let abs_width = segment.abs_width(&self.width);
        let abs_height = segment.abs_height(&self.height);

        let vertices = vec![
            // Top left
            Vertex {
                position: [
                    abs_position[0] - abs_width / 2.,
                    abs_position[1] + abs_height / 2.,
                    0.0,
                ],
                color: self.color,
            },
            // Bottom left
            Vertex {
                position: [
                    abs_position[0] - abs_width / 2.,
                    abs_position[1] - abs_height / 2.,
                    0.0,
                ],
                color: self.color,
            },
            // Bottom right
            Vertex {
                position: [
                    abs_position[0] + abs_width / 2.,
                    abs_position[1] - abs_height / 2.,
                    0.0,
                ],
                color: self.color,
            },
            // Top right
            Vertex {
                position: [
                    abs_position[0] + abs_width / 2.,
                    abs_position[1] + abs_height / 2.,
                    0.0,
                ],
                color: self.color,
            },
        ];
        vertices
    }

    fn indices(&self) -> &[u32] {
        &[0, 1, 2, 0, 2, 3]
    }
}

#[derive(Clone, Debug, Default)]
pub enum HAlign {
    #[default]
    Left,
    Center,
}

#[derive(Clone, Debug, Default)]
pub enum VAlign {
    #[default]
    Top,
    Center,
}

#[derive(Clone, Debug)]
pub struct Text {
    pub value: String,
    pub font_size: f32,
    pub position: (Unit, Unit),
    pub h_align: HAlign,
    pub v_align: VAlign,
    pub rotated: bool,
    pub wrap: bool,
}
impl Text {
    pub fn new(value: String, font_size: f32, position: (Unit, Unit)) -> Self {
        Self {
            value,
            font_size,
            position,
            h_align: HAlign::Left,
            v_align: VAlign::Top,
            rotated: false,
            wrap: false,
        }
    }

    pub fn centered(value: String, font_size: f32, position: (Unit, Unit)) -> Self {
        Self {
            value,
            font_size,
            position,
            h_align: HAlign::Center,
            v_align: VAlign::Top,
            rotated: false,
            wrap: false,
        }
    }

    pub fn with_wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn with_v_align(mut self, v_align: VAlign) -> Self {
        self.v_align = v_align;
        self
    }

    pub fn with_rotation(mut self) -> Self {
        self.rotated = true;
        self
    }

    /// Return the text as a wgpu_text::glyph_brush::Section.
    /// For rotated text, `window_height` is needed to transform coordinates
    /// into the rotated brush's coordinate system.
    pub fn as_section<'a>(
        &'a self,
        window_segment: &WindowSegment,
        window_height: f32,
    ) -> wgpu_text::glyph_brush::Section<'a> {
        let h_align = match self.h_align {
            HAlign::Left => wgpu_text::glyph_brush::HorizontalAlign::Left,
            HAlign::Center => wgpu_text::glyph_brush::HorizontalAlign::Center,
        };
        let v_align = match self.v_align {
            VAlign::Top => wgpu_text::glyph_brush::VerticalAlign::Top,
            VAlign::Center => wgpu_text::glyph_brush::VerticalAlign::Center,
        };
        let position = if self.rotated {
            let (sx, sy) = self.position_as_pixels(window_segment);
            (window_height - sy, sx)
        } else {
            self.position_as_pixels(window_segment)
        };
        let layout = if self.wrap {
            wgpu_text::glyph_brush::Layout::default_wrap()
                .h_align(h_align)
                .v_align(v_align)
        } else {
            wgpu_text::glyph_brush::Layout::default_single_line()
                .h_align(h_align)
                .v_align(v_align)
        };
        let bounds = if self.wrap {
            if self.rotated {
                (
                    window_segment.pixel_scale_y.span() as f32,
                    window_segment.pixel_scale_x.span() as f32,
                )
            } else {
                (
                    window_segment.pixel_scale_x.span() as f32,
                    window_segment.pixel_scale_y.span() as f32,
                )
            }
        } else {
            (f32::INFINITY, f32::INFINITY)
        };
        wgpu_text::glyph_brush::Section::default()
            .with_screen_position(position)
            .with_bounds(bounds)
            .with_layout(layout)
            .add_text(wgpu_text::glyph_brush::Text::new(&self.value).with_scale(self.font_size))
    }

    fn position_as_pixels(&self, window_segment: &WindowSegment) -> (f32, f32) {
        // x position in ndc coords
        let x_ndc = window_segment.abs_x(&self.position.0);

        // convert to px
        let x = window_segment
            .ndc_scale_x
            .map_position(&window_segment.pixel_scale_x, x_ndc.into());

        // y position in ndc coords
        let y_ndc = window_segment.abs_y(&self.position.1);

        // convert to px (flip y: NDC y+ is up, but screen y+ is down)
        let flipped_pixel_y = ContinuousNumericScale {
            min: window_segment.pixel_scale_y.max,
            max: window_segment.pixel_scale_y.min,
        };
        let y = window_segment
            .ndc_scale_y
            .map_position(&flipped_pixel_y, y_ndc.into());

        (x as f32, y as f32)
    }

}

/// Domain-level polyline data — one per group/series, tessellated in Frame.
pub struct PolylineData {
    pub points: Vec<[Unit; 2]>,
    pub thickness: f32, // pixels
    pub colors: Vec<[f32; 4]>, // per-point RGBA
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

/// Domain-level point data — position, size, and color in layout-relative units.
/// Converted to GPU instances in Frame.
pub struct PointData {
    pub position: [Unit; 2],
    pub size: Unit,
    pub color: [f32; 4],
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
                    // After center ([f32; 2]) + half_size ([f32; 2])
                    offset: (std::mem::size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// An element can be a Shape, Point, Polyline, or Text
pub enum Element {
    Shape(Box<dyn Shape>),
    Point(PointData),
    Polyline(PolylineData),
    Text(Text),
}
