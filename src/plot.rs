use crate::layout::{LayoutNode, PlotOutput, PlotRegion, SizeSpec, SplitAxis, Unit};
use crate::shape::{Element, PolylineData, PointData, Rectangle, Text, VAlign};
use crate::theme::Theme;
use crate::transform::{nice_ticks, ContinuousNumericScale, NDC_SCALE};
use std::collections::HashMap;

/// A model of a plot
///
/// Mappings
/// Layers
/// Scales
/// Facets
/// Coordinates
/// Theme
pub struct Blueprint<'a> {
    /// Default mappings of data to visual channels
    mappings: Vec<Mapping>,

    /// Plot Layers
    layers: Vec<Layer>,

    /// Scales
    scales: Vec<Box<dyn Scale>>,

    /// Faceting
    facets: Vec<Variable>,

    /// Coordinate System
    coordinates: CoordinateSystem,

    /// Theme settings
    theme: &'a Theme,

    /// Optional plot title
    pub title: Option<String>,

    /// Optional plot caption
    pub caption: Option<String>,

    /// Optional x-axis label (defaults to mapped column name)
    pub x_label: Option<String>,

    /// Optional y-axis label (defaults to mapped column name)
    pub y_label: Option<String>,
}
impl<'a> Blueprint<'a> {
    /// Create a new, empty blueprint
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            mappings: vec![],
            layers: vec![],
            scales: vec![],
            facets: vec![],
            coordinates: CoordinateSystem::Cartesian,
            theme,
            title: None,
            caption: None,
            x_label: None,
            y_label: None,
        }
    }

    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn with_scale(mut self, scale: Box<dyn Scale>) -> Self {
        self.scales.push(scale);
        self
    }

    pub fn has_scale_for_family(&self, family: AestheticFamily) -> bool {
        self.scales.iter().any(|s| s.aesthetic_family() == family)
    }

    pub fn with_facet(mut self, facet: Variable) -> Self {
        self.facets.push(facet);
        self
    }

    pub fn with_mapping(mut self, mapping: Mapping) -> Self {
        self.mappings.push(mapping);
        self
    }

    pub fn with_coordinates(mut self, coordinates: CoordinateSystem) -> Self {
        self.coordinates = coordinates;
        self
    }

    pub fn with_theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_caption(mut self, caption: String) -> Self {
        self.caption = Some(caption);
        self
    }

    pub fn with_x_label(mut self, label: String) -> Self {
        self.x_label = Some(label);
        self
    }

    pub fn with_y_label(mut self, label: String) -> Self {
        self.y_label = Some(label);
        self
    }

    /// Render a plot from this blueprint.
    ///
    /// Data is provided with raw column names; the blueprint's mappings are
    /// applied to bind columns to aesthetic channels before rendering.
    pub fn render(&mut self, raw_data: PlotData) -> Result<PlotOutput, String> {
        // Step 1: Column rename — PlotData (string-keyed) → AesData (Aesthetic-keyed)
        let mut aes_data = AesData::new();
        let mut mapped_aesthetics: Vec<Aesthetic> = vec![];
        for mapping in &self.mappings {
            let col = raw_data
                .get(&mapping.variable)
                .ok_or_else(|| format!("Column '{}' not found in data", mapping.variable))?;
            aes_data.insert(mapping.aesthetic, col.clone());
            mapped_aesthetics.push(mapping.aesthetic);
        }

        // Auto-map: if a data column name matches an aesthetic name and that
        // aesthetic isn't already explicitly mapped, use it as the default.
        for aes in Aesthetic::all() {
            if mapped_aesthetics.contains(aes) {
                continue;
            }
            if let Some(col) = raw_data.get(aes.name()) {
                aes_data.insert(*aes, col.clone());
                mapped_aesthetics.push(*aes);
                self.mappings.push(Mapping {
                    aesthetic: *aes,
                    variable: aes.name().to_string(),
                });

                // Auto-create scale if one doesn't exist for this family
                let family = aes.family();
                if !self.scales.iter().any(|s| s.aesthetic_family() == family) {
                    if let Some(scale) = aes.default_scale() {
                        self.scales.push(scale);
                    }
                }
            }
        }

        // Validate required mappings are satisfied for all geometries
        for g in self.layers.iter().map(|l| &l.geometry) {
            for aes in g.required_aesthetics() {
                if !aes_data.contains(aes) {
                    return Err(format!("Missing required aesthetic {}", aes.name()));
                }
            }
        }

        // Step 2: Scale transforms (identity by default)
        for scale in &self.scales {
            aes_data = scale.transform(aes_data);
        }

        // Step 3: Per-layer stat transforms and scale feeding
        let mut layer_aes_map = HashMap::new();
        self.layers.iter().enumerate().for_each(|(i, layer)| {
            let transformed = layer.stat.transform(&aes_data);
            layer.geometry.update_scales(&mut self.scales, &transformed);
            layer_aes_map.insert(i, transformed);
        });

        // Step 4: Fit scales
        for scale in &mut self.scales {
            scale.fit().expect("Scale can't be fit")
        }

        // Step 5: Bulk mapping — build ResolvedData per layer
        let mut layer_resolved_map: HashMap<usize, ResolvedData> = HashMap::new();
        for (i, layer_aes) in &layer_aes_map {
            let mut resolved = ResolvedData {
                mapped: HashMap::new(),
                raw: HashMap::new(),
            };
            for aes in Aesthetic::all() {
                if let Some(col) = layer_aes.get(*aes) {
                    let family = aes.family();
                    if let Some(scale) = self.scales.iter().find(|s| s.aesthetic_family() == family) {
                        let mapped_col = scale.map(col)?;
                        resolved.mapped.insert(*aes, mapped_col);
                    } else {
                        // No scale for this aesthetic (e.g. Group) — keep raw
                        resolved.raw.insert(*aes, col.clone());
                    }
                }
            }
            layer_resolved_map.insert(*i, resolved);
        }

        let mut regions: HashMap<PlotRegion, Vec<Element>> = HashMap::new();

        // Step 6: Render geoms into DataArea
        self.layers.iter().enumerate().for_each(|(i, layer)| {
            let resolved = layer_resolved_map.get(&i).unwrap();
            let mut geom_elements = layer.geometry.render(resolved);
            regions
                .entry(PlotRegion::DataArea)
                .or_default()
                .append(&mut geom_elements);
        });

        // Render scales — each declares its own region
        for scale in &self.scales {
            let (region, mut scale_elements) = scale.render(self.theme);
            regions
                .entry(region)
                .or_default()
                .append(&mut scale_elements);
        }

        // Derive default axis labels from mapping column names
        let x_label = self.x_label.clone().or_else(|| {
            self.mappings
                .iter()
                .find(|m| m.aesthetic == Aesthetic::X)
                .map(|m| m.variable.clone())
        });
        let y_label = self.y_label.clone().or_else(|| {
            self.mappings
                .iter()
                .find(|m| m.aesthetic == Aesthetic::Y)
                .map(|m| m.variable.clone())
        });

        // Emit label elements into their respective regions
        if let Some(title) = &self.title {
            regions
                .entry(PlotRegion::Title)
                .or_default()
                .push(Element::Text(Text::centered(
                    title.clone(),
                    self.theme.title_font_size,
                    (Unit::Percent(50.0), Unit::Percent(50.0)),
                ).with_wrap()));
        }
        if let Some(label) = x_label {
            regions
                .entry(PlotRegion::XAxisGutter)
                .or_default()
                .push(Element::Text(Text::centered(
                    label,
                    self.theme.axis_label_font_size,
                    (Unit::Percent(50.0), Unit::NDC(-0.8)),
                ).with_wrap()));
        }
        if let Some(label) = y_label {
            regions
                .entry(PlotRegion::YAxisGutter)
                .or_default()
                .push(Element::Text(
                    Text::centered(
                        label,
                        self.theme.axis_label_font_size,
                        (Unit::NDC(-0.5), Unit::Percent(50.0)),
                    )
                    .with_v_align(VAlign::Center)
                    .with_rotation()
                    .with_wrap(),
                ));
        }
        if let Some(caption) = &self.caption {
            regions
                .entry(PlotRegion::Caption)
                .or_default()
                .push(Element::Text(Text::centered(
                    caption.clone(),
                    self.theme.caption_font_size,
                    (Unit::Percent(50.0), Unit::Percent(50.0)),
                ).with_wrap()));
        }

        let has_legend = self
            .scales
            .iter()
            .any(|s| s.aesthetic_family() == AestheticFamily::Color);
        let layout = standard_plot_layout(self.title.is_some(), self.caption.is_some(), has_legend, self.theme);

        Ok(PlotOutput { regions, layout })
    }
}

