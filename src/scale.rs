use crate::aesthetic::{Aesthetic, AestheticFamily};
use crate::column::{AesData, MappedColumn, RawColumn};
use crate::layout::{PlotRegion, Unit};
use crate::shape::{Element, HAlign, Rectangle, Text, VAlign};
use crate::theme::Theme;
use crate::transform::{nice_bounds, nice_step, ContinuousNumericScale, NDC_SCALE};

/// Enumerate tick values from `min` to `max` using the given step size.
/// Both `min` and `max` must already be multiples of `step` (i.e. nice bounds).
fn ticks_from_step(min: f64, max: f64, step: f64) -> Vec<f64> {
    if step <= 0.0 || !step.is_finite() {
        return vec![min];
    }
    let mut ticks = vec![];
    let mut tick = min;
    while tick <= max + step * 1e-10 {
        ticks.push(tick);
        tick += step;
    }
    ticks
}

/// Scales produce legends.
/// They are used to convert between the projection on the screen and the data.
///
/// For example, a continuous numeric scale maps length on the screen to
/// the mapped variable. A discrete color scale maps color to a category.
pub trait Scale {
    /// Transform aesthetic-keyed data for this scale.
    ///
    /// By default, no transformations are applied
    fn transform(&self, data: AesData) -> AesData {
        data
    }

    /// Map an array of raw column values through the scale, returning mapped output.
    fn map(&self, v: &RawColumn) -> Result<MappedColumn, String>;

    /// Append a set of raw column values to the scale
    fn append(&mut self, v: &RawColumn) -> Result<(), String>;

    /// Fit the scale to the data
    fn fit(&mut self) -> Result<(), String>;

    /// Render the elements for this scale, returning them tagged with their target region.
    fn render(&self, theme: &Theme) -> (PlotRegion, Vec<Element>);

    /// Return the family this scale belongs to.
    fn aesthetic_family(&self) -> AestheticFamily;

    /// Create a fresh, unfitted copy of this scale (same type/axis, no data).
    fn clone_unfitted(&self) -> Box<dyn Scale>;
}

/// Format a set of tick values with consistent suffix and just enough decimal places
/// that consecutive ticks are distinguishable.
///
/// Suffix is chosen from the largest absolute value; precision is derived from the
/// step size so that e.g. [1.38M, 1.40M, 1.42M] never collapses to ["1.4M", "1.4M", "1.4M"].
fn format_ticks(values: &[f64]) -> Vec<String> {
    if values.is_empty() {
        return vec![];
    }

    let max_abs = values.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    let (divisor, suffix) = if max_abs >= 1_000_000_000.0 {
        (1_000_000_000.0, "B")
    } else if max_abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if max_abs >= 10_000.0 {
        (1_000.0, "K")
    } else {
        (1.0, "")
    };

    // Minimum decimal places to make consecutive ticks distinct after scaling
    let decimals: usize = if values.len() >= 2 {
        let step = (values[1] - values[0]).abs() / divisor;
        if step > 0.0 {
            let d = (-step.log10().floor()) as i32;
            d.max(0).min(6) as usize
        } else {
            0
        }
    } else {
        0
    };

    values.iter().map(|&v| {
        if v == 0.0 {
            return "0".to_string();
        }
        let scaled = v / divisor;
        if decimals == 0 {
            format!("{}{}", scaled as i64, suffix)
        } else {
            format!("{:.prec$}{}", scaled, suffix, prec = decimals)
        }
    }).collect()
}

const TARGET_TICK_COUNT: usize = 5;

/// Which axis a positional scale operates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// A unified continuous positional scale for both X and Y axes.
pub struct ScalePositionContinuous {
    axis: Axis,
    data_scale: Option<ContinuousNumericScale>,
    /// Step size computed from the raw data range during fit(); used for tick generation
    /// so that the same step that aligned the axis bounds also spaces the tick marks.
    tick_step: Option<f64>,
}

