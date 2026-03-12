use crate::layout::{LayoutNode, PlotOutput, PlotRegion, SizeSpec, SplitAxis, Unit};
use crate::shape::{Element, LineSegment, Rectangle, Text, VAlign};
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
        // Apply explicit mappings: raw column names → aesthetic channel names
        let mut data = PlotData::new();
        let mut mapped_aesthetics: Vec<Aesthetic> = vec![];
        for mapping in &self.mappings {
            let param = raw_data
                .get(&mapping.variable)
                .ok_or_else(|| format!("Column '{}' not found in data", mapping.variable))?;
            data.insert(mapping.aesthetic.name().to_string(), param.clone());
            mapped_aesthetics.push(mapping.aesthetic);
        }

        // Auto-map: if a data column name matches an aesthetic name and that
        // aesthetic isn't already explicitly mapped, use it as the default.
        for aes in Aesthetic::all() {
            if mapped_aesthetics.contains(aes) {
                continue;
            }
            if let Some(param) = raw_data.get(aes.name()) {
                data.insert(aes.name().to_string(), param.clone());
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
                if !data.contains(aes.name()) {
                    return Err(format!("Missing required aesthetic {}", aes.name()));
                }
            }
        }

        // Scale transforms
        for scale in &self.scales {
            data = scale.transform(data);
        }

        // TODO: Apply facet transforms at this stage, grouping elements by facet value.

        let mut layer_data_map = std::collections::HashMap::new();
        self.layers.iter().enumerate().for_each(|(i, layer)| {
            // Copy data, then run stat transforms
            let mut layer_data = data.clone();
            layer_data = layer.stat.transform(&layer_data);
            // Append to scales
            layer.geometry.update_scales(&mut self.scales, &layer_data);
            layer_data_map.insert(i, layer_data);
        });
        // fit scales
        for scale in &mut self.scales {
            scale.fit().expect("Scale can't be fit")
        }

        let mut regions: HashMap<PlotRegion, Vec<Element>> = HashMap::new();

        // Render geoms into DataArea
        self.layers.iter().enumerate().for_each(|(i, layer)| {
            let layer_data = layer_data_map.get(&i).unwrap();
            let mut geom_elements = layer.geometry.render(layer_data, &self.scales);
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

    /// Renders shapes to be drawn on the screen.
    ///
    /// Coordinates of the shapes are in data-space. These are later projected
    /// onto a coordinate system and translated into screen-space.
    fn render(&self, data: &PlotData, scales: &Vec<Box<dyn Scale>>) -> Vec<Element>;

    /// The list of aesthetic families that may be used in this layer
    fn aesthetic_families(&self) -> Vec<AestheticFamily> {
        self.required_aesthetics()
            .iter()
            .chain(self.extra_aesthetics().iter())
            .map(|a| a.family())
            .collect()
    }

    /// Filter down the plot data to only the aesthetics used in this geometry, and convert to MappedData
    fn mapped_data(&self, data: &PlotData) -> MappedData {
        let mut mapped_data: Vec<(Aesthetic, PlotParameter)> = vec![];
        for aes in self
            .required_aesthetics()
            .iter()
            .chain(self.extra_aesthetics().iter())
        {
            if let Some(param) = data.get(aes.name()) {
                mapped_data.push((*aes, param.clone()));
            }
        }
        MappedData { data: mapped_data }
    }

    /// Update scales using the data in this plot
    fn update_scales(&self, scales: &mut Vec<Box<dyn Scale>>, data: &PlotData) {
        let mapped_data = self.mapped_data(&data);
        let families: Vec<AestheticFamily> = self.aesthetic_families();

        // build a map of family to scale for the scales used in this plot
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

        // update the scale values for the scales used in this plot each
        // mapping's data is routed to specific scales. e.g. x axis goes to
        // horizontal position scale
        for (aes, values) in mapped_data.data.iter() {
            if let Some(scale) = family_scale_map.get_mut(&aes.family()) {
                scale.append(values).expect("scale append failed");
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
/// Extra aeshetics: none
pub struct GeomPoint;
impl Geometry for GeomPoint {
    fn required_aesthetics(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn extra_aesthetics(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Color]
    }

    fn render(&self, data: &PlotData, scales: &Vec<Box<dyn Scale>>) -> Vec<Element> {
        let mut points = vec![];
        let x_scale = scales
            .iter()
            .find(|s| s.aesthetic_family() == AestheticFamily::HorizontalPosition)
            .unwrap();
        let x = data.get("x").expect("key existence was already validated");
        let x_mapped = match x_scale.map(x).expect("scales were fit to data") {
            PlotParameter::UnitArray(v) => v,
            _ => panic!("expected unit array from position scale"),
        };

        let y_scale = scales
            .iter()
            .find(|s| s.aesthetic_family() == AestheticFamily::VerticalPosition)
            .unwrap();
        let y = data.get("y").expect("already validated");
        let y_mapped = match y_scale.map(y).expect("scales were fit to data") {
            PlotParameter::UnitArray(v) => v,
            _ => panic!("expected unit array from position scale"),
        };

        // Resolve per-point colors if a color aesthetic is mapped
        let colors: Option<Vec<[f32; 3]>> = data.get("color").map(|color_data| {
            let color_scale = scales
                .iter()
                .find(|s| s.aesthetic_family() == AestheticFamily::Color)
                .expect("color scale must exist when color aesthetic is mapped");
            match color_scale.map(color_data).expect("color scale was fit") {
                PlotParameter::ColorArray(v) => v,
                _ => panic!("expected color array from color scale"),
            }
        });

        for i in 0..x.len() {
            let color = colors.as_ref().map_or([0.0, 0.0, 0.0], |c| c[i]);
            let r = Rectangle::new(
                [x_mapped[i], y_mapped[i]],
                Unit::Pixels(16),
                Unit::Pixels(16),
                color,
            );
            points.push(Element::Shape(Box::new(r)));
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

    fn render(&self, data: &PlotData, scales: &Vec<Box<dyn Scale>>) -> Vec<Element> {
        let x_scale = scales
            .iter()
            .find(|s| s.aesthetic_family() == AestheticFamily::HorizontalPosition)
            .unwrap();
        let x = data.get("x").expect("key existence was already validated");
        let x_mapped = match x_scale.map(x).expect("scales were fit to data") {
            PlotParameter::UnitArray(v) => v,
            _ => panic!("expected unit array from position scale"),
        };

        let y_scale = scales
            .iter()
            .find(|s| s.aesthetic_family() == AestheticFamily::VerticalPosition)
            .unwrap();
        let y = data.get("y").expect("already validated");
        let y_mapped = match y_scale.map(y).expect("scales were fit to data") {
            PlotParameter::UnitArray(v) => v,
            _ => panic!("expected unit array from position scale"),
        };

        // Resolve per-point colors if a color aesthetic is mapped
        let colors: Option<Vec<[f32; 3]>> = data.get("color").map(|color_data| {
            let color_scale = scales
                .iter()
                .find(|s| s.aesthetic_family() == AestheticFamily::Color)
                .expect("color scale must exist when color aesthetic is mapped");
            match color_scale.map(color_data).expect("color scale was fit") {
                PlotParameter::ColorArray(v) => v,
                _ => panic!("expected color array from color scale"),
            }
        });

        // Partition row indices by group value (or all rows = one group)
        let groups: Vec<Vec<usize>> =
            if let Some(PlotParameter::StringArray(group_vals)) = data.get("group") {
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
            for pair in group_indices.windows(2) {
                let i = pair[0];
                let j = pair[1];
                let color = colors.as_ref().map_or([0.0, 0.0, 0.0], |c| c[i]);
                let seg = LineSegment::new(
                    [x_mapped[i], y_mapped[i]],
                    [x_mapped[j], y_mapped[j]],
                    2.0,
                    color,
                );
                elements.push(Element::Shape(Box::new(seg)));
            }
        }
        elements
    }
}

/// Renders a bar for every data point
struct GeomBar;
// impl Geometry for GeomBar {}

/// A stat
pub trait StatTransform {
    /// Transform data before plotting a geometry
    fn transform(&self, data: &PlotData) -> PlotData;
}
pub struct IdentityTransform;
impl StatTransform for IdentityTransform {
    fn transform(&self, data: &PlotData) -> PlotData {
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
    /// Transform plot data for this scale.
    ///
    /// By default, no transformations are applied
    fn transform(&self, data: PlotData) -> PlotData {
        data
    }

    /// Map an array of data values to the scale, returning an array of
    /// transformed values, possibly of a different type
    fn map(&self, v: &PlotParameter) -> Result<PlotParameter, String>;

    /// Append a set of data values to the scale
    fn append(&mut self, v: &PlotParameter) -> Result<(), String>;

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
            [0.0, 0.0, 0.0],
        );
        elements.push(Element::Shape(Box::new(xaxis)));

        let s = &self.data_scale.expect("Scale isn't fit");
        for tick_value in nice_ticks(s.min, s.max, 5) {
            let x_ndc = s.map_position(&NDC_SCALE, tick_value) as f32;

            // Tick mark hangs down from top edge
            let tick = Rectangle::new(
                [Unit::NDC(x_ndc), Unit::NDC(1.0)],
                Unit::Pixels(1),
                Unit::Pixels(6),
                [0.0, 0.0, 0.0],
            );
            elements.push(Element::Shape(Box::new(tick)));

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
            [0.0, 0.0, 0.0],
        );
        elements.push(Element::Shape(Box::new(yaxis)));

        let s = &self.data_scale.expect("Scale isn't fit");
        for tick_value in nice_ticks(s.min, s.max, 5) {
            let y_ndc = s.map_position(&NDC_SCALE, tick_value) as f32;

            // Tick mark protrudes left from right edge
            let tick = Rectangle::new(
                [Unit::NDC(1.0), Unit::NDC(y_ndc)],
                Unit::Pixels(6),
                Unit::Pixels(1),
                [0.0, 0.0, 0.0],
            );
            elements.push(Element::Shape(Box::new(tick)));

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

    fn map(&self, v: &PlotParameter) -> Result<PlotParameter, String> {
        let values = v.as_f64()?;

        if let Some(s) = &self.data_scale {
            Ok(PlotParameter::UnitArray(
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

    fn append(&mut self, v: &PlotParameter) -> Result<(), String> {
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
    fn append(&mut self, v: &PlotParameter) -> Result<(), String> {
        match v {
            PlotParameter::StringArray(strings) => {
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

    fn map(&self, v: &PlotParameter) -> Result<PlotParameter, String> {
        match v {
            PlotParameter::StringArray(strings) => {
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
                Ok(PlotParameter::ColorArray(colors))
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
            let swatch = Rectangle::new(
                [Unit::Percent(10.0), Unit::NDC(y)],
                Unit::Pixels(14),
                Unit::Pixels(14),
                self.palette[i],
            );
            elements.push(Element::Shape(Box::new(swatch)));
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

/// Array data types that are either provided to a plot, or produced via a
/// transformation.
#[derive(Clone)]
pub enum PlotParameter {
    FloatArray(Vec<f64>),
    IntArray(Vec<i64>),
    StringArray(Vec<String>),
    ColorArray(Vec<[f32; 3]>),

    // UnitArray represents post-transform position values
    UnitArray(Vec<Unit>),
}
impl PlotParameter {
    pub fn len(&self) -> usize {
        match self {
            Self::FloatArray(v) => v.len(),
            Self::IntArray(v) => v.len(),
            Self::StringArray(v) => v.len(),
            Self::ColorArray(v) => v.len(),
            Self::UnitArray(v) => v.len(),
        }
    }

    /// Try to unpack values as a f64 vector
    pub fn as_f64(&self) -> Result<Vec<f64>, String> {
        match self {
            Self::FloatArray(v) => Ok(v.clone()),
            Self::IntArray(v) => Ok(v.iter().map(|i| *i as f64).collect()),
            Self::UnitArray(v) => Ok(v
                .iter()
                .map(|u| match u {
                    Unit::NDC(v) => *v as f64,
                    Unit::Pixels(v) => *v as f64,
                    Unit::Percent(v) => *v as f64,
                })
                .collect()),
            Self::StringArray(_) => Err("Cannot convert StringArray to f64".into()),
            Self::ColorArray(_) => Err("Cannot convert ColorArray to f64".into()),
        }
    }
}

/// A structure to store the data for a plot
#[derive(Clone)]
pub struct PlotData {
    data: HashMap<String, PlotParameter>,
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

    pub fn insert(&mut self, key: String, value: PlotParameter) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&PlotParameter> {
        self.data.get(key)
    }
}

/// For a layer, MappedData is parsed to specific aesthetics for a plot
pub struct MappedData {
    data: Vec<(Aesthetic, PlotParameter)>,
}
impl MappedData {
    fn aesthetics(&self) -> Vec<Aesthetic> {
        self.data.iter().map(|(aes, _)| *aes).collect()
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
        let input =
            PlotParameter::StringArray(vec!["a".into(), "b".into(), "a".into(), "c".into()]);
        scale.append(&input).unwrap();
        scale.fit().unwrap();

        let mapped = scale.map(&input).unwrap();
        match mapped {
            PlotParameter::ColorArray(colors) => {
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
            PlotParameter::StringArray(vec!["banana".into(), "apple".into(), "banana".into()]);
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
        let input = PlotParameter::FloatArray(vec![1.0, 2.0]);
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
        data.insert("x".into(), PlotParameter::FloatArray(vec![1.0, 3.0, 5.0]));
        data.insert("y".into(), PlotParameter::FloatArray(vec![2.0, 4.0, 6.0]));
        data.insert(
            "species".into(),
            PlotParameter::StringArray(vec!["a".into(), "b".into(), "a".into()]),
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
        data.insert("x".into(), PlotParameter::FloatArray(vec![1.0, 3.0]));
        data.insert("y".into(), PlotParameter::FloatArray(vec![2.0, 4.0]));

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
        data.insert("x".into(), PlotParameter::FloatArray(vec![1.0, 2.0]));
        data.insert("y".into(), PlotParameter::FloatArray(vec![3.0, 4.0]));

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
        data.insert("x".into(), PlotParameter::FloatArray(vec![1.0, 2.0]));
        data.insert("y".into(), PlotParameter::FloatArray(vec![3.0, 4.0]));
        data.insert(
            "color".into(),
            PlotParameter::StringArray(vec!["a".into(), "b".into()]),
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
        data.insert("x".into(), PlotParameter::FloatArray(vec![1.0, 2.0]));
        data.insert("y".into(), PlotParameter::FloatArray(vec![3.0, 4.0]));

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
        data.insert("x".into(), PlotParameter::FloatArray(vec![1.0, 2.0]));
        data.insert(
            "year".into(),
            PlotParameter::FloatArray(vec![2020.0, 2021.0]),
        );
        data.insert("y".into(), PlotParameter::FloatArray(vec![3.0, 4.0]));

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
        data.insert("x".into(), PlotParameter::FloatArray(vec![1.0, 2.0, 3.0]));
        data.insert("y".into(), PlotParameter::FloatArray(vec![1.0, 4.0, 2.0]));

        let output = bp.render(data).expect("render should succeed");
        // 3 points → 2 line segments in DataArea
        let data_elements = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(&[][..], |v| v.as_slice());
        let shape_count = data_elements
            .iter()
            .filter(|e| matches!(e, Element::Shape(_)))
            .count();
        assert!(
            shape_count >= 2,
            "expected at least 2 line segments, got {}",
            shape_count
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
            PlotParameter::FloatArray(vec![1.0, 2.0, 3.0, 4.0]),
        );
        data.insert(
            "y".into(),
            PlotParameter::FloatArray(vec![1.0, 2.0, 3.0, 4.0]),
        );
        data.insert(
            "grp".into(),
            PlotParameter::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]),
        );

        let output = bp.render(data).expect("render should succeed");
        // 2 groups × 1 segment each = 2 line segments in DataArea
        let data_elements = output
            .regions
            .get(&PlotRegion::DataArea)
            .map_or(&[][..], |v| v.as_slice());
        let shape_count = data_elements
            .iter()
            .filter(|e| matches!(e, Element::Shape(_)))
            .count();
        assert!(shape_count >= 2);
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
        data.insert("x".into(), PlotParameter::FloatArray(vec![1.0]));
        data.insert("y".into(), PlotParameter::FloatArray(vec![1.0]));
        data.insert("grp".into(), PlotParameter::StringArray(vec!["a".into()]));

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
            PlotParameter::FloatArray(vec![1.0, 2.0, 3.0, 4.0]),
        );
        data.insert(
            "y".into(),
            PlotParameter::FloatArray(vec![1.0, 2.0, 3.0, 4.0]),
        );
        data.insert(
            "grp".into(),
            PlotParameter::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]),
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