type Variable = String;

/// A Layer contains a Geometry, Mappings, a StatTransform, and a PositionAdjustment
///
/// Examples
///
/// Scatterplot
///   - Geom Point
///   - Mappings: {x, y}
///   - Stat Identity
///   - Position Identity

pub struct Layer {
    geometry: Box<dyn Geometry>,
    mappings: Vec<Mapping>,
    stat: Box<dyn StatTransform>,
    position: Box<dyn PositionAdjustment>,
}
impl Layer {
    pub fn new(
        geometry: Box<dyn Geometry>,
        mappings: Vec<Mapping>,
        stat: Box<dyn StatTransform>,
        position: Box<dyn PositionAdjustment>,
    ) -> Self {
        Self {
            geometry,
            mappings,
            stat,
            position,
        }
    }
}

/// A geometry converts transformed data into graphical elements.
///
/// For example:
/// - Point - a marker is drawn for every point
/// - Bar - a bar is drawn from 0 to Y for every point
/// - Line - a line is drawn through every point
///
/// A geometry supports a specific set of aesthetics.
///
/// A geometry has a default stat transform.
///
/// The coordinates of shapes returned is later projected onto a coordinate
/// system.
pub trait Geometry {
    /// These aesthetics are required to use this geometry.
    fn required_aesthetics(&self) -> Vec<Aesthetic>;

