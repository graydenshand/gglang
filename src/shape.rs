#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::layout::{Unit, WindowSegment};
use crate::transform::ContinuousNumericScale;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// The position of the vertex (x,y,z)
    position: [f32; 3],
    color: [f32; 3],
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
                    format: wgpu::VertexFormat::Float32x3,
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
    fn indices(&self) -> &[u16];
}

pub struct Rectangle {
    /// X,Y position
    position: [Unit; 2],
    /// Dimensions of the rectangle either in relative NDC units or in Pixels
    width: Unit,
    height: Unit,
    /// Rectangle fill color
    color: [f32; 3],
}
impl Rectangle {
    /// Create a new rectangle
    ///
    /// Args:
    /// - position: X,Y position of center of the rectangle
    /// - width: The width of the rectange
    /// - height: The height of the rectange
    /// - color: The fill color of the rectangles
    pub fn new(position: [Unit; 2], width: Unit, height: Unit, color: [f32; 3]) -> Self {
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

    fn indices(&self) -> &[u16] {
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

pub struct LineSegment {
    start: [Unit; 2],
    end: [Unit; 2],
    thickness: f32,
    color: [f32; 3],
}
impl LineSegment {
    pub fn new(start: [Unit; 2], end: [Unit; 2], thickness: f32, color: [f32; 3]) -> Self {
        Self {
            start,
            end,
            thickness,
            color,
        }
    }
}
impl Shape for LineSegment {
    fn vertices(&self, segment: &WindowSegment) -> Vec<Vertex> {
        let x0 = segment.abs_x(&self.start[0]);
        let y0 = segment.abs_y(&self.start[1]);
        let x1 = segment.abs_x(&self.end[0]);
        let y1 = segment.abs_y(&self.end[1]);

        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            return vec![];
        }

        // Perpendicular offset scaled by half-thickness in each axis
        let nx = -dy / len;
        let ny = dx / len;
        let half_w = segment.abs_width(&Unit::Pixels(self.thickness as u32)) / 2.0;
        let half_h = segment.abs_height(&Unit::Pixels(self.thickness as u32)) / 2.0;
        let ox = nx * half_w;
        let oy = ny * half_h;

        vec![
            Vertex { position: [x0 + ox, y0 + oy, 0.0], color: self.color },
            Vertex { position: [x0 - ox, y0 - oy, 0.0], color: self.color },
            Vertex { position: [x1 - ox, y1 - oy, 0.0], color: self.color },
            Vertex { position: [x1 + ox, y1 + oy, 0.0], color: self.color },
        ]
    }

    fn indices(&self) -> &[u16] {
        &[0, 1, 2, 0, 2, 3]
    }
}

/// An element can either be a Shape or a TextSection
pub enum Element {
    Shape(Box<dyn Shape>),
    Text(Text),
}
