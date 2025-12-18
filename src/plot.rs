use crate::shape::{Rectangle, Shape, Unit, WindowSegment};
use crate::transform::{ContinuousNumericScale, NDC_SCALE};
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

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

    /// Render a plot from this blueprint
    pub fn render(&mut self, mut data: PlotData) -> Result<Vec<Box<dyn Shape>>, String> {
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

        let mut shapes: Vec<Box<dyn Shape>> = vec![];
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

        // Project shapes onto coordinate system
        // Render scales
        // Project position scales onto coordinate system
        // Assign window segments to subplots
        // Window segment transforms
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
    fn required_aesthetics(&self) -> Vec<Rc<dyn Aesthetic>>;

    /// These aesthetics are supported, but not required.
    ///
    /// By default, no extra aesthetics are supported
    fn extra_aesthetics(&self) -> Vec<Rc<dyn Aesthetic>> {
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
    fn render(&self, data: &PlotData, sclaes: &Vec<Box<dyn Scale>>) -> Vec<Box<dyn Shape>>;

    /// The list of aesthetic families that may be used in this layer
    fn aesthetic_families(&self) -> Vec<Box<dyn AestheticFamily>> {
        self.required_aesthetics()
            .iter()
            .chain(self.extra_aesthetics().iter())
            .map(|a| a.aesthetic_family())
            .collect()
    }

    /// Filter down the plot data to only the aesthetics used in this geometry, and convert to MappedData
    fn mapped_data(&self, data: &PlotData) -> MappedData {
        let mut mapped_data: Vec<(Rc<dyn Aesthetic>, PlotParameter)> = vec![];
        for aes in self
            .required_aesthetics()
            .iter()
            .chain(self.extra_aesthetics().iter())
        {
            if let Some(param) = data.data.get(aes.name()) {
                mapped_data.push((aes.clone(), param.clone()));
            }
        }
        MappedData { data: mapped_data }
    }

    /// Update scales using the data in this plot
    fn update_scales(&self, scales: &mut Vec<Box<dyn Scale>>, data: &PlotData) {
        let mapped_data = self.mapped_data(&data);
        let families: Vec<String> = self
            .aesthetic_families()
            .iter()
            .map(|f| f.name().to_string())
            // .cloned()
            .collect();

        // build a map of family name to scale for the scales used in this plot
        let mut family_scale_map: HashMap<String, &mut Box<dyn Scale>> = scales
            .iter_mut()
            .filter_map(|s| {
                let name = s.aesthetic_family().name().to_string();
                if families.contains(&name) {
                    Some((name, s))
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
                .get_mut(aes.aesthetic_family().name())
                .expect("scale exists in map");
            scale.append(values);
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
    fn required_aesthetics(&self) -> Vec<Rc<dyn Aesthetic>> {
        vec![Rc::new(AesX {}), Rc::new(AesY {})]
    }

    fn render(&self, data: &PlotData, scales: &Vec<Box<dyn Scale>>) -> Vec<Box<dyn Shape>> {
        let mut rectangles: Vec<Box<dyn Shape>> = vec![];
        let x_scale = scales
            .iter()
            .find(|s| s.aesthetic_family().name() == "HorizontalPosition")
            .unwrap();
        let x = data
            .data
            .get("x")
            .expect("key existance was already validated");
        let x_mapped = match x_scale.map(x).expect("scales were fit to data") {
            PlotParameter::UnitArray(v) => v,
            _ => panic!("expected unit array from position scale"),
        };

        let y_scale = scales
            .iter()
            .find(|s| s.aesthetic_family().name() == "VerticalPosition")
            .unwrap();
        let y = data.data.get("y").expect("already validated");
        let y_mapped = match y_scale.map(y).expect("scales were fit to data") {
            PlotParameter::UnitArray(v) => v,
            _ => panic!("expected unit array from position scale"),
        };

        for i in 0..x.len() {
            rectangles.push(Box::new(Rectangle::new(
                [x_mapped[i], y_mapped[i]],
                Unit::Pixels(16),
                Unit::Pixels(16),
                [0.0, 0.0, 0.0],
            )));
        }
        rectangles
    }

    fn aesthetic_families(&self) -> Vec<Box<dyn AestheticFamily>> {
        vec![Box::new(FamHPosition {}), Box::new(FamVPosition {})]
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

// Stores the mapping of a visual channel to a column
pub enum Mapping {
    X(String),
    Y(String),
}

/// The Aesthetic trait is used to define an aesthetic.
///
/// Examples:
/// - x / y
/// - color
/// - xmin/xmax
/// - width/height
/// - shape
/// - linewidth
/// - linetype
///
/// Each aesthetic must declare the AestheticFamily it belongs to.
trait Aesthetic {
    fn name(&self) -> &str;
    fn aesthetic_family(&self) -> Box<dyn AestheticFamily>;
}

/// The X Aesthetic defines an elements horizontal position.
struct AesX {}
impl Aesthetic for AesX {
    fn name(&self) -> &str {
        "x"
    }
    fn aesthetic_family(&self) -> Box<dyn AestheticFamily> {
        Box::new(FamHPosition)
    }
}
/// The Y Aesthetic defines an elements vertical position.
struct AesY {}
impl Aesthetic for AesY {
    fn name(&self) -> &str {
        "y"
    }
    fn aesthetic_family(&self) -> Box<dyn AestheticFamily> {
        Box::new(FamVPosition)
    }
}

trait AestheticFamily {
    fn name(&self) -> &str;
}

/// A family for scales and aesthetics that use horizontal position (x axis)
#[derive(PartialEq)]
struct FamHPosition;
impl AestheticFamily for FamHPosition {
    fn name(&self) -> &str {
        "HorizontalPosition"
    }
}

/// A family for scales and aesthetics that use vertical position (y axis)
#[derive(PartialEq)]
struct FamVPosition;
impl AestheticFamily for FamVPosition {
    fn name(&self) -> &str {
        "VerticalPosition"
    }
}

#[derive(PartialEq)]
struct FamColor;
impl AestheticFamily for FamColor {
    fn name(&self) -> &str {
        "Color"
    }
}

/// Scales produce legends.
/// They are used to convert between the projection on the screen and the data.
///
/// For example, a continuous numeric scale maps length on the screen to
/// the mapped variable. A discrete color scale maps color to a category.
pub trait Scale: Any {
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
    fn render(&self) -> Vec<Box<dyn Shape>>;

    /// Return the family this scale belongs to.
    ///
    /// Aesthetics are only compatible with scales in a single
    /// scale family, and only one scale from a family can be
    /// used in each plot.
    fn aesthetic_family(&self) -> Box<dyn AestheticFamily>;
}

/// ScaleXContinuous is a positional scale.
///
/// It maps data points to horizontal positions over a portion of the screen.
pub struct ScaleXContinuous {
    /// The scale of the xaxis in data units
    data_scale: Option<ContinuousNumericScale>,
}
impl ScaleXContinuous {
    /// Create a new scale, mapping to a specific region of the screen
    pub fn new() -> Self {
        Self { data_scale: None }
    }
}
impl Scale for ScaleXContinuous {
    /// Extend scale by 10% to add a margin between data points and plot boundaries
    fn fit(&mut self) -> Result<(), String> {
        if let Some(s) = &self.data_scale {
            self.data_scale = Some(s.scale(1.1));
        }
        Ok(())
    }

    /// Translate data values into relative ndc values for rendering position on screen
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

    /// Render x axis
    fn render(&self) -> Vec<Box<dyn Shape>> {
        let mut shapes: Vec<Box<dyn Shape>> = vec![];

        // draw primary line the full width of the allocated space
        let xaxis = Rectangle::new(
            // place the center of the axis in the center of our window segment
            [
                Unit::NDC(NDC_SCALE.midpoint() as f32),
                Unit::NDC(NDC_SCALE.midpoint() as f32),
            ],
            Unit::NDC(NDC_SCALE.span() as f32),
            Unit::Pixels(1), // fixed 1px line width
            [0.0, 0.0, 0.0], // black
        );
        shapes.push(Box::new(xaxis) as Box<dyn Shape>);

        // Todo: add tickmarks and labels
        shapes
    }

    /// The aesthetic family fo the scale
    fn aesthetic_family(&self) -> Box<dyn AestheticFamily> {
        Box::new(FamHPosition)
    }

    /// Append a set of values to the scale.
    ///
    /// Expands the min and max values of the scale if they don't
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

/// ScaleYContinuous is a positional scale.
///
/// It maps data points to horizontal positions over a portion of the screen.
pub struct ScaleYContinuous {
    /// The scale of the xaxis in data units
    data_scale: Option<ContinuousNumericScale>,
}
impl ScaleYContinuous {
    /// Create a new scale, mapping to a specific region of the screen
    pub fn new() -> Self {
        Self { data_scale: None }
    }
}
impl Scale for ScaleYContinuous {
    /// Extend scale by 10% to add a margin between data points and plot boundaries
    fn fit(&mut self) -> Result<(), String> {
        if let Some(s) = &self.data_scale {
            self.data_scale = Some(s.scale(1.1));
        }
        Ok(())
    }

    /// Translate data values into relative ndc values for rendering position on screen
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

    /// Render y axis
    fn render(&self) -> Vec<Box<dyn Shape>> {
        let mut shapes: Vec<Box<dyn Shape>> = vec![];

        // draw primary line the full width of the allocated space
        let xaxis = Rectangle::new(
            // place the center of the axis in the center of our window segment
            [
                Unit::NDC(NDC_SCALE.midpoint() as f32),
                Unit::NDC(NDC_SCALE.midpoint() as f32),
            ],
            Unit::Pixels(1), // fixed 1px line width
            Unit::NDC(NDC_SCALE.span() as f32),
            [0.0, 0.0, 0.0], // black
        );
        shapes.push(Box::new(xaxis) as Box<dyn Shape>);

        // Todo: add tickmarks and labels
        shapes
    }

    /// The aesthetic family fo the scale
    fn aesthetic_family(&self) -> Box<dyn AestheticFamily> {
        Box::new(FamVPosition)
    }

    /// Append a set of values to the scale.
    ///
    /// Expands the min and max values of the scale if they don't
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
}

/// For a layer, MappedData is parsed to specific aesthetics for a plot
pub struct MappedData {
    data: Vec<(Rc<dyn Aesthetic>, PlotParameter)>,
}
impl MappedData {
    fn aesthetics(&self) -> Vec<Rc<dyn Aesthetic>> {
        self.data.iter().map(|(aes, _)| aes).cloned().collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_blueprint() {
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![Mapping::X("x".into()), Mapping::Y("y".into())],
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let bp = Blueprint {
            mappings: vec![],
            layers: vec![layer],
            scales: vec![
                Box::new(ScaleXContinuous::new()),
                Box::new(ScaleYContinuous::new()),
            ],
            facets: vec![],
            coordinates: CoordinateSystem::Cartesian,
            theme: Theme {},
        };
    }
}
