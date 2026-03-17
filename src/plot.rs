use crate::aesthetic::{Aesthetic, AestheticFamily, Mapping};
use crate::column::{AesData, PlotData, ResolvedData};
use crate::geom::Geometry;
use crate::layout::{LayoutNode, PlotOutput, PlotRegion, SizeSpec, SplitAxis, Unit};
use crate::scale::{default_scale_for, PositionAdjustment, Scale, StatTransform};
use crate::shape::{Element, Text, VAlign};
use crate::theme::Theme;
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
                    if let Some(scale) = default_scale_for(aes) {
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

// should this be a trait?
enum CoordinateSystem {
    Cartesian,
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
    use crate::column::{PlotData, RawColumn};
    use crate::geom::{GeomLine, GeomPoint};
    use crate::scale::{Axis, ScaleColorDiscrete, ScalePositionContinuous};
    use crate::transform::ContinuousNumericScale;

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
