use std::collections::HashMap;
use std::sync::Arc;
use winit::window::Window;

use crate::shape::Element;
use crate::transform::{ContinuousNumericScale, NDC_SCALE, PERCENT_SCALE};

/// A value in a particular coordinate system
#[derive(Debug, Clone, Copy)]
pub enum Unit {
    // Pixels
    Pixels(u32),
    // Normalized Device Coordinates (-1,1)
    NDC(f32),
    // Percent (0, 100)
    Percent(f32),
}
impl Unit {
    /// Convert to a Unit::NDC
    pub(crate) fn as_ndc(&self, pixels: u32) -> Unit {
        match self {
            Unit::NDC(v) => Unit::NDC(*v),
            Unit::Pixels(v) => Unit::NDC(*v as f32 / pixels as f32),
            Unit::Percent(v) => Unit::NDC((v / 100. * 2.0) as f32),
        }
    }
    /// Convert to a Unit::Pixels
    pub(crate) fn as_px(&self, pixels: u32) -> Unit {
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
    pub(crate) fn as_f64(&self) -> f64 {
        match self {
            Unit::Pixels(v) => *v as f64,
            Unit::NDC(v) => *v as f64,
            Unit::Percent(v) => *v as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowSegment {
    /// Window segment in NDC units
    pub(crate) ndc_scale_x: ContinuousNumericScale,
    pub(crate) ndc_scale_y: ContinuousNumericScale,

    /// Window segment in pixel units
    pub(crate) pixel_scale_x: ContinuousNumericScale,
    pub(crate) pixel_scale_y: ContinuousNumericScale,
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
    pub fn new_root(window: Arc<Window>) -> Self {
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
    pub fn with_margin(&self, margin: Unit) -> Self {
        let margin_ndc_x = margin.as_ndc(self.pixel_scale_x.span() as u32);
        let margin_ndc_y = margin.as_ndc(self.pixel_scale_y.span() as u32);

        let margin_pixels_x = margin_ndc_x.as_px(self.pixel_scale_x.span() as u32);
        let margin_pixels_y = margin_ndc_y.as_px(self.pixel_scale_y.span() as u32);

        Self {
            ndc_scale_x: self.ndc_scale_x.shrink(margin_ndc_x.as_f64()),
            ndc_scale_y: self.ndc_scale_y.shrink(margin_ndc_y.as_f64()),
            pixel_scale_x: self.pixel_scale_x.shrink(margin_pixels_x.as_f64()),
            pixel_scale_y: self.pixel_scale_y.shrink(margin_pixels_y.as_f64()),
        }
    }

    /// Slice the segment along the X axis.
    /// start_frac=0.0 is the left edge, end_frac=1.0 is the right edge.
    pub fn slice_x(&self, start_frac: f64, end_frac: f64) -> Self {
        let ndc_min = self.ndc_scale_x.min + start_frac * self.ndc_scale_x.span();
        let ndc_max = self.ndc_scale_x.min + end_frac * self.ndc_scale_x.span();
        let px_min = self.pixel_scale_x.min + start_frac * self.pixel_scale_x.span();
        let px_max = self.pixel_scale_x.min + end_frac * self.pixel_scale_x.span();
        Self {
            ndc_scale_x: ContinuousNumericScale { min: ndc_min, max: ndc_max },
            ndc_scale_y: self.ndc_scale_y,
            pixel_scale_x: ContinuousNumericScale { min: px_min, max: px_max },
            pixel_scale_y: self.pixel_scale_y,
        }
    }

    /// Slice the segment along the Y axis.
    /// start_frac=0.0 is the top edge (NDC max), end_frac=1.0 is the bottom (NDC min).
    /// Pixel Y increases downward (top=min).
    pub fn slice_y(&self, start_frac: f64, end_frac: f64) -> Self {
        // NDC: top is max, bottom is min; inverted
        let ndc_max = self.ndc_scale_y.max - start_frac * self.ndc_scale_y.span();
        let ndc_min = self.ndc_scale_y.max - end_frac * self.ndc_scale_y.span();
        // Pixels: top is min, bottom is max; normal
        let px_min = self.pixel_scale_y.min + start_frac * self.pixel_scale_y.span();
        let px_max = self.pixel_scale_y.min + end_frac * self.pixel_scale_y.span();
        Self {
            ndc_scale_x: self.ndc_scale_x,
            ndc_scale_y: ContinuousNumericScale { min: ndc_min, max: ndc_max },
            pixel_scale_x: self.pixel_scale_x,
            pixel_scale_y: ContinuousNumericScale { min: px_min, max: px_max },
        }
    }
}

/// Named regions in a plot layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlotRegion {
    DataArea,
    XAxisGutter,
    YAxisGutter,
    Title,
    Legend,
    Caption,
    Spacer,
}

/// Size specification for a layout child
pub enum SizeSpec {
    Pixels(u32),
    Percent(f32),
    Flex(f32),
}

/// Axis along which children are split
pub enum SplitAxis {
    /// Rows: top to bottom
    Vertical,
    /// Columns: left to right
    Horizontal,
}

/// A node in the layout tree
pub enum LayoutNode {
    Leaf(PlotRegion),
    Split {
        axis: SplitAxis,
        children: Vec<(SizeSpec, LayoutNode)>,
    },
}

impl LayoutNode {
    /// Resolve this layout tree against a window segment, returning a map from
    /// PlotRegion to the WindowSegment it occupies.
    pub fn resolve(&self, segment: &WindowSegment) -> HashMap<PlotRegion, WindowSegment> {
        let mut map = HashMap::new();
        self.resolve_into(segment, &mut map);
        map
    }

    fn resolve_into(&self, segment: &WindowSegment, map: &mut HashMap<PlotRegion, WindowSegment>) {
        match self {
            LayoutNode::Leaf(region) => {
                if *region != PlotRegion::Spacer {
                    map.insert(*region, segment.clone());
                }
            }
            LayoutNode::Split { axis, children } => {
                let total_px = match axis {
                    SplitAxis::Vertical => segment.pixel_scale_y.span(),
                    SplitAxis::Horizontal => segment.pixel_scale_x.span(),
                };

                // Sum fixed sizes, accumulate flex weight
                let mut fixed_total: f64 = 0.0;
                let mut flex_total: f32 = 0.0;
                for (spec, _) in children.iter() {
                    match spec {
                        SizeSpec::Pixels(n) => fixed_total += *n as f64,
                        SizeSpec::Percent(p) => fixed_total += *p as f64 / 100.0 * total_px,
                        SizeSpec::Flex(w) => flex_total += w,
                    }
                }

                let remaining = (total_px - fixed_total).max(0.0);

                let mut current_frac = 0.0_f64;
                for (spec, child_node) in children.iter() {
                    let child_px = match spec {
                        SizeSpec::Pixels(n) => *n as f64,
                        SizeSpec::Percent(p) => *p as f64 / 100.0 * total_px,
                        SizeSpec::Flex(w) => {
                            if flex_total > 0.0 {
                                (*w / flex_total) as f64 * remaining
                            } else {
                                0.0
                            }
                        }
                    };
                    let child_frac = if total_px > 0.0 { child_px / total_px } else { 0.0 };
                    let end_frac = current_frac + child_frac;
                    let child_segment = match axis {
                        SplitAxis::Vertical => segment.slice_y(current_frac, end_frac),
                        SplitAxis::Horizontal => segment.slice_x(current_frac, end_frac),
                    };
                    child_node.resolve_into(&child_segment, map);
                    current_frac = end_frac;
                }
            }
        }
    }
}

/// Output of Blueprint::render — elements partitioned into named regions,
/// plus the layout tree describing how to position those regions.
pub struct PlotOutput {
    pub regions: HashMap<PlotRegion, Vec<Element>>,
    pub layout: LayoutNode,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_segment() -> WindowSegment {
        WindowSegment::new(
            ContinuousNumericScale { min: -1., max: 1. },
            ContinuousNumericScale { min: -1., max: 1. },
            ContinuousNumericScale { min: 0., max: 800. },
            ContinuousNumericScale { min: 0., max: 600. },
        )
    }

    #[test]
    fn slice_x_left_half() {
        let seg = root_segment();
        let left = seg.slice_x(0.0, 0.5);
        assert!((left.ndc_scale_x.min - (-1.0)).abs() < 1e-6);
        assert!((left.ndc_scale_x.max - 0.0).abs() < 1e-6);
        assert!((left.pixel_scale_x.min - 0.0).abs() < 1e-6);
        assert!((left.pixel_scale_x.max - 400.0).abs() < 1e-6);
        // Y unchanged
        assert_eq!(left.ndc_scale_y.min, seg.ndc_scale_y.min);
        assert_eq!(left.ndc_scale_y.max, seg.ndc_scale_y.max);
        assert_eq!(left.pixel_scale_y.min, seg.pixel_scale_y.min);
        assert_eq!(left.pixel_scale_y.max, seg.pixel_scale_y.max);
    }

    #[test]
    fn slice_x_right_half() {
        let seg = root_segment();
        let right = seg.slice_x(0.5, 1.0);
        assert!((right.ndc_scale_x.min - 0.0).abs() < 1e-6);
        assert!((right.ndc_scale_x.max - 1.0).abs() < 1e-6);
        assert!((right.pixel_scale_x.min - 400.0).abs() < 1e-6);
        assert!((right.pixel_scale_x.max - 800.0).abs() < 1e-6);
    }

    #[test]
    fn slice_y_top_half() {
        let seg = root_segment();
        // start_frac=0.0 is top, end_frac=0.5 is halfway down
        let top = seg.slice_y(0.0, 0.5);
        // NDC y: top=max=1.0, halfway=0.0
        assert!((top.ndc_scale_y.max - 1.0).abs() < 1e-6, "ndc_max={}", top.ndc_scale_y.max);
        assert!((top.ndc_scale_y.min - 0.0).abs() < 1e-6, "ndc_min={}", top.ndc_scale_y.min);
        // Pixel y: top=min=0, halfway=300
        assert!((top.pixel_scale_y.min - 0.0).abs() < 1e-6);
        assert!((top.pixel_scale_y.max - 300.0).abs() < 1e-6);
        // X unchanged
        assert_eq!(top.ndc_scale_x.min, seg.ndc_scale_x.min);
        assert_eq!(top.ndc_scale_x.max, seg.ndc_scale_x.max);
    }

    #[test]
    fn slice_y_bottom_half() {
        let seg = root_segment();
        let bottom = seg.slice_y(0.5, 1.0);
        // NDC y: halfway=0.0, bottom=min=-1.0
        assert!((bottom.ndc_scale_y.max - 0.0).abs() < 1e-6, "ndc_max={}", bottom.ndc_scale_y.max);
        assert!((bottom.ndc_scale_y.min - (-1.0)).abs() < 1e-6, "ndc_min={}", bottom.ndc_scale_y.min);
        // Pixel y: halfway=300, bottom=600
        assert!((bottom.pixel_scale_y.min - 300.0).abs() < 1e-6);
        assert!((bottom.pixel_scale_y.max - 600.0).abs() < 1e-6);
    }

    #[test]
    fn slice_x_full_is_identity() {
        let seg = root_segment();
        let full = seg.slice_x(0.0, 1.0);
        assert_eq!(full.ndc_scale_x.min, seg.ndc_scale_x.min);
        assert_eq!(full.ndc_scale_x.max, seg.ndc_scale_x.max);
        assert_eq!(full.pixel_scale_x.min, seg.pixel_scale_x.min);
        assert_eq!(full.pixel_scale_x.max, seg.pixel_scale_x.max);
    }

    #[test]
    fn slice_y_full_is_identity() {
        let seg = root_segment();
        let full = seg.slice_y(0.0, 1.0);
        assert_eq!(full.ndc_scale_y.min, seg.ndc_scale_y.min);
        assert_eq!(full.ndc_scale_y.max, seg.ndc_scale_y.max);
        assert_eq!(full.pixel_scale_y.min, seg.pixel_scale_y.min);
        assert_eq!(full.pixel_scale_y.max, seg.pixel_scale_y.max);
    }

}