    /// These aesthetics are supported, but not required.
    ///
    /// By default, no extra aesthetics are supported
    fn extra_aesthetics(&self) -> Vec<Aesthetic> {
        vec![]
    }

    /// The default statistical transformation for this geometry type.
    fn default_stat(&self) -> &dyn StatTransform {
        &IdentityTransform {}
    }

    /// Renders shapes to be drawn on the screen using fully resolved (scale-mapped) data.
    fn render(&self, data: &ResolvedData) -> Vec<Element>;

    /// The list of aesthetic families that may be used in this layer
    fn aesthetic_families(&self) -> Vec<AestheticFamily> {
        self.required_aesthetics()
            .iter()
            .chain(self.extra_aesthetics().iter())
            .map(|a| a.family())
            .collect()
    }

    /// Update scales using the aesthetic-keyed raw data for this layer.
    fn update_scales(&self, scales: &mut Vec<Box<dyn Scale>>, data: &AesData) {
        let families: Vec<AestheticFamily> = self.aesthetic_families();

        let mut family_scale_map: HashMap<AestheticFamily, &mut Box<dyn Scale>> = scales
            .iter_mut()
            .filter_map(|s| {
                let family = s.aesthetic_family();
                if families.contains(&family) {
                    Some((family, s))
                } else {
                    None
                }
            })
            .collect();

        for aes in self
            .required_aesthetics()
            .iter()
            .chain(self.extra_aesthetics().iter())
        {
            if let Some(col) = data.get(*aes) {
                if let Some(scale) = family_scale_map.get_mut(&aes.family()) {
                    scale.append(col).expect("scale append failed");
                }
            }
        }
    }
}

/// GeomPoint renders a marker for every data point.
///
/// It is used to create the archetypal "Scatterplot".
///
/// Required aesthetics: `x`, `y`
///
/// Extra aesthetics: `color`
pub struct GeomPoint;
impl Geometry for GeomPoint {
    fn required_aesthetics(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn extra_aesthetics(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Color]
    }

    fn render(&self, data: &ResolvedData) -> Vec<Element> {
        let x_mapped = match data.mapped.get(&Aesthetic::X).expect("X was validated") {
            MappedColumn::UnitArray(v) => v,
            _ => panic!("expected UnitArray from position scale"),
        };
        let y_mapped = match data.mapped.get(&Aesthetic::Y).expect("Y was validated") {
            MappedColumn::UnitArray(v) => v,
            _ => panic!("expected UnitArray from position scale"),
        };
        let colors: Option<&Vec<[f32; 3]>> = data.mapped.get(&Aesthetic::Color).map(|c| match c {
            MappedColumn::ColorArray(v) => v,
            _ => panic!("expected ColorArray from color scale"),
        });

        let n = x_mapped.len();
        let mut points = Vec::with_capacity(n);
        for i in 0..n {
            let color = colors.map_or([0.0, 0.0, 0.0, 1.0], |c| {
                let [r, g, b] = c[i];
                [r, g, b, 1.0]
            });
            points.push(Element::Point(PointData {
                position: [x_mapped[i], y_mapped[i]],
                size: Unit::Pixels(16),
                color,
            }));
        }
        points
    }
}

