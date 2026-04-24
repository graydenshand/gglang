use crate::layout::Unit;

pub struct Rectangle {
    /// X,Y position
    pub position: [Unit; 2],
    /// Dimensions of the rectangle either in relative NDC units or in Pixels
    pub width: Unit,
    pub height: Unit,
    /// Rectangle fill color (RGBA)
    pub color: [f32; 4],
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextRotation {
    #[default]
    None,
    Ccw90, // Y-axis labels
    Cw90,  // Facet row labels
}

#[derive(Clone, Debug, Default)]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
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
    pub rotation: TextRotation,
    pub wrap: bool,
    pub color: [f32; 4],
}
impl Text {
    pub fn new(value: String, font_size: f32, position: (Unit, Unit)) -> Self {
        Self {
            value,
            font_size,
            position,
            h_align: HAlign::Left,
            v_align: VAlign::Top,
            rotation: TextRotation::None,
            wrap: false,
            color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn centered(value: String, font_size: f32, position: (Unit, Unit)) -> Self {
        Self {
            value,
            font_size,
            position,
            h_align: HAlign::Center,
            v_align: VAlign::Top,
            rotation: TextRotation::None,
            wrap: false,
            color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn with_wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn with_h_align(mut self, h_align: HAlign) -> Self {
        self.h_align = h_align;
        self
    }

    pub fn with_v_align(mut self, v_align: VAlign) -> Self {
        self.v_align = v_align;
        self
    }

    pub fn with_rotation(mut self, rotation: TextRotation) -> Self {
        self.rotation = rotation;
        self
    }
}

/// Domain-level polyline data — one per group/series, tessellated in Frame.
pub struct PolylineData {
    pub points: Vec<[Unit; 2]>,
    pub thickness: f32, // pixels
    pub colors: Vec<[f32; 4]>, // per-point RGBA
}

/// Domain-level point data — position, size, and color in layout-relative units.
/// Converted to GPU instances in Frame.
pub struct PointData {
    pub position: [Unit; 2],
    pub size: Unit,
    pub color: [f32; 4],
}

/// Domain-level arc/wedge data — used for pie charts and polar bar rendering.
pub struct ArcData {
    pub center: [Unit; 2],
    pub inner_radius: Unit,
    pub outer_radius: Unit,
    pub start_angle: f32, // radians
    pub end_angle: f32,   // radians
    pub color: [f32; 4],
}

/// Domain-level gradient bar — a rectangular region filled with a color gradient.
/// `stops` are ordered bottom→top (min→max value). Backends render via SVG
/// `<linearGradient>` or GPU strip tessellation.
pub struct GradientBarData {
    pub position: [Unit; 2], // top-left corner in region-local NDC
    pub width: Unit,
    pub height: Unit,
    pub stops: Vec<[f32; 3]>, // RGB, bottom→top == min→max
}

/// An element can be a Rect, Point, Polyline, Text, Arc, or GradientBar
pub enum Element {
    Rect(Rectangle),
    Point(PointData),
    Polyline(PolylineData),
    Text(Text),
    Arc(ArcData),
    GradientBar(GradientBarData),
}
