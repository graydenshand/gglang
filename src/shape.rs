#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::window::{self, Window};

use crate::transform::{ContinuousNumericScale, NDC_SCALE, PERCENT_SCALE};

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

/// A value in a particular coordinate system
#[derive(Debug, Clone, Copy)]
pub enum Unit {
    // Pixels
    Pixels(u32),
    // Normalized Device Coordinates (-1,1)
    NDC(f32),
    // Percent (0, 1)
    Percent(f32),
}
impl Unit {
    /// Convert to a Unit::NDC
    fn as_ndc(&self, pixels: u32) -> Unit {
        match self {
            Unit::NDC(v) => Unit::NDC(*v),
            Unit::Pixels(v) => Unit::NDC(*v as f32 / pixels as f32),
            Unit::Percent(v) => Unit::NDC((v / 100. * 2.0) as f32),
        }
    }
    /// Convert to a Unit::Pixels
    fn as_px(&self, pixels: u32) -> Unit {
        match self {
            Unit::NDC(v) => Unit::Pixels((*v / 2.0 * pixels as f32) as u32),
            Unit::Pixels(v) => Unit::Pixels(*v),
            Unit::Percent(v) => Unit::Pixels((v / 100. * pixels as f32) as u32),
        }
    }
    /// Extract the inner value, and coerce to f64.
    ///
    /// WARNING: this function isn't completely safe. All enum variants will
    /// return a compliant value, but the interpretation of that value depends
    /// on the variant. You should only use this when you already know the
    /// value's variant.
    fn as_f64(&self) -> f64 {
        match self {
            Unit::Pixels(v) => *v as f64,
            Unit::NDC(v) => *v as f64,
            Unit::Percent(v) => *v as f64,
        }
    }
}

#[derive(Debug)]
pub struct WindowSegment {
    /// Window segment in NDC units
    ndc_scale_x: ContinuousNumericScale,
    ndc_scale_y: ContinuousNumericScale,

    /// Window segment in pixel units
    pixel_scale_x: ContinuousNumericScale,
    pixel_scale_y: ContinuousNumericScale,
}
impl WindowSegment {
    pub fn new(
        ndc_scale_x: ContinuousNumericScale,
        ndc_scale_y: ContinuousNumericScale,
        pixel_scale_x: ContinuousNumericScale,
        pixel_scale_y: ContinuousNumericScale,
    ) -> Self {
        Self {
            ndc_scale_x,
            ndc_scale_y,
            pixel_scale_x,
            pixel_scale_y,
        }
    }

    /// Create a new WindowSegment for the entire window.
    pub fn new_root(window: std::sync::Arc<Window>) -> Self {
        Self::new(
            NDC_SCALE,
            NDC_SCALE,
            ContinuousNumericScale {
                min: 0.,
                max: window.inner_size().width as f64,
            },
            ContinuousNumericScale {
                min: 0.,
                max: window.inner_size().height as f64,
            },
        )
    }

    /// Map an x position to absolute window coordinates
    pub fn abs_x(&self, x: &Unit) -> f32 {
        match x {
            // relative NDC coordinates
            Unit::NDC(v) => NDC_SCALE.map_position(&self.ndc_scale_x, *v as f64) as f32,
            // pixel coordinates
            Unit::Pixels(v) => self
                .pixel_scale_x
                .map_position(&self.ndc_scale_x, *v as f64) as f32,
            Unit::Percent(v) => PERCENT_SCALE.map_position(&self.ndc_scale_x, *v as f64) as f32,
        }
    }

    /// Map a width unit to absolute window coordinates
    pub fn abs_width(&self, x: &Unit) -> f32 {
        match x {
            Unit::NDC(v) => NDC_SCALE.map_size(&self.ndc_scale_x, *v as f64) as f32,
            Unit::Pixels(v) => self.pixel_scale_x.map_size(&self.ndc_scale_x, *v as f64) as f32,
            Unit::Percent(v) => PERCENT_SCALE.map_size(&self.ndc_scale_x, *v as f64) as f32,
        }
    }