/// GeomLine renders connected line segments through data points.
///
/// Required aesthetics: `x`, `y`
///
/// Extra aesthetics: `group`, `color`
///
/// When a `group` aesthetic is mapped, data is partitioned into separate
/// series and each series is rendered as an independent polyline.
pub struct GeomLine;
impl Geometry for GeomLine {
    fn required_aesthetics(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn extra_aesthetics(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Group, Aesthetic::Color]
    }

    fn render(&self, data: &ResolvedData) -> Vec<Element> {
        let x_mapped = match data.mapped.get(&Aesthetic::X).expect("X was validated") {
            MappedColumn::UnitArray(v) => v,
            _ => panic!("expected UnitArray from position scale"),
        };
        let y_mapped = match data.mapped.get(&Aesthetic::Y).expect("Y was validated") {
            MappedColumn::UnitArray(v) => v,
            _ => panic!("expected UnitArray from position scale"),
        };
        let colors: Option<&Vec<[f32; 3]>> = data.mapped.get(&Aesthetic::Color).map(|c| match c {
            MappedColumn::ColorArray(v) => v,
            _ => panic!("expected ColorArray from color scale"),
        });

        // Partition row indices by group value (or all rows = one group)
        let groups: Vec<Vec<usize>> =
            if let Some(RawColumn::StringArray(group_vals)) = data.raw.get(&Aesthetic::Group) {
                let mut group_map: Vec<(String, Vec<usize>)> = vec![];
                for (i, val) in group_vals.iter().enumerate() {
                    if let Some(entry) = group_map.iter_mut().find(|(k, _)| k == val) {
                        entry.1.push(i);
                    } else {
                        group_map.push((val.clone(), vec![i]));
                    }
                }
                group_map.into_iter().map(|(_, indices)| indices).collect()
            } else {
                vec![(0..x_mapped.len()).collect()]
            };

        let mut elements = vec![];
        for group_indices in &groups {
            if group_indices.len() < 2 {
                continue;
            }
            let points: Vec<[Unit; 2]> = group_indices
                .iter()
                .map(|&i| [x_mapped[i], y_mapped[i]])
                .collect();
            let point_colors: Vec<[f32; 4]> = group_indices
                .iter()
                .map(|&i| {
                    colors.map_or([0.0, 0.0, 0.0, 1.0], |c| {
                        let [r, g, b] = c[i];
                        [r, g, b, 1.0]
                    })
                })
                .collect();
            elements.push(Element::Polyline(PolylineData {
                points,
                thickness: 3.0,
                colors: point_colors,
            }));
        }
        elements
    }
}

/// Renders a bar for every data point
struct GeomBar;
// impl Geometry for GeomBar {}

/// A stat
pub trait StatTransform {
    /// Transform aesthetic-keyed data before plotting a geometry
    fn transform(&self, data: &AesData) -> AesData;
}
pub struct IdentityTransform;
impl StatTransform for IdentityTransform {
    fn transform(&self, data: &AesData) -> AesData {
        data.clone()
    }
}
impl PositionAdjustment for IdentityTransform {}

/// A position
pub trait PositionAdjustment {}

/// An aesthetic channel that maps data to a visual property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aesthetic {
    X,
    Y,
    Color,
    Group,
}

impl Aesthetic {
    pub fn all() -> &'static [Aesthetic] {
        &[
            Aesthetic::X,
            Aesthetic::Y,
            Aesthetic::Color,
            Aesthetic::Group,
        ]
    }

    pub fn family(&self) -> AestheticFamily {
        match self {
            Aesthetic::X => AestheticFamily::HorizontalPosition,
            Aesthetic::Y => AestheticFamily::VerticalPosition,
            Aesthetic::Color => AestheticFamily::Color,
            Aesthetic::Group => AestheticFamily::Group,
        }
    }

    pub fn default_scale(&self) -> Option<Box<dyn Scale>> {
        match self {
            Aesthetic::X => Some(Box::new(ScalePositionContinuous::new(Axis::X))),
            Aesthetic::Y => Some(Box::new(ScalePositionContinuous::new(Axis::Y))),
            Aesthetic::Color => Some(Box::new(ScaleColorDiscrete::new())),
            Aesthetic::Group => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Aesthetic::X => "x",
            Aesthetic::Y => "y",
            Aesthetic::Color => "color",
            Aesthetic::Group => "group",
        }
    }
}