impl ScalePositionContinuous {
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            data_scale: None,
            tick_step: None,
        }
    }

    fn render_x_axis(&self, theme: &Theme) -> (PlotRegion, Vec<Element>) {
        let mut elements = vec![];

        // Axis line: full width at top of gutter (adjacent to DataArea)
        let xaxis = Rectangle::new(
            [Unit::NDC(0.0), Unit::NDC(1.0)],
            Unit::NDC(2.0),
            Unit::Pixels(1),
            theme.axis_color,
        );
        elements.push(Element::Rect(xaxis));

        let s = &self.data_scale.expect("Scale isn't fit");
        let step = self.tick_step.expect("Scale isn't fit");
        let tick_values = ticks_from_step(s.min, s.max, step);
        let labels = format_ticks(&tick_values);
        for (tick_value, label) in tick_values.iter().zip(labels) {
            let x_ndc = s.map_position(&NDC_SCALE, *tick_value) as f32;

            // Tick mark hangs down from top edge
            let tick = Rectangle::new(
                [Unit::NDC(x_ndc), Unit::NDC(1.0)],
                Unit::Pixels(1),
                Unit::Pixels(6),
                theme.axis_color,
            );
            elements.push(Element::Rect(tick));

            // Tick label just below tick marks
            elements.push(Element::Text(Text::centered(
                label,
                theme.tick_label_font_size,
                (Unit::NDC(x_ndc), Unit::NDC(0.8)),
            )));
        }

        (PlotRegion::XAxisGutter, elements)
    }

    fn render_y_axis(&self, theme: &Theme) -> (PlotRegion, Vec<Element>) {
        let mut elements: Vec<Element> = vec![];

        // Axis line: at right edge of gutter (adjacent to DataArea), full height
        let yaxis = Rectangle::new(
            [Unit::NDC(1.0), Unit::NDC(0.0)],
            Unit::Pixels(1),
            Unit::NDC(2.0),
            theme.axis_color,
        );
        elements.push(Element::Rect(yaxis));

        let s = &self.data_scale.expect("Scale isn't fit");
        let step = self.tick_step.expect("Scale isn't fit");
        let tick_values = ticks_from_step(s.min, s.max, step);
        let labels = format_ticks(&tick_values);
        for (tick_value, label) in tick_values.iter().zip(labels) {
            let y_ndc = s.map_position(&NDC_SCALE, *tick_value) as f32;

            // Tick mark protrudes left from right edge
            let tick = Rectangle::new(
                [Unit::NDC(1.0), Unit::NDC(y_ndc)],
                Unit::Pixels(6),
                Unit::Pixels(1),
                theme.axis_color,
            );
            elements.push(Element::Rect(tick));
            // Tick label: right-aligned, flush against the tick mark with a small gap
            elements.push(Element::Text(
                Text::new(
                    label,
                    theme.tick_label_font_size,
                    (Unit::Percent(85.0), Unit::NDC(y_ndc)),
                )
                .with_h_align(HAlign::Right)
                .with_v_align(VAlign::Center),
            ));
        }

        (PlotRegion::YAxisGutter, elements)
    }
}

impl Scale for ScalePositionContinuous {
    fn fit(&mut self) -> Result<(), String> {
        if let Some(s) = &self.data_scale {
            let (nice_min, nice_max) = nice_bounds(s.min, s.max, TARGET_TICK_COUNT);
            // Compute step from the raw range; fall back to the expanded range for the
            // degenerate case (min == max) where nice_step would receive a zero-width range.
            let step = if s.min == s.max {
                nice_step(nice_min, nice_max, TARGET_TICK_COUNT)
            } else {
                nice_step(s.min, s.max, TARGET_TICK_COUNT)
            };
            self.data_scale = Some(ContinuousNumericScale {
                min: nice_min,
                max: nice_max,
            });
            self.tick_step = Some(step);
        }
        Ok(())
    }

    fn map(&self, v: &RawColumn) -> Result<MappedColumn, String> {
        let values = v.as_f64()?;

        if let Some(s) = &self.data_scale {
            Ok(MappedColumn::UnitArray(
                values
                    .iter()
                    .map(|v| Unit::NDC(s.map_position(&NDC_SCALE, *v) as f32))
                    .collect(),
            ))
        } else {
            Err("Scale is uninitialized".into())
        }
    }

    fn render(&self, theme: &Theme) -> (PlotRegion, Vec<Element>) {
        match self.axis {
            Axis::X => self.render_x_axis(theme),
            Axis::Y => self.render_y_axis(theme),
        }
    }

    fn aesthetic_family(&self) -> AestheticFamily {
        match self.axis {
            Axis::X => AestheticFamily::HorizontalPosition,
            Axis::Y => AestheticFamily::VerticalPosition,
        }
    }

    fn append(&mut self, v: &RawColumn) -> Result<(), String> {
        let new_scale = ContinuousNumericScale::from_vec(&v.as_f64()?);
        if let Some(s) = &self.data_scale {
            self.data_scale = Some(s.union(&new_scale));
        } else {
            self.data_scale = Some(new_scale);
        }
        Ok(())
    }

    fn clone_unfitted(&self) -> Box<dyn Scale> {
        Box::new(ScalePositionContinuous::new(self.axis))
    }
}

/// Convert HSL (h in 0..360, s and l in 0..1) to RGB (each in 0..1).
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match h_prime as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r1 + m, g1 + m, b1 + m]
}

/// A discrete color scale that maps categorical string values to colors.
pub struct ScaleColorDiscrete {
    categories: Vec<String>,
    palette: Vec<[f32; 3]>,
}

impl ScaleColorDiscrete {
    pub fn new() -> Self {
        Self {
            categories: vec![],
            palette: vec![],
        }
    }
}

impl Scale for ScaleColorDiscrete {
    fn append(&mut self, v: &RawColumn) -> Result<(), String> {
        match v {
            RawColumn::StringArray(strings) => {
                for s in strings {
                    if !self.categories.contains(s) {
                        self.categories.push(s.clone());
                    }
                }
                Ok(())
            }
            _ => Err("ScaleColorDiscrete expects StringArray".into()),
        }
    }

    fn fit(&mut self) -> Result<(), String> {
        let n = self.categories.len();
        self.palette = (0..n)
            .map(|i| {
                let hue = (i as f32 / n as f32) * 360.0;
                hsl_to_rgb(hue, 0.65, 0.55)
            })
            .collect();
        Ok(())
    }

