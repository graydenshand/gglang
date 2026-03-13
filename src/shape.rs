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

/// An element can be a Rect, Point, Polyline, or Text
pub enum Element {
    Rect(Rectangle),
    Point(PointData),
    Polyline(PolylineData),
    Text(Text),
}