    /// Map a y position to absolute window coordinates
    pub fn abs_y(&self, y: &Unit) -> f32 {
        match y {
            Unit::NDC(v) => NDC_SCALE.map_position(&self.ndc_scale_y, *v as f64) as f32,
            Unit::Pixels(v) => self
                .pixel_scale_y
                .map_position(&self.ndc_scale_y, *v as f64) as f32,
            Unit::Percent(v) => PERCENT_SCALE.map_position(&self.ndc_scale_y, *v as f64) as f32,
        }
    }

    /// Map a height unit to absolute window coordinates
    pub fn abs_height(&self, y: &Unit) -> f32 {
        match y {
            Unit::NDC(v) => NDC_SCALE.map_size(&self.ndc_scale_y, *v as f64) as f32,
            Unit::Pixels(v) => self.pixel_scale_y.map_size(&self.ndc_scale_y, *v as f64) as f32,
            Unit::Percent(v) => PERCENT_SCALE.map_size(&self.ndc_scale_y, *v as f64) as f32,
        }
    }

    /// Create a new WindowSegment with margin (padding) on each side
    ///
    /// The margin is subtracted from all sides of the current window segment,
    /// creating a smaller window segment with the specified padding.
    pub fn with_margin(&self, margin: Unit) -> Self {
        // Convert margin to NDC and pixel units for both axes
        let margin_ndc_x = margin.as_ndc(self.pixel_scale_x.span() as u32);
        let margin_ndc_y = margin.as_ndc(self.pixel_scale_y.span() as u32);

        let margin_pixels_x = margin_ndc_x.as_px(self.pixel_scale_x.span() as u32);
        let margin_pixels_y = margin_ndc_y.as_px(self.pixel_scale_y.span() as u32);

        // Create new scales with margin applied
        Self {
            ndc_scale_x: self.ndc_scale_x.shrink(margin_ndc_x.as_f64()),
            ndc_scale_y: self.ndc_scale_y.shrink(margin_ndc_y.as_f64()),
            pixel_scale_x: self.pixel_scale_x.shrink(margin_pixels_x.as_f64()),
            pixel_scale_y: self.pixel_scale_y.shrink(margin_pixels_y.as_f64()),
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

#[derive(Clone, Debug)]
pub struct Text {
    value: String,
    font_size: f32,
    position: (Unit, Unit),
}
impl Text {
    pub fn new(value: String, font_size: f32, position: (Unit, Unit)) -> Self {
        Self {
            value,
            font_size,
            position,
        }
    }

    /// Return the text as a wgpu_text::glyph_brush::Section
    pub fn as_section<'a>(
        &'a self,
        window_segment: &WindowSegment,
    ) -> wgpu_text::glyph_brush::Section<'a> {
        println!("position: {:?}", (self.position_as_pixels(window_segment)));
        wgpu_text::glyph_brush::Section::default()
            .with_screen_position(self.position_as_pixels(window_segment))
            .add_text(wgpu_text::glyph_brush::Text::new(&self.value).with_scale(self.font_size))
    }

    fn position_as_pixels(&self, window_segment: &WindowSegment) -> (f32, f32) {
        // x position in ndc coords
        let x_ndc = window_segment.abs_x(&self.position.0);

        // convert to px
        let x = window_segment
            .ndc_scale_x
            .map_position(&window_segment.pixel_scale_x, x_ndc.into());

        println!("x: {}", x);

        // y position in ndc coords
        let y_ndc = window_segment.abs_y(&self.position.1);

        // convert to py
        let y = window_segment
            .ndc_scale_y
            .map_position(&window_segment.pixel_scale_y, y_ndc.into());

        (x as f32, y as f32)
    }
}

/// An element can either be a Shape or a TextSection
pub enum Element {
    Shape(Box<dyn Shape>),
    Text(Text),
}