    fn map(&self, v: &RawColumn) -> Result<MappedColumn, String> {
        match v {
            RawColumn::StringArray(strings) => {
                let colors: Vec<[f32; 3]> = strings
                    .iter()
                    .map(|s| {
                        let idx = self
                            .categories
                            .iter()
                            .position(|c| c == s)
                            .expect("category not found in scale");
                        self.palette[idx]
                    })
                    .collect();
                Ok(MappedColumn::ColorArray(colors))
            }
            _ => Err("ScaleColorDiscrete expects StringArray".into()),
        }
    }

    fn render(&self, theme: &Theme) -> (PlotRegion, Vec<Element>) {
        let mut elements = vec![];
        // Region-local coords: NDC(-1..1) spans the legend segment
        let y_start = 0.7_f32;
        let spacing = 0.18_f32;

        for (i, cat) in self.categories.iter().enumerate() {
            let y = y_start - (i as f32 * spacing);
            let [r, g, b] = self.palette[i];
            let swatch = Rectangle::new(
                [Unit::Percent(10.0), Unit::NDC(y)],
                Unit::Pixels(14),
                Unit::Pixels(14),
                [r, g, b, 1.0],
            );
            elements.push(Element::Rect(swatch));
            elements.push(Element::Text(
                Text::new(
                    cat.clone(),
                    theme.legend_label_font_size,
                    (Unit::Percent(22.0), Unit::NDC(y)),
                )
                .with_v_align(VAlign::Center),
            ));
        }
        (PlotRegion::Legend, elements)
    }

    fn aesthetic_family(&self) -> AestheticFamily {
        AestheticFamily::Color
    }

    fn clone_unfitted(&self) -> Box<dyn Scale> {
        Box::new(ScaleColorDiscrete::new())
    }
}

/// A stat transform applied before rendering a geometry.
pub trait StatTransform {
    /// Transform aesthetic-keyed data before plotting a geometry
    fn transform(&self, data: &AesData) -> AesData;
}

/// A position adjustment applied after stat transforms.
pub trait PositionAdjustment {}

pub struct IdentityTransform;
impl StatTransform for IdentityTransform {
    fn transform(&self, data: &AesData) -> AesData {
        data.clone()
    }
}
impl PositionAdjustment for IdentityTransform {}

/// Return the default scale for a given aesthetic, if one exists.
pub fn default_scale_for(aesthetic: &Aesthetic) -> Option<Box<dyn Scale>> {
    match aesthetic {
        Aesthetic::X => Some(Box::new(ScalePositionContinuous::new(Axis::X))),
        Aesthetic::Y => Some(Box::new(ScalePositionContinuous::new(Axis::Y))),
        Aesthetic::Color => Some(Box::new(ScaleColorDiscrete::new())),
        Aesthetic::Group => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hsl_to_rgb_red() {
        let [r, g, b] = hsl_to_rgb(0.0, 1.0, 0.5);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);
    }

    #[test]
    fn hsl_to_rgb_green() {
        let [r, g, b] = hsl_to_rgb(120.0, 1.0, 0.5);
        assert!(r.abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!(b.abs() < 0.01);
    }

    #[test]
    fn hsl_to_rgb_blue() {
        let [r, g, b] = hsl_to_rgb(240.0, 1.0, 0.5);
        assert!(r.abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!((b - 1.0).abs() < 0.01);
    }

    #[test]
    fn scale_color_discrete_round_trip() {
        let mut scale = ScaleColorDiscrete::new();
        let input = RawColumn::StringArray(vec!["a".into(), "b".into(), "a".into(), "c".into()]);
        scale.append(&input).unwrap();
        scale.fit().unwrap();

        let mapped = scale.map(&input).unwrap();
        match mapped {
            MappedColumn::ColorArray(colors) => {
                assert_eq!(colors.len(), 4);
                // "a" appears at index 0 and 2 — same color
                assert_eq!(colors[0], colors[2]);
                // "a", "b", "c" are different colors
                assert_ne!(colors[0], colors[1]);
                assert_ne!(colors[1], colors[3]);
                assert_ne!(colors[0], colors[3]);
            }
            _ => panic!("Expected ColorArray"),
        }
    }

    #[test]
    fn scale_color_discrete_preserves_insertion_order() {
        let mut scale = ScaleColorDiscrete::new();
        let input =
            RawColumn::StringArray(vec!["banana".into(), "apple".into(), "banana".into()]);
        scale.append(&input).unwrap();
        scale.fit().unwrap();

        // First unique value gets hue 0, second gets hue 180
        let theme = crate::theme::Theme::default();
        let (region, legend) = scale.render(&theme);
        assert_eq!(region, PlotRegion::Legend);
        // Legend should have 2 entries (swatch + label each)
        assert_eq!(legend.len(), 4);
    }

    #[test]
    fn scale_color_discrete_rejects_float() {
        let mut scale = ScaleColorDiscrete::new();
        let input = RawColumn::FloatArray(vec![1.0, 2.0]);
        assert!(scale.append(&input).is_err());
    }
}
