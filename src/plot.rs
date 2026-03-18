use crate::aesthetic::{Aesthetic, AestheticFamily, ConstantValue, Mapping};
use crate::column::{AesData, MappedColumn, PlotData, RawColumn, ResolvedData};
use crate::geom::Geometry;
use crate::layout::{LayoutNode, PlotOutput, PlotRegion, RegionKey, SizeSpec, SplitAxis, Unit};
use crate::scale::{default_scale_for, PositionAdjustment, Scale, StatTransform};
use crate::shape::{Element, Rectangle, Text, VAlign};
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

    /// Number of columns in facet grid (None = auto)
    facet_columns: Option<u32>,

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
            facet_columns: None,
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

    pub fn with_facet_columns(mut self, n: u32) -> Self {
        self.facet_columns = Some(n);
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
        // Step 1: Auto-map — discover aesthetics whose name matches a column name
        // and add them to blueprint mappings (if not already explicitly mapped).
        let mut already_mapped: Vec<Aesthetic> =
            self.mappings.iter().map(|m| m.aesthetic).collect();
        for aes in Aesthetic::all() {
            if already_mapped.contains(aes) {
                continue;
            }
            if raw_data.contains(aes.name()) {
                already_mapped.push(*aes);
                self.mappings.push(Mapping {
                    aesthetic: *aes,
                    variable: aes.name().to_string(),
                });
                let family = aes.family();
                if !self.scales.iter().any(|s| s.aesthetic_family() == family) {
                    if let Some(scale) = default_scale_for(aes) {
                        self.scales.push(scale);
                    }
                }
            }
        }

        // Step 2: Build per-layer AesData using effective mappings
        // (blueprint defaults overridden by layer-level mappings).
        let mut per_layer_aes: Vec<AesData> = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            let mut effective: HashMap<Aesthetic, &str> = self
                .mappings
                .iter()
                .map(|m| (m.aesthetic, m.variable.as_str()))
                .collect();
            for m in &layer.mappings {
                effective.insert(m.aesthetic, m.variable.as_str());
            }
            // Constants override mappings — don't fetch CSV data for those aesthetics
            for aes in layer.constants.keys() {
                effective.remove(aes);
            }
            let mut aes_data = AesData::new();
            for (aes, var) in &effective {
                let col = raw_data
                    .get(var)
                    .ok_or_else(|| format!("Column '{}' not found in data", var))?;
                aes_data.insert(*aes, col.clone());
            }
            per_layer_aes.push(aes_data);
        }

        // Step 3: Validate required aesthetics per layer
        // A constant value satisfies the aesthetic requirement.
        for (i, layer) in self.layers.iter().enumerate() {
            for aes in layer.geometry.required_aesthetics() {
                if !per_layer_aes[i].contains(aes) && !layer.constants.contains_key(&aes) {
                    return Err(format!("Missing required aesthetic {}", aes.name()));
                }
            }
        }

        // Step 4: Scale transforms (identity by default), applied per layer
        for aes_data in &mut per_layer_aes {
            for scale in &self.scales {
                *aes_data = scale.transform(aes_data.clone());
            }
        }

        // Step 5: Per-layer stat transforms and scale feeding
        let mut layer_aes_map: HashMap<usize, AesData> = HashMap::new();
        for (i, layer) in self.layers.iter().enumerate() {
            let transformed = layer.stat.transform(&per_layer_aes[i]);
            layer.geometry.update_scales(&mut self.scales, &transformed);
            // Feed constant float values to scales so they're included in the domain
            for (aes, constant) in &layer.constants {
                if let ConstantValue::Float(f) = constant {
                    let family = aes.family();
                    if let Some(scale) =
                        self.scales.iter_mut().find(|s| s.aesthetic_family() == family)
                    {
                        scale.append(&RawColumn::FloatArray(vec![*f])).ok();
                    }
                }
            }
            layer_aes_map.insert(i, transformed);
        }

        // Step 6: Fit scales
        for scale in &mut self.scales {
            scale.fit().expect("Scale can't be fit")
        }

        let mut regions: HashMap<RegionKey, Vec<Element>> = HashMap::new();

        let has_facets = !self.facets.is_empty();

        if has_facets {
            // --- Faceted rendering ---
            let facet_var = &self.facets[0];
            let facet_col = raw_data
                .get(facet_var)
                .ok_or_else(|| format!("Facet column '{}' not found in data", facet_var))?
                .clone();
            let facet_values = facet_col.distinct_strings();
            let num_panels = facet_values.len();
            let num_cols = self.facet_columns
                .map(|n| n as usize)
                .unwrap_or_else(|| (num_panels as f64).sqrt().ceil() as usize)
                .max(1);

            for (panel_idx, facet_value) in facet_values.iter().enumerate() {
                let indices = facet_col.indices_where_eq(facet_value);
                let panel_data = raw_data.subset(&indices);

                // Re-run step 2 (build AesData) for the subset
                let mut panel_layer_aes: Vec<AesData> = Vec::with_capacity(self.layers.len());
                for layer in &self.layers {
                    let mut effective: HashMap<Aesthetic, &str> = self
                        .mappings
                        .iter()
                        .map(|m| (m.aesthetic, m.variable.as_str()))
                        .collect();
                    for m in &layer.mappings {
                        effective.insert(m.aesthetic, m.variable.as_str());
                    }
                    for aes in layer.constants.keys() {
                        effective.remove(aes);
                    }
                    let mut aes_data = AesData::new();
                    for (aes, var) in &effective {
                        let col = panel_data
                            .get(var)
                            .ok_or_else(|| format!("Column '{}' not found in data", var))?;
                        aes_data.insert(*aes, col.clone());
                    }
                    panel_layer_aes.push(aes_data);
                }

                // Step 7: Bulk mapping with already-fitted scales
                for (i, layer) in self.layers.iter().enumerate() {
                    let layer_aes = &panel_layer_aes[i];
                    let resolved = self.resolve_layer(layer, layer_aes)?;
                    let mut geom_elements = layer.geometry.render(&resolved);
                    regions
                        .entry(RegionKey::panel(PlotRegion::DataArea, panel_idx))
                        .or_default()
                        .append(&mut geom_elements);
                }

                // Emit X-axis scale elements for bottom-of-column panels only
                let is_bottom_of_column = panel_idx + num_cols >= num_panels;
                if is_bottom_of_column {
                    for scale in &self.scales {
                        let (region, mut scale_elements) = scale.render(self.theme);
                        if region == PlotRegion::XAxisGutter {
                            regions
                                .entry(RegionKey::panel(PlotRegion::XAxisGutter, panel_idx))
                                .or_default()
                                .append(&mut scale_elements);
                        }
                    }
                }

                // Facet strip label with gray background
                let label_elements = regions
                    .entry(RegionKey::panel(PlotRegion::FacetLabel, panel_idx))
                    .or_default();
                label_elements.push(Element::Rect(Rectangle::new(
                    [Unit::Percent(50.0), Unit::Percent(50.0)],
                    Unit::Percent(100.0),
                    Unit::Percent(100.0),
                    self.theme.facet_label_bg_color,
                )));
                label_elements.push(Element::Text(
                    Text::centered(
                        facet_value.clone(),
                        self.theme.facet_label_font_size,
                        (Unit::Percent(50.0), Unit::Percent(50.0)),
                    )
                    .with_v_align(VAlign::Center)
                    .with_wrap(),
                ));

                // Panel border (4 edge rectangles around DataArea)
                let border_color = self.theme.panel_border_color;
                let border_px = self.theme.panel_border_thickness;
                let panel_borders = regions
                    .entry(RegionKey::panel(PlotRegion::DataArea, panel_idx))
                    .or_default();
                // Top edge
                panel_borders.push(Element::Rect(Rectangle::new(
                    [Unit::Percent(50.0), Unit::Percent(100.0)],
                    Unit::Percent(100.0), Unit::Pixels(border_px as u32), border_color,
                )));
                // Bottom edge
                panel_borders.push(Element::Rect(Rectangle::new(
                    [Unit::Percent(50.0), Unit::Percent(0.0)],
                    Unit::Percent(100.0), Unit::Pixels(border_px as u32), border_color,
                )));
                // Left edge
                panel_borders.push(Element::Rect(Rectangle::new(
                    [Unit::Percent(0.0), Unit::Percent(50.0)],
                    Unit::Pixels(border_px as u32), Unit::Percent(100.0), border_color,
                )));
                // Right edge
                panel_borders.push(Element::Rect(Rectangle::new(
                    [Unit::Percent(100.0), Unit::Percent(50.0)],
                    Unit::Pixels(border_px as u32), Unit::Percent(100.0), border_color,
                )));
            }

            // Per-row Y-axis (aligned with each row's DataArea)
            let num_rows = (num_panels + num_cols - 1) / num_cols;
            for row in 0..num_rows {
                for scale in &self.scales {
                    let (region, mut scale_elements) = scale.render(self.theme);
                    if region == PlotRegion::YAxisGutter {
                        regions
                            .entry(RegionKey::panel(PlotRegion::YAxisGutter, row))
                            .or_default()
                            .append(&mut scale_elements);
                    }
                }
            }

            // Shared legend
            for scale in &self.scales {
                let (region, mut scale_elements) = scale.render(self.theme);
                if region == PlotRegion::Legend {
                    regions
                        .entry(RegionKey::shared(PlotRegion::Legend))
                        .or_default()
                        .append(&mut scale_elements);
                }
            }

            // Labels
            self.emit_labels(&mut regions, &raw_data);

            let has_legend = self
                .scales
                .iter()
                .any(|s| s.aesthetic_family() == AestheticFamily::Color);
            let layout = faceted_plot_layout(
                num_panels,
                self.facet_columns,
                self.title.is_some(),
                self.caption.is_some(),
                has_legend,
                self.theme,
            );

            Ok(PlotOutput { regions, layout })
        } else {
            // --- Non-faceted rendering (original path) ---

            // Step 7: Bulk mapping + constant injection — build ResolvedData per layer
            for (i, layer) in self.layers.iter().enumerate() {
                let layer_aes = layer_aes_map.get(&i).unwrap();
                let resolved = self.resolve_layer(layer, layer_aes)?;
                let mut geom_elements = layer.geometry.render(&resolved);
                regions
                    .entry(RegionKey::shared(PlotRegion::DataArea))
                    .or_default()
                    .append(&mut geom_elements);
            }

            // Render scales — each declares its own region
            for scale in &self.scales {
                let (region, mut scale_elements) = scale.render(self.theme);
                regions
                    .entry(RegionKey::shared(region))
                    .or_default()
                    .append(&mut scale_elements);
            }

            // Labels
            self.emit_labels(&mut regions, &raw_data);

            let has_legend = self
                .scales
                .iter()
                .any(|s| s.aesthetic_family() == AestheticFamily::Color);
            let layout = standard_plot_layout(self.title.is_some(), self.caption.is_some(), has_legend, self.theme);

            Ok(PlotOutput { regions, layout })
        }
    }

    /// Bulk-map a single layer's AesData through fitted scales, injecting constants.
    fn resolve_layer(&self, layer: &Layer, layer_aes: &AesData) -> Result<ResolvedData, String> {
        let mut resolved = ResolvedData {
            mapped: HashMap::new(),
            raw: HashMap::new(),
        };
        for aes in Aesthetic::all() {
            if let Some(col) = layer_aes.get(*aes) {
                let family = aes.family();
                if let Some(scale) =
                    self.scales.iter().find(|s| s.aesthetic_family() == family)
                {
                    let mapped_col = scale.map(col)?;
                    resolved.mapped.insert(*aes, mapped_col);
                } else {
                    resolved.raw.insert(*aes, col.clone());
                }
            }
        }
        let data_len = layer_aes
            .get(Aesthetic::X)
            .or_else(|| layer_aes.get(Aesthetic::Y))
            .map(|c| c.len())
            .unwrap_or(0);
        for (aes, constant) in &layer.constants {
            match constant {
                ConstantValue::Color(rgb) => {
                    resolved
                        .mapped
                        .insert(*aes, MappedColumn::ColorArray(vec![*rgb; data_len]));
                }
                ConstantValue::Float(f) => {
                    let family = aes.family();
                    if let Some(scale) =
                        self.scales.iter().find(|s| s.aesthetic_family() == family)
                    {
                        let raw = RawColumn::FloatArray(vec![*f; data_len]);
                        let mapped_col = scale.map(&raw)?;
                        resolved.mapped.insert(*aes, mapped_col);
                    }
                }
            }
        }
        Ok(resolved)
    }

    /// Emit title, axis labels, and caption into the regions map.
    fn emit_labels(&self, regions: &mut HashMap<RegionKey, Vec<Element>>, _raw_data: &PlotData) {
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

        if let Some(title) = &self.title {
            regions
                .entry(RegionKey::shared(PlotRegion::Title))
                .or_default()
                .push(Element::Text(Text::centered(
                    title.clone(),
                    self.theme.title_font_size,
                    (Unit::Percent(50.0), Unit::Percent(50.0)),
                ).with_wrap()));
        }
        if let Some(label) = x_label {
            regions
                .entry(RegionKey::shared(PlotRegion::XAxisGutter))
                .or_default()
                .push(Element::Text(Text::centered(
                    label,
                    self.theme.axis_label_font_size,
                    (Unit::Percent(50.0), Unit::Percent(50.0)),
                ).with_wrap()));
        }
        if let Some(label) = y_label {
            regions
                .entry(RegionKey::shared(PlotRegion::YAxisGutter))
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
                .entry(RegionKey::shared(PlotRegion::Caption))
                .or_default()
                .push(Element::Text(Text::centered(
                    caption.clone(),
                    self.theme.caption_font_size,
                    (Unit::Percent(50.0), Unit::Percent(50.0)),
                ).with_wrap()));
        }
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
    /// Per-layer mapping overrides (not blueprint defaults).
    mappings: Vec<Mapping>,
    /// Hardcoded constant values for aesthetics (bypass scale mapping).
    constants: HashMap<Aesthetic, ConstantValue>,
    stat: Box<dyn StatTransform>,
    position: Box<dyn PositionAdjustment>,
}
impl Layer {
    pub fn new(
        geometry: Box<dyn Geometry>,
        mappings: Vec<Mapping>,
        constants: HashMap<Aesthetic, ConstantValue>,
        stat: Box<dyn StatTransform>,
        position: Box<dyn PositionAdjustment>,
    ) -> Self {
        Self {
            geometry,
            mappings,
            constants,
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
            (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::shared(PlotRegion::DataArea))),
            (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::XAxisGutter))),
        ],
    };

    let y_axis_column = LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: vec![
            (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::shared(PlotRegion::YAxisGutter))),
            (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
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
                (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Legend))),
                (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
            ],
        };
        main_columns.push((SizeSpec::Pixels(theme.legend_margin), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
        main_columns.push((SizeSpec::Pixels(theme.legend_width), legend_column));
    }

    let main = LayoutNode::Split {
        axis: SplitAxis::Horizontal,
        children: main_columns,
    };

    let mut rows: Vec<(SizeSpec, LayoutNode)> = vec![];
    if has_title {
        rows.push((SizeSpec::Pixels(theme.title_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Title))));
    }
    rows.push((SizeSpec::Flex(1.0), main));
    if has_caption {
        rows.push((SizeSpec::Pixels(theme.caption_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Caption))));
    }

    LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: rows,
    }
}

