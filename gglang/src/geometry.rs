#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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


/// A 1d vector in a particular coordinate system, can represent either
/// position or length along an axis.
#[derive(Debug, Clone, Copy)]
pub enum Unit {
    // vector in pixel coordinates
    Pixels(u32),
    // vector in normalized device coordinates (-1,1)
    NDC(f32)
}
impl Unit {
    /// Convert a vector in pixel coordinates to a vector in NDC
    /// 
    /// Args
    /// - pixels: the number of pixels along the dimension
    fn px_to_ndc(&self, pixels: f32) -> Result<Unit, &str> {
        if let Self::Pixels(v) = self {
            Ok(Self::NDC(
                2. / pixels * *v as f32,
            ))
        } else {
            Err("Not a Pixel coordinate")
        }
    }
}

pub struct WindowSegment {
    /// Window segment in NDI units
    ndi_scale_x: ContinuousNumericScale,
    ndi_scale_y: ContinuousNumericScale,

    /// Window segment in pixel units
    pixel_scale_x: ContinuousNumericScale,
    pixel_scale_y: ContinuousNumericScale,
}
impl WindowSegment {
    /// Map an x unit to absolute window coordinates
    pub fn abs_x(&self, x: &Unit) -> f32 {
        match x {
            // relative NDC coordinates
            Unit::NDC(v) => {
                let relative_ndi_scale = ContinuousNumericScale { min: -1., max: 1.};
                relative_ndi_scale.map_to(&self.ndi_scale_x, *v as f64) as f32
            },
            // pixel coordinates
            Unit::Pixels(v) => {
                self.pixel_scale_x.map_to(&self.ndi_scale_x, *v as f64) as f32
            }
        }
    }

    pub fn abs_y(&self, y: &Unit) -> f32 {
        match y {
            // relative NDC coordinates
            Unit::NDC(v) => {
                let relative_ndi_scale = ContinuousNumericScale { min: -1., max: 1.};
                relative_ndi_scale.map_to(&self.ndi_scale_y, *v as f64) as f32
            },
            // pixel coordinates
            Unit::Pixels(v) => {
                self.pixel_scale_x.map_to(&self.ndi_scale_y, *v as f64) as f32
            }
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
    fn vertices(&self, segment: WindowSegment) -> Vec<Vertex>;

    /// Indices of vertex triplets for drawing triangles
    fn indices(&self) -> &[u16];
}

pub struct Rectangle {
    /// X,Y position
    position: [Unit; 2],
    /// Dimensions of the rectangle either in relative NDI units or in Pixels
    width: Unit,
    height: Unit,
    /// Rectangle fill color
    color: [f32; 3]
}
impl Rectangle {
    /// Create a new rectangle
    /// 
    /// Args:
    /// - position: X,Y position of center of the rectangle
    /// - width: The width of the rectange
    /// - height: The height of the rectange
    /// - color: The fill color of the rectangles
    pub fn new(position: [Unit; 2], width: Unit, height: Unit, color: [f32; 3], ) -> Self {
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
    fn vertices(&self, segment: WindowSegment) -> Vec<Vertex> {
        // Convert position from relative to absolute units
        let abs_position = [
            segment.abs_x(&self.position[0]),
            segment.abs_x(&self.position[1]),
        ];
        let abs_width = segment.abs_x(&self.width);
        let abs_height = segment.abs_x(&self.height);

        let vertices = vec![
            // Top left
            Vertex {
                position: [abs_position[0] - abs_width / 2., abs_position[1] + abs_height / 2., 0.0],
                color: self.color,
            },
            // Bottom left
            Vertex {
                position: [abs_position[0] - abs_width / 2., abs_position[1] - abs_height / 2., 0.0],
                color: self.color,
            },
            // Bottom right
            Vertex {
                position: [abs_position[0] + abs_width / 2., abs_position[1] - abs_height / 2., 0.0],
                color: self.color,
            },
            // Top right
            Vertex {
                position: [abs_position[0] + abs_width / 2., abs_position[1] + abs_height / 2., 0.0],
                color: self.color,
            },
        ];
        vertices
    }

    fn indices(&self) -> &[u16] {
        &[
            0, 1, 2,
            0, 3, 2,
        ]
    }
}