/// Aesthetic families group related aesthetics that share a scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AestheticFamily {
    HorizontalPosition,
    VerticalPosition,
    Color,
    Group,
}

/// A mapping from a data variable to an aesthetic channel.
#[derive(Clone, Debug)]
pub struct Mapping {
    pub aesthetic: Aesthetic,
    pub variable: String,
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
}

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
}

impl ScalePositionContinuous {
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            data_scale: None,
        }
    }

    fn render_x_axis(&self, theme: &Theme) -> (PlotRegion, Vec<Element>) {
        let mut elements = vec![];

        // Axis line: full width at top of gutter (adjacent to DataArea)
        let xaxis = Rectangle::new(
            [Unit::NDC(0.0), Unit::NDC(1.0)],
            Unit::NDC(2.0),
            Unit::Pixels(1),
            [0.0, 0.0, 0.0, 1.0],
        );
        elements.push(Element::Rect(xaxis));

        let s = &self.data_scale.expect("Scale isn't fit");
        for tick_value in nice_ticks(s.min, s.max, 5) {
            let x_ndc = s.map_position(&NDC_SCALE, tick_value) as f32;

            // Tick mark hangs down from top edge
            let tick = Rectangle::new(
                [Unit::NDC(x_ndc), Unit::NDC(1.0)],
                Unit::Pixels(1),
                Unit::Pixels(6),
                [0.0, 0.0, 0.0, 1.0],
            );
            elements.push(Element::Rect(tick));

            let label = if tick_value.fract() == 0.0 {
                format!("{}", tick_value as i64)
            } else {
                format!("{:.1}", tick_value)
            };
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
            [0.0, 0.0, 0.0, 1.0],
        );
        elements.push(Element::Rect(yaxis));

        let s = &self.data_scale.expect("Scale isn't fit");
        for tick_value in nice_ticks(s.min, s.max, 5) {
            let y_ndc = s.map_position(&NDC_SCALE, tick_value) as f32;

            // Tick mark protrudes left from right edge
            let tick = Rectangle::new(
                [Unit::NDC(1.0), Unit::NDC(y_ndc)],
                Unit::Pixels(6),
                Unit::Pixels(1),
                [0.0, 0.0, 0.0, 1.0],
            );
            elements.push(Element::Rect(tick));

            let label = if tick_value.fract() == 0.0 {
                format!("{}", tick_value as i64)
            } else {
                format!("{:.1}", tick_value)
            };
            // Tick label vertically centered on tick mark
            elements.push(Element::Text(
                Text::centered(
                    label,
                    theme.tick_label_font_size,
                    (Unit::NDC(0.5), Unit::NDC(y_ndc)),
                )
                .with_v_align(VAlign::Center),
            ));
        }

        (PlotRegion::YAxisGutter, elements)
    }
}