/// Build a faceted plot layout tree.
///
/// Each row: per-row Y-axis gutter (tick marks) + panel cells (FacetLabel + DataArea + XAxisGutter).
/// Every cell gets an XAxisGutter for uniform row height; only bottom-of-column panels get tick content.
/// Shared YAxisGutter (axis label) on far left, shared XAxisGutter (axis label) below the grid.
fn faceted_plot_layout(
    num_panels: usize,
    facet_columns: Option<u32>,
    has_title: bool,
    has_caption: bool,
    has_legend: bool,
    theme: &Theme,
) -> LayoutNode {
    let num_cols = facet_columns
        .map(|n| n as usize)
        .unwrap_or_else(|| (num_panels as f64).sqrt().ceil() as usize)
        .max(1);
    let num_rows = (num_panels + num_cols - 1) / num_cols;

    // Build grid rows, each with its own Y-axis gutter aligned to the DataArea.
    // Every cell has uniform vertical structure: FacetLabel + DataArea + XAxisGutter.
    let mut grid_rows: Vec<(SizeSpec, LayoutNode)> = Vec::new();
    for row in 0..num_rows {
        // Y-axis gutter for this row mirrors the cell vertical structure
        let y_gutter_col = LayoutNode::Split {
            axis: SplitAxis::Vertical,
            children: vec![
                (SizeSpec::Pixels(theme.facet_label_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
                (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::YAxisGutter, row))),
                (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
            ],
        };

        // Panel cells for this row, with horizontal gaps between columns
        let mut row_cells: Vec<(SizeSpec, LayoutNode)> = Vec::new();
        for col in 0..num_cols {
            if col > 0 {
                row_cells.push((SizeSpec::Pixels(theme.facet_gap), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
            }
            let panel_idx = row * num_cols + col;
            if panel_idx >= num_panels {
                row_cells.push((SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
                continue;
            }
            row_cells.push((SizeSpec::Flex(1.0), LayoutNode::Split {
                axis: SplitAxis::Vertical,
                children: vec![
                    (SizeSpec::Pixels(theme.facet_label_height), LayoutNode::Leaf(RegionKey::panel(PlotRegion::FacetLabel, panel_idx))),
                    (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::DataArea, panel_idx))),
                    (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::panel(PlotRegion::XAxisGutter, panel_idx))),
                ],
            }));
        }

        let panel_grid = LayoutNode::Split {
            axis: SplitAxis::Horizontal,
            children: row_cells,
        };

        let row_node = LayoutNode::Split {
            axis: SplitAxis::Horizontal,
            children: vec![
                (SizeSpec::Pixels(theme.y_gutter_width), y_gutter_col),
                (SizeSpec::Flex(1.0), panel_grid),
            ],
        };

        grid_rows.push((SizeSpec::Flex(1.0), row_node));
    }

    let grid = LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: grid_rows,
    };

    // Shared Y-axis label (rotated) on far left + grid + shared X-axis label below
    let y_label_column = LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: vec![
            (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::shared(PlotRegion::YAxisGutter))),
            (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
        ],
    };

    let grid_with_x_label = LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: vec![
            (SizeSpec::Flex(1.0), grid),
            (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::XAxisGutter))),
        ],
    };

    let mut main_columns: Vec<(SizeSpec, LayoutNode)> = vec![
        (SizeSpec::Pixels(theme.y_gutter_width), y_label_column),
        (SizeSpec::Flex(1.0), grid_with_x_label),
    ];

    if has_legend {
        let legend_column = LayoutNode::Split {
            axis: SplitAxis::Vertical,
            children: vec![
                (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Legend))),
                (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
            ],
        };
        main_columns.push((SizeSpec::Pixels(theme.legend_margin), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
        main_columns.push((SizeSpec::Pixels(theme.legend_width), legend_column));
    }

    let main = LayoutNode::Split {
        axis: SplitAxis::Horizontal,
        children: main_columns,
    };

    // Outer frame: title + main + caption
    let mut rows: Vec<(SizeSpec, LayoutNode)> = vec![];
    if has_title {
        rows.push((SizeSpec::Pixels(theme.title_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Title))));
    }
    rows.push((SizeSpec::Flex(1.0), main));
    if has_caption {
        rows.push((SizeSpec::Pixels(theme.caption_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Caption))));
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
    use crate::layout::RegionKey;
    use crate::scale::{Axis, IdentityTransform, ScaleColorDiscrete, ScalePositionContinuous};
    use crate::transform::ContinuousNumericScale;

    #[test]
    fn render_blueprint_with_color() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            HashMap::new(),
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
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .map_or(0, |v| v.len());
        assert!(data_count >= 3);
        assert!(output.regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
    }

    #[test]
    fn render_blueprint_without_color() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            HashMap::new(),
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
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
    }

    #[test]
    fn auto_map_from_column_names() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            HashMap::new(),
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
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
    }

    #[test]
    fn auto_map_with_color() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            HashMap::new(),
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
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
        assert!(output.regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
    }

    #[test]
    fn auto_map_produces_axis_labels() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            HashMap::new(),
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
            HashMap::new(),
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
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
    }

    #[test]
    fn geom_line_no_group_produces_segments() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomLine),
            vec![],
            HashMap::new(),
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
            .get(&RegionKey::shared(PlotRegion::DataArea))
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
            HashMap::new(),
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
            .get(&RegionKey::shared(PlotRegion::DataArea))
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
            HashMap::new(),
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
        let _ = output.regions.get(&RegionKey::shared(PlotRegion::DataArea));
    }

    #[test]
    fn render_blueprint_geom_line_with_color() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomLine),
            vec![],
            HashMap::new(),
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
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
        assert!(output.regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
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
            HashMap::new(),
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

        let data = regions.get(&RegionKey::shared(PlotRegion::DataArea)).unwrap();
        let xgutter = regions.get(&RegionKey::shared(PlotRegion::XAxisGutter)).unwrap();

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

        let data = regions.get(&RegionKey::shared(PlotRegion::DataArea)).unwrap();
        let ygutter = regions.get(&RegionKey::shared(PlotRegion::YAxisGutter)).unwrap();

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
        assert!(!regions.contains_key(&RegionKey::shared(PlotRegion::Spacer)));
    }

    #[test]
    fn layout_with_legend_has_all_regions() {
        let seg = layout_test_segment();
        let layout = standard_plot_layout(true, true, true, &Theme::default());
        let regions = layout.resolve(&seg);
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::DataArea)));
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::XAxisGutter)));
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::YAxisGutter)));
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::Title)));
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::Caption)));
    }

    #[test]
    fn constant_color_produces_colored_points_no_legend() {
        let theme = Theme::default();
        let mut constants = HashMap::new();
        constants.insert(
            Aesthetic::Color,
            ConstantValue::Color([1.0, 0.0, 0.0]),
        );
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            constants,
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping { aesthetic: Aesthetic::X, variable: "x".into() })
            .with_mapping(Mapping { aesthetic: Aesthetic::Y, variable: "y".into() })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0]));

        let output = bp.render(data).expect("render should succeed");
        // Constant color — no color scale — no legend
        assert!(!output.regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
        // 3 points rendered
        let data_count = output.regions.get(&RegionKey::shared(PlotRegion::DataArea)).map_or(0, |v| v.len());
        assert!(data_count >= 3);
        // Points should carry the injected red color
        let all_red = output
            .regions
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                Element::Point(p) => Some(p.color),
                _ => None,
            })
            .all(|c| (c[0] - 1.0).abs() < 1e-5 && c[1].abs() < 1e-5 && c[2].abs() < 1e-5);
        assert!(all_red, "all points should be red");
    }

    #[test]
    fn per_layer_mapping_override_uses_correct_column() {
        let theme = Theme::default();
        // Blueprint maps y → "a"; layer overrides y → "b"
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![Mapping { aesthetic: Aesthetic::Y, variable: "b".into() }],
            HashMap::new(),
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping { aesthetic: Aesthetic::X, variable: "x".into() })
            .with_mapping(Mapping { aesthetic: Aesthetic::Y, variable: "a".into() })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0]));
        data.insert("a".into(), RawColumn::FloatArray(vec![10.0, 20.0]));
        data.insert("b".into(), RawColumn::FloatArray(vec![100.0, 200.0]));

        let output = bp.render(data).expect("render should succeed");
        let data_count = output.regions.get(&RegionKey::shared(PlotRegion::DataArea)).map_or(0, |v| v.len());
        assert!(data_count >= 2);
    }

    #[test]
    fn constant_float_position_renders_and_affects_scale() {
        let theme = Theme::default();
        // Layer 1: normal scatter from data
        let layer1 = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            HashMap::new(),
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        // Layer 2: constant y=0, should produce points at y=0
        let mut constants = HashMap::new();
        constants.insert(Aesthetic::Y, ConstantValue::Float(0.0));
        let layer2 = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            constants,
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer1)
            .with_layer(layer2)
            .with_mapping(Mapping { aesthetic: Aesthetic::X, variable: "x".into() })
            .with_mapping(Mapping { aesthetic: Aesthetic::Y, variable: "y".into() })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![10.0, 20.0, 30.0]));

        let output = bp.render(data).expect("render should succeed");
        let points: Vec<_> = output
            .regions
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                Element::Point(p) => Some(p),
                _ => None,
            })
            .collect();
        // 3 from layer 1 + 3 from layer 2
        assert_eq!(points.len(), 6, "expected 6 points, got {}", points.len());
        // Layer 2 points should all share the same Y position (y=0 mapped through scale)
        let layer2_y_ndc: Vec<f32> = points[3..]
            .iter()
            .map(|p| match p.position[1] {
                Unit::NDC(v) => v,
                _ => panic!("expected NDC unit"),
            })
            .collect();
        assert!(
            layer2_y_ndc.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-6),
            "all layer-2 points should have the same Y position"
        );
    }

    #[test]
    fn layout_without_optional_regions() {
        let seg = layout_test_segment();
        let layout = standard_plot_layout(false, false, false, &Theme::default());
        let regions = layout.resolve(&seg);
        assert!(!regions.contains_key(&RegionKey::shared(PlotRegion::Title)));
        assert!(!regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
        assert!(!regions.contains_key(&RegionKey::shared(PlotRegion::Caption)));
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::DataArea)));
    }

    #[test]
    fn facet_splits_data_into_panels() {
        let theme = Theme::default();
        let layer = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            HashMap::new(),
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
        let mut bp = Blueprint::new(&theme)
            .with_layer(layer)
            .with_mapping(Mapping { aesthetic: Aesthetic::X, variable: "x".into() })
            .with_mapping(Mapping { aesthetic: Aesthetic::Y, variable: "y".into() })
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::X)))
            .with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)))
            .with_facet("grp".into());

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]));
        data.insert("grp".into(), RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]));

        let output = bp.render(data).expect("faceted render should succeed");

        // Panel 0 ("a") should have 2 points
        let panel0 = output.regions.get(&RegionKey::panel(PlotRegion::DataArea, 0))
            .expect("panel 0 should exist");
        let p0_points: Vec<_> = panel0.iter().filter(|e| matches!(e, Element::Point(_))).collect();
        assert_eq!(p0_points.len(), 2);

        // Panel 1 ("b") should have 2 points
        let panel1 = output.regions.get(&RegionKey::panel(PlotRegion::DataArea, 1))
            .expect("panel 1 should exist");
        let p1_points: Vec<_> = panel1.iter().filter(|e| matches!(e, Element::Point(_))).collect();
        assert_eq!(p1_points.len(), 2);

        // Should NOT have a shared DataArea
        assert!(!output.regions.contains_key(&RegionKey::shared(PlotRegion::DataArea)));

        // Should have facet labels
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetLabel, 0)));
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetLabel, 1)));

        // Per-row Y axis (2 groups → 2 cols default → 1 row)
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::YAxisGutter, 0)));
    }

    #[test]
    fn faceted_layout_resolves_all_panels() {
        let seg = layout_test_segment();
        let theme = Theme::default();
        let layout = faceted_plot_layout(3, None, true, false, false, &theme);
        let regions = layout.resolve(&seg);

        // 3 panels → ceil(sqrt(3)) = 2 columns, 2 rows
        for i in 0..3 {
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::DataArea, i)),
                "panel {} DataArea missing", i);
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::FacetLabel, i)),
                "panel {} FacetLabel missing", i);
        }
        // Per-row Y-axis gutters
        assert!(regions.contains_key(&RegionKey::panel(PlotRegion::YAxisGutter, 0)));
        assert!(regions.contains_key(&RegionKey::panel(PlotRegion::YAxisGutter, 1)));
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::Title)));
    }

    #[test]
    fn facet_columns_override() {
        let seg = layout_test_segment();
        let theme = Theme::default();
        // 4 panels, force 4 columns → 1 row
        let layout = faceted_plot_layout(4, Some(4), false, false, false, &theme);
        let regions = layout.resolve(&seg);
        for i in 0..4 {
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::DataArea, i)));
        }
    }
}
