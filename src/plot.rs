use crate::shape::{Element, Rectangle, Text, Unit};
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
    pub fn render(&mut self, raw_data: PlotData) -> Result<Vec<Element>, String> {
        // Apply mappings: raw column names → aesthetic channel names
        let mut data = PlotData::new();
        for mapping in &self.mappings {
            let param = raw_data
                .get(&mapping.variable)
                .ok_or_else(|| format!("Column '{}' not found in data", mapping.variable))?;
            data.insert(mapping.aesthetic.name().to_string(), param.clone());
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

        let mut shapes: Vec<Element> = vec![];
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
        // Render geoms, appending to shapes vecs
        self.layers.iter().enumerate().for_each(|(i, layer)| {
            let layer_data = layer_data_map.get(&i).unwrap();
            shapes.append(&mut layer.geometry.render(layer_data, &self.scales));
        });

        //Render scales
        for scale in &self.scales {
            let mut scale_shapes = scale.render();
            shapes.append(&mut scale_shapes);
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

        // Emit label elements
        if let Some(title) = &self.title {
            shapes.push(Element::Text(Text::centered(
                title.clone(),
                32.0,
                (Unit::NDC(0.0), Unit::NDC(1.2)),
            )));
        }
        if let Some(label) = x_label {
            shapes.push(Element::Text(Text::centered(
                label,
                24.0,
                (Unit::NDC(0.0), Unit::NDC(-1.2)),
            )));
        }
        if let Some(label) = y_label {
            shapes.push(Element::Text(Text::new(
                label,
                24.0,
                (Unit::NDC(-1.3), Unit::NDC(0.0)),
            )));
        }
        if let Some(caption) = &self.caption {
            shapes.push(Element::Text(Text::centered(
                caption.clone(),
                20.0,
                (Unit::NDC(0.0), Unit::NDC(-1.35)),
            )));
        }

        // TODO: Project shapes onto coordinate system
        // TODO: Project position scales onto coordinate system
        // TODO: Assign window segments to subplots
        // TODO: Window segment transforms
        Ok(shapes)
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
            let scale = family_scale_map
                .get_mut(&aes.family())
                .expect("scale exists in map");
            scale.append(values).expect("scale append failed");
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

    fn render(&self, data: &PlotData, scales: &Vec<Box<dyn Scale>>) -> Vec<Element> {
        let mut points = vec![];
        let x_scale = scales
            .iter()
            .find(|s| s.aesthetic_family() == AestheticFamily::HorizontalPosition)
            .unwrap();
        let x = data
            .get("x")
            .expect("key existence was already validated");
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

        for i in 0..x.len() {
            let r = Rectangle::new(
                [x_mapped[i], y_mapped[i]],
                Unit::Pixels(16),
                Unit::Pixels(16),
                [0.0, 0.0, 0.0],
            );
            points.push(Element::Shape(Box::new(r)));
        }
        points
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
    // Future: Color, Shape, Size
}

impl Aesthetic {
    pub fn family(&self) -> AestheticFamily {
        match self {
            Aesthetic::X => AestheticFamily::HorizontalPosition,
            Aesthetic::Y => AestheticFamily::VerticalPosition,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Aesthetic::X => "x",
            Aesthetic::Y => "y",
        }
    }
}

/// Aesthetic families group related aesthetics that share a scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AestheticFamily {
    HorizontalPosition,
    VerticalPosition,
    // Future: Color
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

    /// Render the legend for this scale.
    fn render(&self) -> Vec<Element>;

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

    fn render_x_axis(&self) -> Vec<Element> {
        let mut elements = vec![];

        let xaxis = Rectangle::new(
            [
                Unit::NDC(NDC_SCALE.midpoint() as f32),
                Unit::NDC(NDC_SCALE.min as f32),
            ],
            Unit::NDC(NDC_SCALE.span() as f32),
            Unit::Pixels(1),
            [0.0, 0.0, 0.0],
        );
        elements.push(Element::Shape(Box::new(xaxis)));

        let s = &self.data_scale.expect("Scale isn't fit");
        for tick_value in nice_ticks(s.min, s.max, 5) {
            let x_ndc = s.map_position(&NDC_SCALE, tick_value) as f32;

            let tick = Rectangle::new(
                [Unit::NDC(x_ndc), Unit::NDC(NDC_SCALE.min as f32)],
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
            elements.push(Element::Text(Text::new(
                label,
                24.0,
                (Unit::NDC(x_ndc), Unit::NDC(-1.08)),
            )));
        }

        elements
    }

    fn render_y_axis(&self) -> Vec<Element> {
        let mut elements: Vec<Element> = vec![];

        let yaxis = Rectangle::new(
            [
                Unit::NDC(NDC_SCALE.min as f32),
                Unit::NDC(NDC_SCALE.midpoint() as f32),
            ],
            Unit::Pixels(1),
            Unit::NDC(NDC_SCALE.span() as f32),
            [0.0, 0.0, 0.0],
        );
        elements.push(Element::Shape(Box::new(yaxis)));

        let s = &self.data_scale.expect("Scale isn't fit");
        for tick_value in nice_ticks(s.min, s.max, 5) {
            let y_ndc = s.map_position(&NDC_SCALE, tick_value) as f32;

            let tick = Rectangle::new(
                [Unit::NDC(NDC_SCALE.min as f32), Unit::NDC(y_ndc)],
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
            elements.push(Element::Text(Text::new(
                label,
                24.0,
                (Unit::NDC(-1.08), Unit::NDC(y_ndc)),
            )));
        }

        elements
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

    fn render(&self) -> Vec<Element> {
        match self.axis {
            Axis::X => self.render_x_axis(),
            Axis::Y => self.render_y_axis(),
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

// should this be a trait?
enum CoordinateSystem {
    Cartesian,
}

pub struct Theme {
    pub window_margin: Unit,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            window_margin: Unit::Percent(25.),
        }
    }
}

/// Array data types that are either provided to a plot, or produced via a
/// transformation.
#[derive(Clone)]
pub enum PlotParameter {
    FloatArray(Vec<f64>),
    IntArray(Vec<i64>),

    // UnitArray represents post-transform position values
    UnitArray(Vec<Unit>),
}
impl PlotParameter {
    pub fn len(&self) -> usize {
        match self {
            Self::FloatArray(v) => v.len(),
            Self::IntArray(v) => v.len(),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_blueprint() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![
                Mapping { aesthetic: Aesthetic::X, variable: "x".into() },
                Mapping { aesthetic: Aesthetic::Y, variable: "y".into() },
            ],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let _bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));
    }
}