impl Scale for ScalePositionContinuous {
    fn fit(&mut self) -> Result<(), String> {
        if let Some(s) = &self.data_scale {
            self.data_scale = Some(s.scale(1.1));
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
}

// should this be a trait?
enum CoordinateSystem {
    Cartesian,
}

/// Input data columns from CSV / stat transforms — before scale mapping.
#[derive(Clone, Debug)]
pub enum RawColumn {
    FloatArray(Vec<f64>),
    IntArray(Vec<i64>),
    StringArray(Vec<String>),
}

impl RawColumn {
    pub fn len(&self) -> usize {
        match self {
            Self::FloatArray(v) => v.len(),
            Self::IntArray(v) => v.len(),
            Self::StringArray(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Try to unpack values as a f64 vector
    pub fn as_f64(&self) -> Result<Vec<f64>, String> {
        match self {
            Self::FloatArray(v) => Ok(v.clone()),
            Self::IntArray(v) => Ok(v.iter().map(|i| *i as f64).collect()),
            Self::StringArray(_) => Err("Cannot convert StringArray to f64".into()),
        }
    }
}

/// Output of scale mapping — ready for geometry rendering.
#[derive(Clone, Debug)]
pub enum MappedColumn {
    UnitArray(Vec<Unit>),
    ColorArray(Vec<[f32; 3]>),
}

impl MappedColumn {
    pub fn len(&self) -> usize {
        match self {
            Self::UnitArray(v) => v.len(),
            Self::ColorArray(v) => v.len(),
        }
    }
}

/// Aesthetic-keyed raw data (after column renaming, before scale mapping).
#[derive(Clone)]
pub struct AesData {
    data: HashMap<Aesthetic, RawColumn>,
}

impl AesData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get(&self, aes: Aesthetic) -> Option<&RawColumn> {
        self.data.get(&aes)
    }

    pub fn insert(&mut self, aes: Aesthetic, col: RawColumn) {
        self.data.insert(aes, col);
    }

    pub fn contains(&self, aes: Aesthetic) -> bool {
        self.data.contains_key(&aes)
    }
}

/// Fully resolved data for geometry rendering.
pub struct ResolvedData {
    /// Scale-mapped aesthetics (X, Y, Color)
    pub mapped: HashMap<Aesthetic, MappedColumn>,
    /// Unscaled aesthetics (Group)
    pub raw: HashMap<Aesthetic, RawColumn>,
}

/// Raw column data keyed by column name — used at the CSV boundary and passed into Blueprint::render().
#[derive(Clone)]
pub struct PlotData {
    data: HashMap<String, RawColumn>,
}

impl PlotData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn insert(&mut self, key: String, value: RawColumn) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&RawColumn> {
        self.data.get(key)
    }
}

/// Build the standard single-plot layout tree.
///
/// ```text
/// Window (after margin)
/// +-- Vertical split (top to bottom)
///     +-- Pixels(50): Title         [if has_title]
///     +-- Flex(1.0): Horizontal split
///     |   +-- Pixels(80): YAxisGutter column
///     |   +-- Flex(1.0):  DataArea + XAxisGutter column
///     |   +-- Pixels(120): Legend column [if has_legend]
///     +-- Pixels(30): Caption        [if has_caption]
/// ```
fn standard_plot_layout(has_title: bool, has_caption: bool, has_legend: bool, theme: &Theme) -> LayoutNode {
    let data_column = LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: vec![
            (SizeSpec::Flex(1.0), LayoutNode::Leaf(PlotRegion::DataArea)),
            (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(PlotRegion::XAxisGutter)),
        ],
    };

    let y_axis_column = LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: vec![
            (SizeSpec::Flex(1.0), LayoutNode::Leaf(PlotRegion::YAxisGutter)),
            (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(PlotRegion::Spacer)),
        ],
    };

    let mut main_columns: Vec<(SizeSpec, LayoutNode)> = vec![
        (SizeSpec::Pixels(theme.y_gutter_width), y_axis_column),
        (SizeSpec::Flex(1.0), data_column),
    ];

    if has_legend {
        let legend_column = LayoutNode::Split {
            axis: SplitAxis::Vertical,
            children: vec![
                (SizeSpec::Flex(1.0), LayoutNode::Leaf(PlotRegion::Legend)),
                (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(PlotRegion::Spacer)),
            ],
        };
        main_columns.push((SizeSpec::Pixels(theme.legend_margin), LayoutNode::Leaf(PlotRegion::Spacer)));
        main_columns.push((SizeSpec::Pixels(theme.legend_width), legend_column));
    }

    let main = LayoutNode::Split {
        axis: SplitAxis::Horizontal,
        children: main_columns,
    };

    let mut rows: Vec<(SizeSpec, LayoutNode)> = vec![];
    if has_title {
        rows.push((SizeSpec::Pixels(theme.title_height), LayoutNode::Leaf(PlotRegion::Title)));
    }
    rows.push((SizeSpec::Flex(1.0), main));
    if has_caption {
        rows.push((SizeSpec::Pixels(theme.caption_height), LayoutNode::Leaf(PlotRegion::Caption)));
    }

    LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: rows,
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
        let theme = Theme::default();
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

    #[test]
    fn render_blueprint_with_color() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping {
                aesthetic: Aesthetic::X,
                variable: "x".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Y,
                variable: "y".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Color,
                variable: "species".into(),
            })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)))
            .with_scale(Box::new(ScaleColorDiscrete::new()));

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 3.0, 5.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![2.0, 4.0, 6.0]));
        data.insert(
            "species".into(),
            RawColumn::StringArray(vec!["a".into(), "b".into(), "a".into()]),
        );

        let output = bp.render(data).expect("render should succeed");
        // Should have: 3 points in DataArea + axis elements + legend elements
        let data_count = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(0, |v| v.len());
        assert!(data_count >= 3);
        assert!(output.regions.contains_key(&PlotRegion::Legend));
    }

    #[test]
    fn render_blueprint_without_color() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping {
                aesthetic: Aesthetic::X,
                variable: "x".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Y,
                variable: "y".into(),
            })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 3.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![2.0, 4.0]));

        let output = bp
            .render(data)
            .expect("render should succeed without color");
        let data_count = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
    }

    #[test]
    fn auto_map_from_column_names() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        // No explicit mappings or scales — just a geom
        let mut bp = Blueprint::new(&theme).with_layer(layer);

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![3.0, 4.0]));

        let output = bp.render(data).expect("auto-mapping should work");
        let data_count = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
    }

    #[test]
    fn auto_map_with_color() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme).with_layer(layer);

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![3.0, 4.0]));
        data.insert(
            "color".into(),
            RawColumn::StringArray(vec!["a".into(), "b".into()]),
        );

        let output = bp
            .render(data)
            .expect("auto-mapping with color should work");
        let data_count = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
        assert!(output.regions.contains_key(&PlotRegion::Legend));
    }

    #[test]
    fn auto_map_produces_axis_labels() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme).with_layer(layer);

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![3.0, 4.0]));

        let output = bp.render(data).unwrap();
        let all_text: Vec<String> = output
            .regions
            .values()
            .flat_map(|v| v.iter())
            .filter_map(|e| match e {
                Element::Text(t) => Some(t.value.clone()),
                _ => None,
            })
            .collect();
        assert!(
            all_text.contains(&"x".to_string()),
            "should have x axis label"
        );
        assert!(
            all_text.contains(&"y".to_string()),
            "should have y axis label"
        );
    }

    #[test]
    fn explicit_mapping_overrides_auto() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        // Explicitly map "year" → X, but data also has an "x" column
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping {
                aesthetic: Aesthetic::X,
                variable: "year".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Y,
                variable: "y".into(),
            })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0]));
        data.insert(
            "year".into(),
            RawColumn::FloatArray(vec![2020.0, 2021.0]),
        );
        data.insert("y".into(), RawColumn::FloatArray(vec![3.0, 4.0]));

        let output = bp
            .render(data)
            .expect("explicit mapping should take precedence");
        let data_count = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
    }

    #[test]
    fn geom_line_no_group_produces_segments() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomLine),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping {
                aesthetic: Aesthetic::X,
                variable: "x".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Y,
                variable: "y".into(),
            })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 4.0, 2.0]));

        let output = bp.render(data).expect("render should succeed");
        // 3 points → 2 line segments in DataArea
        let data_elements = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(&[][..], |v| v.as_slice());
        let polyline_count = data_elements
            .iter()
            .filter(|e| matches!(e, Element::Polyline(_)))
            .count();
        assert!(
            polyline_count >= 1,
            "expected at least 1 polyline, got {}",
            polyline_count
        );
    }

    #[test]
    fn geom_line_with_group_partitions() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomLine),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping {
                aesthetic: Aesthetic::X,
                variable: "x".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Y,
                variable: "y".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Group,
                variable: "grp".into(),
            })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));

        let mut data = PlotData::new();
        data.insert(
            "x".into(),
            RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]),
        );
        data.insert(
            "y".into(),
            RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]),
        );
        data.insert(
            "grp".into(),
            RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]),
        );

        let output = bp.render(data).expect("render should succeed");
        // 2 groups → 2 polylines in DataArea
        let data_elements = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(&[][..], |v| v.as_slice());
        let polyline_count = data_elements
            .iter()
            .filter(|e| matches!(e, Element::Polyline(_)))
            .count();
        assert!(polyline_count >= 2);
    }

    #[test]
    fn geom_line_single_point_group_no_panic() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomLine),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping {
                aesthetic: Aesthetic::X,
                variable: "x".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Y,
                variable: "y".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Group,
                variable: "grp".into(),
            })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0]));
        data.insert("grp".into(), RawColumn::StringArray(vec!["a".into()]));

        let output = bp.render(data).expect("single point should not panic");
        // Just verifying no panic; DataArea may be absent or empty
        let _ = output.regions.get(&PlotRegion::DataArea);
    }

    #[test]
    fn render_blueprint_geom_line_with_color() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomLine),
            vec![],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping {
                aesthetic: Aesthetic::X,
                variable: "x".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Y,
                variable: "y".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Group,
                variable: "grp".into(),
            })
            .with_mapping(Mapping {
                aesthetic: Aesthetic::Color,
                variable: "grp".into(),
            })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)))
            .with_scale(Box::new(ScaleColorDiscrete::new()));

        let mut data = PlotData::new();
        data.insert(
            "x".into(),
            RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]),
        );
        data.insert(
            "y".into(),
            RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]),
        );
        data.insert(
            "grp".into(),
            RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]),
        );

        let output = bp.render(data).expect("render with color should succeed");
        let data_count = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
        assert!(output.regions.contains_key(&PlotRegion::Legend));
    }

    #[test]
    fn create_blueprint() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![
                Mapping {
                    aesthetic: Aesthetic::X,
                    variable: "x".into(),
                },
                Mapping {
                    aesthetic: Aesthetic::Y,
                    variable: "y".into(),
                },
            ],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let _bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));
    }

    fn layout_test_segment() -> crate::layout::WindowSegment {
        crate::layout::WindowSegment::new(
            ContinuousNumericScale { min: -1., max: 1. },
            ContinuousNumericScale { min: -1., max: 1. },
            ContinuousNumericScale { min: 0., max: 800. },
            ContinuousNumericScale { min: 0., max: 600. },
        )
    }

    #[test]
    fn data_area_and_x_gutter_share_x_range() {
        let seg = layout_test_segment();
        let layout = standard_plot_layout(true, true, false, &Theme::default());
        let regions = layout.resolve(&seg);

        let data = regions.get(&PlotRegion::DataArea).unwrap();
        let xgutter = regions.get(&PlotRegion::XAxisGutter).unwrap();

        assert!((data.ndc_scale_x.min - xgutter.ndc_scale_x.min).abs() < 1e-5);
        assert!((data.ndc_scale_x.max - xgutter.ndc_scale_x.max).abs() < 1e-5);
        assert!((data.pixel_scale_x.min - xgutter.pixel_scale_x.min).abs() < 1e-5);
        assert!((data.pixel_scale_x.max - xgutter.pixel_scale_x.max).abs() < 1e-5);
    }

    #[test]
    fn data_area_and_y_gutter_share_y_range() {
        let seg = layout_test_segment();
        let layout = standard_plot_layout(true, true, false, &Theme::default());
        let regions = layout.resolve(&seg);

        let data = regions.get(&PlotRegion::DataArea).unwrap();
        let ygutter = regions.get(&PlotRegion::YAxisGutter).unwrap();

        assert!((data.ndc_scale_y.min - ygutter.ndc_scale_y.min).abs() < 1e-5);
        assert!((data.ndc_scale_y.max - ygutter.ndc_scale_y.max).abs() < 1e-5);
        assert!((data.pixel_scale_y.min - ygutter.pixel_scale_y.min).abs() < 1e-5);
        assert!((data.pixel_scale_y.max - ygutter.pixel_scale_y.max).abs() < 1e-5);
    }

    #[test]
    fn spacer_not_in_resolved_map() {
        let seg = layout_test_segment();
        let layout = standard_plot_layout(true, true, true, &Theme::default());
        let regions = layout.resolve(&seg);
        assert!(!regions.contains_key(&PlotRegion::Spacer));
    }

    #[test]
    fn layout_with_legend_has_all_regions() {
        let seg = layout_test_segment();
        let layout = standard_plot_layout(true, true, true, &Theme::default());
        let regions = layout.resolve(&seg);
        assert!(regions.contains_key(&PlotRegion::DataArea));
        assert!(regions.contains_key(&PlotRegion::XAxisGutter));
        assert!(regions.contains_key(&PlotRegion::YAxisGutter));
        assert!(regions.contains_key(&PlotRegion::Title));
        assert!(regions.contains_key(&PlotRegion::Legend));
        assert!(regions.contains_key(&PlotRegion::Caption));
    }

    #[test]
    fn layout_without_optional_regions() {
        let seg = layout_test_segment();
        let layout = standard_plot_layout(false, false, false, &Theme::default());
        let regions = layout.resolve(&seg);
        assert!(!regions.contains_key(&PlotRegion::Title));
        assert!(!regions.contains_key(&PlotRegion::Legend));
        assert!(!regions.contains_key(&PlotRegion::Caption));
        assert!(regions.contains_key(&PlotRegion::DataArea));
    }
}
