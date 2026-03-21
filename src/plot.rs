use crate::aesthetic::{Aesthetic, AestheticFamily, ConstantValue, Mapping};
use crate::ast::{FacetSpec, ScaleFreedom};
use crate::column::{AesData, MappedColumn, PlotData, RawColumn, ResolvedData};
use crate::error::GglangError;
use crate::geom::Geometry;
use crate::layout::{LayoutNode, PlotOutput, PlotRegion, RegionKey, SizeSpec, SplitAxis, Unit};
use crate::scale::{default_scale_for, PositionAdjustment, Scale, StatTransform};
use crate::shape::{Element, Rectangle, Text, TextRotation, VAlign};
use crate::theme::Theme;
use std::collections::{HashMap, HashSet};

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

    /// Faceting specification
    facet: Option<FacetSpec>,

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
            facet: None,
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

    pub fn with_facet_spec(mut self, spec: FacetSpec) -> Self {
        self.facet = Some(spec);
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

    /// Build AesData for each layer given a dataset (handles effective mappings + constants).
    fn build_layer_aes(&self, data: &PlotData) -> Result<Vec<AesData>, GglangError> {
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
            for aes in layer.constants.keys() {
                effective.remove(aes);
            }
            let mut aes_data = AesData::new();
            for (aes, var) in &effective {
                let col = data.get(var).ok_or_else(|| GglangError::Render {
                    message: format!("Column '{}' not found in data", var),
                })?;
                aes_data.insert(*aes, col.clone());
            }
            per_layer_aes.push(aes_data);
        }
        Ok(per_layer_aes)
    }

    /// Bulk-map a single layer's AesData through provided scales, injecting constants.
    fn resolve_layer_with_scales(
        &self,
        layer: &Layer,
        layer_aes: &AesData,
        scales: &[Box<dyn Scale>],
    ) -> Result<ResolvedData, GglangError> {
        let refs: Vec<&dyn Scale> = scales.iter().map(|s| s.as_ref()).collect();
        self.resolve_layer_with_scale_refs(layer, layer_aes, &refs)
    }

    /// Bulk-map a single layer's AesData through scale references, injecting constants.
    fn resolve_layer_with_scale_refs(
        &self,
        layer: &Layer,
        layer_aes: &AesData,
        scales: &[&dyn Scale],
    ) -> Result<ResolvedData, GglangError> {
        let mut resolved = ResolvedData {
            mapped: HashMap::new(),
            raw: HashMap::new(),
        };
        for aes in Aesthetic::all() {
            if let Some(col) = layer_aes.get(*aes) {
                let family = aes.family();
                if let Some(scale) = scales.iter().find(|s| s.aesthetic_family() == family) {
                    let mapped_col = scale.map(col)?;
                    resolved.mapped.insert(*aes, mapped_col);
                }
                // Always preserve raw data — geoms like GeomBar need raw values
                // for stacking even when the aesthetic has been mapped through a scale
                resolved.raw.insert(*aes, col.clone());
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
                    if let Some(scale) = scales.iter().find(|s| s.aesthetic_family() == family) {
                        let raw = RawColumn::FloatArray(vec![*f; data_len]);
                        let mapped_col = scale.map(&raw)?;
                        resolved.mapped.insert(*aes, mapped_col);
                    }
                }
            }
        }
        Ok(resolved)
    }

    /// Bulk-map a single layer's AesData through the blueprint's fitted scales.
    fn resolve_layer(&self, layer: &Layer, layer_aes: &AesData) -> Result<ResolvedData, GglangError> {
        self.resolve_layer_with_scales(layer, layer_aes, &self.scales)
    }

    /// Render panel borders into a DataArea region.
    fn emit_panel_borders(&self, elements: &mut Vec<Element>) {
        let border_color = self.theme.panel_border_color;
        let border_px = self.theme.panel_border_thickness;
        // Top edge
        elements.push(Element::Rect(Rectangle::new(
            [Unit::Percent(50.0), Unit::Percent(100.0)],
            Unit::Percent(100.0), Unit::Pixels(border_px as u32), border_color,
        )));
        // Bottom edge
        elements.push(Element::Rect(Rectangle::new(
            [Unit::Percent(50.0), Unit::Percent(0.0)],
            Unit::Percent(100.0), Unit::Pixels(border_px as u32), border_color,
        )));
        // Left edge
        elements.push(Element::Rect(Rectangle::new(
            [Unit::Percent(0.0), Unit::Percent(50.0)],
            Unit::Pixels(border_px as u32), Unit::Percent(100.0), border_color,
        )));
        // Right edge
        elements.push(Element::Rect(Rectangle::new(
            [Unit::Percent(100.0), Unit::Percent(50.0)],
            Unit::Pixels(border_px as u32), Unit::Percent(100.0), border_color,
        )));
    }

    /// Emit a facet strip label (gray background + centered text).
    fn emit_facet_label(&self, regions: &mut HashMap<RegionKey, Vec<Element>>, key: RegionKey, label: &str) {
        self.emit_facet_label_rotated(regions, key, label, TextRotation::None);
    }

    fn emit_facet_label_rotated(&self, regions: &mut HashMap<RegionKey, Vec<Element>>, key: RegionKey, label: &str, rotation: TextRotation) {
        let elements = regions.entry(key).or_default();
        elements.push(Element::Rect(Rectangle::new(
            [Unit::Percent(50.0), Unit::Percent(50.0)],
            Unit::Percent(100.0),
            Unit::Percent(100.0),
            self.theme.facet_label_bg_color,
        )));
        let mut text = Text::centered(
                label.to_string(),
                self.theme.facet_label_font_size,
                (Unit::Percent(50.0), Unit::Percent(50.0)),
            )
            .with_v_align(VAlign::Center)
            .with_wrap();
        if rotation != TextRotation::None {
            text = text.with_rotation(rotation);
        }
        elements.push(Element::Text(
            text,
        ));
    }

    /// Render a plot from this blueprint.
    ///
    /// Data is provided with raw column names; the blueprint's mappings are
    /// applied to bind columns to aesthetic channels before rendering.
    pub fn render(&mut self, raw_data: PlotData) -> Result<PlotOutput, GglangError> {
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
                    let data_hint = raw_data.get(aes.name());
                    if let Some(scale) = default_scale_for(aes, data_hint) {
                        self.scales.push(scale);
                    }
                }
            }
        }

        // Step 2: Build per-layer AesData using effective mappings
        let per_layer_aes = self.build_layer_aes(&raw_data)?;

        // Step 2b: Create default scales for explicitly-mapped aesthetics that don't
        // have a scale yet (compile-time defaults were removed; we pick the right
        // scale type now that we can inspect actual column types).
        for aes in Aesthetic::all() {
            let family = aes.family();
            if self.has_scale_for_family(family) {
                continue;
            }
            // Find the first layer that has data for this aesthetic
            for layer_aes in &per_layer_aes {
                if let Some(col) = layer_aes.get(*aes) {
                    if let Some(scale) = default_scale_for(aes, Some(col)) {
                        self.scales.push(scale);
                    }
                    break;
                }
            }
        }

        // Step 3: Validate required aesthetics per layer
        for (i, layer) in self.layers.iter().enumerate() {
            for aes in layer.geometry.required_aesthetics() {
                if !per_layer_aes[i].contains(aes) && !layer.constants.contains_key(&aes) {
                    return Err(GglangError::Render {
                        message: format!("Missing required aesthetic '{}'", aes.name()),
                    });
                }
            }
        }

        // Step 4: Scale transforms (identity by default), applied per layer
        let mut per_layer_aes = per_layer_aes;
        for aes_data in &mut per_layer_aes {
            for scale in &self.scales {
                *aes_data = scale.transform(aes_data.clone());
            }
        }

        // Step 5: Per-layer stat transforms and scale feeding
        let mut layer_aes_map: HashMap<usize, AesData> = HashMap::new();
        for (i, layer) in self.layers.iter().enumerate() {
            let transformed = layer.stat.transform(&per_layer_aes[i]);
            layer.geometry.update_scales(&mut self.scales, &transformed)?;
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
            scale.fit()?;
        }

        let mut regions: HashMap<RegionKey, Vec<Element>> = HashMap::new();

        match &self.facet {
            Some(spec) => {
                let spec = spec.clone();
                match &spec {
                    FacetSpec::Wrap { variable, columns, scales } => {
                        self.render_facet_wrap(&raw_data, variable, *columns, scales, &mut regions)?;
                    }
                    FacetSpec::Grid { row_var, col_var, scales } => {
                        self.render_facet_grid(&raw_data, row_var.as_deref(), col_var.as_deref(), scales, &mut regions)?;
                    }
                }
                let has_legend = self
                    .scales
                    .iter()
                    .any(|s| matches!(s.aesthetic_family(), AestheticFamily::Color | AestheticFamily::Fill));
                let layout = match &spec {
                    FacetSpec::Wrap { variable, columns, scales } => {
                        let facet_col = raw_data.get(variable).ok_or_else(|| GglangError::Render {
                            message: format!("Facet variable '{}' not found in data", variable),
                        })?;
                        let num_panels = facet_col.distinct_strings().len();
                        let y_free = matches!(scales, ScaleFreedom::Free | ScaleFreedom::FreeY);
                        faceted_wrap_layout(
                            num_panels,
                            *columns,
                            self.title.is_some(),
                            self.caption.is_some(),
                            has_legend,
                            y_free,
                            self.theme,
                        )
                    }
                    FacetSpec::Grid { row_var, col_var, .. } => {
                        let num_row_values = if let Some(v) = row_var.as_ref() {
                            raw_data.get(v).ok_or_else(|| GglangError::Render {
                                message: format!("Facet row variable '{}' not found in data", v),
                            })?.distinct_strings().len()
                        } else {
                            1
                        };
                        let num_col_values = if let Some(v) = col_var.as_ref() {
                            raw_data.get(v).ok_or_else(|| GglangError::Render {
                                message: format!("Facet col variable '{}' not found in data", v),
                            })?.distinct_strings().len()
                        } else {
                            1
                        };
                        faceted_grid_layout(
                            num_row_values,
                            num_col_values,
                            self.title.is_some(),
                            self.caption.is_some(),
                            has_legend,
                            self.theme,
                        )
                    }
                };
                Ok(PlotOutput { regions, layout })
            }
            None => {
                // --- Non-faceted rendering (original path) ---

                // Step 7: Bulk mapping + constant injection — build ResolvedData per layer
                for (i, layer) in self.layers.iter().enumerate() {
                    let layer_aes = layer_aes_map.get(&i).ok_or_else(|| GglangError::Render {
                        message: format!("Layer {} has no AesData", i),
                    })?;
                    let resolved = self.resolve_layer(layer, layer_aes)?;
                    let mut geom_elements = layer.geometry.render(&resolved)?;
                    regions
                        .entry(RegionKey::shared(PlotRegion::DataArea))
                        .or_default()
                        .append(&mut geom_elements);
                }

                // Render scales — each declares its own region
                for scale in &self.scales {
                    let (region, mut scale_elements) = scale.render(self.theme)?;
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
                    .any(|s| matches!(s.aesthetic_family(), AestheticFamily::Color | AestheticFamily::Fill));
                let layout = standard_plot_layout(self.title.is_some(), self.caption.is_some(), has_legend, self.theme);

                Ok(PlotOutput { regions, layout })
            }
        }
    }

    /// Render facet wrap panels.
    fn render_facet_wrap(
        &self,
        raw_data: &PlotData,
        facet_var: &str,
        facet_columns: Option<u32>,
        scale_freedom: &ScaleFreedom,
        regions: &mut HashMap<RegionKey, Vec<Element>>,
    ) -> Result<(), GglangError> {
        let facet_col = raw_data
            .get(facet_var)
            .ok_or_else(|| GglangError::Render {
                message: format!("Facet column '{}' not found in data", facet_var),
            })?
            .clone();
        let facet_values = facet_col.distinct_strings();
        let num_panels = facet_values.len();
        let num_cols = facet_columns
            .map(|n| n as usize)
            .unwrap_or_else(|| (num_panels as f64).sqrt().ceil() as usize)
            .max(1);

        // For free scales, we build per-panel scale sets
        let x_free = matches!(scale_freedom, ScaleFreedom::Free | ScaleFreedom::FreeX);
        let y_free = matches!(scale_freedom, ScaleFreedom::Free | ScaleFreedom::FreeY);

        // Build per-panel scale sets if needed
        let mut panel_scales: Vec<Option<Vec<Box<dyn Scale>>>> = (0..num_panels).map(|_| None).collect();

        if x_free || y_free {
            for (panel_idx, facet_value) in facet_values.iter().enumerate() {
                let indices = facet_col.indices_where_eq(facet_value);
                let panel_data = raw_data.subset(&indices);
                let panel_layer_aes = self.build_layer_aes(&panel_data)?;

                // Build only the free scales for this panel
                let mut free_scales: Vec<Box<dyn Scale>> = Vec::new();
                for s in &self.scales {
                    let family = s.aesthetic_family();
                    let is_free = (x_free && family == AestheticFamily::HorizontalPosition)
                        || (y_free && family == AestheticFamily::VerticalPosition);
                    if !is_free { continue; }
                    let mut free_scale = s.clone_unfitted();
                    for (i, layer) in self.layers.iter().enumerate() {
                        let aes = &panel_layer_aes[i];
                        for a in Aesthetic::all() {
                            if a.family() == family {
                                if let Some(col) = aes.get(*a) {
                                    free_scale.append(col).ok();
                                }
                            }
                        }
                        for (a, constant) in &layer.constants {
                            if let ConstantValue::Float(f) = constant {
                                if a.family() == family {
                                    free_scale.append(&RawColumn::FloatArray(vec![*f])).ok();
                                }
                            }
                        }
                    }
                    free_scale.fit()?;
                    free_scales.push(free_scale);
                }

                panel_scales[panel_idx] = Some(free_scales);
            }
        }

        for (panel_idx, facet_value) in facet_values.iter().enumerate() {
            let indices = facet_col.indices_where_eq(facet_value);
            let panel_data = raw_data.subset(&indices);
            let panel_layer_aes = self.build_layer_aes(&panel_data)?;

            // Compose scale refs: shared from self.scales, free from panel_scales
            let scale_refs = self.build_wrap_panel_scale_refs(
                x_free, y_free, &panel_scales[panel_idx],
            );

            // Render layer geometry
            for (i, layer) in self.layers.iter().enumerate() {
                let resolved = self.resolve_layer_with_scale_refs(layer, &panel_layer_aes[i], &scale_refs)?;
                let mut geom_elements = layer.geometry.render(&resolved)?;
                regions
                    .entry(RegionKey::panel(PlotRegion::DataArea, panel_idx))
                    .or_default()
                    .append(&mut geom_elements);
            }

            // X-axis tick marks
            let is_bottom_of_column = panel_idx + num_cols >= num_panels;
            let emit_x_ticks = x_free || is_bottom_of_column;
            if emit_x_ticks {
                for scale in &scale_refs {
                    let (region, mut scale_elements) = scale.render(self.theme)?;
                    if region == PlotRegion::XAxisGutter {
                        regions
                            .entry(RegionKey::panel(PlotRegion::XAxisGutter, panel_idx))
                            .or_default()
                            .append(&mut scale_elements);
                    }
                }
            }

            // Facet strip label
            self.emit_facet_label(regions, RegionKey::panel(PlotRegion::FacetLabel, panel_idx), facet_value);

            // Panel borders
            self.emit_panel_borders(
                regions.entry(RegionKey::panel(PlotRegion::DataArea, panel_idx)).or_default()
            );
        }

        // Per-row Y-axis
        let num_rows = (num_panels + num_cols - 1) / num_cols;
        if y_free {
            // Every panel gets its own Y-axis ticks keyed by panel_idx
            for (panel_idx, _) in facet_values.iter().enumerate() {
                let scale_refs = self.build_wrap_panel_scale_refs(
                    x_free, y_free, &panel_scales[panel_idx],
                );
                for scale in &scale_refs {
                    let (region, mut scale_elements) = scale.render(self.theme)?;
                    if region == PlotRegion::YAxisGutter {
                        regions
                            .entry(RegionKey::panel(PlotRegion::YAxisGutter, panel_idx))
                            .or_default()
                            .append(&mut scale_elements);
                    }
                }
            }
        } else {
            for row in 0..num_rows {
                for scale in &self.scales {
                    let (region, mut scale_elements) = scale.render(self.theme)?;
                    if region == PlotRegion::YAxisGutter {
                        regions
                            .entry(RegionKey::panel(PlotRegion::YAxisGutter, row))
                            .or_default()
                            .append(&mut scale_elements);
                    }
                }
            }
        }

        // Shared legend
        for scale in &self.scales {
            let (region, mut scale_elements) = scale.render(self.theme)?;
            if region == PlotRegion::Legend {
                regions
                    .entry(RegionKey::shared(PlotRegion::Legend))
                    .or_default()
                    .append(&mut scale_elements);
            }
        }

        // Labels
        self.emit_labels(regions, raw_data);

        Ok(())
    }

    /// Render facet grid panels.
    fn render_facet_grid(
        &self,
        raw_data: &PlotData,
        row_var: Option<&str>,
        col_var: Option<&str>,
        scale_freedom: &ScaleFreedom,
        regions: &mut HashMap<RegionKey, Vec<Element>>,
    ) -> Result<(), GglangError> {
        let row_values: Vec<String> = if let Some(rv) = row_var {
            let col = raw_data.get(rv).ok_or_else(|| GglangError::Render {
                message: format!("Row facet column '{}' not found", rv),
            })?;
            col.distinct_strings()
        } else {
            vec!["".to_string()]
        };
        let col_values: Vec<String> = if let Some(cv) = col_var {
            let col = raw_data.get(cv).ok_or_else(|| GglangError::Render {
                message: format!("Col facet column '{}' not found", cv),
            })?;
            col.distinct_strings()
        } else {
            vec!["".to_string()]
        };

        let num_grid_rows = row_values.len();
        let num_grid_cols = col_values.len();

        let x_free = matches!(scale_freedom, ScaleFreedom::Free | ScaleFreedom::FreeX);
        let y_free = matches!(scale_freedom, ScaleFreedom::Free | ScaleFreedom::FreeY);

        // In grid mode, free_x means per-column X scales, free_y means per-row Y scales.
        // However, when one dimension is absent (COLS-only or ROWS-only), the missing
        // dimension has exactly one value, so per-row/per-column degenerates to shared.
        // In that case, we fall back to per-panel scales for the free axis:
        //   COLS-only + free_y → per-column Y scales (= per-panel, since one row)
        //   ROWS-only + free_x → per-row X scales (= per-panel, since one column)
        let x_free_per_col = x_free && col_var.is_some();
        let x_free_per_row = x_free && col_var.is_none(); // ROWS-only: per-row X = per-panel
        let y_free_per_row = y_free && row_var.is_some();
        let y_free_per_col = y_free && row_var.is_none(); // COLS-only: per-col Y = per-panel

        // Build per-column X scales (standard grid behavior, or ROWS-only skips this)
        let col_x_scales: Option<Vec<Vec<Box<dyn Scale>>>> = if x_free_per_col {
            let mut result = Vec::new();
            for (_ci, cv) in col_values.iter().enumerate() {
                let mut scales = self.build_free_scales_for_family(AestheticFamily::HorizontalPosition)?;
                for (_ri, rv) in row_values.iter().enumerate() {
                    let panel_data = self.subset_grid_panel(raw_data, row_var, rv, col_var, cv)?;
                    let panel_layer_aes = self.build_layer_aes(&panel_data)?;
                    for (i, _layer) in self.layers.iter().enumerate() {
                        for scale in scales.iter_mut() {
                            for a in Aesthetic::all() {
                                if a.family() == AestheticFamily::HorizontalPosition {
                                    if let Some(col) = panel_layer_aes[i].get(*a) {
                                        scale.append(col).ok();
                                    }
                                }
                            }
                        }
                    }
                }
                for s in scales.iter_mut() { s.fit()?; }
                result.push(scales);
            }
            Some(result)
        } else {
            None
        };

        // Build per-row X scales (fallback for ROWS-only + free_x)
        let row_x_scales: Option<Vec<Vec<Box<dyn Scale>>>> = if x_free_per_row {
            let mut result = Vec::new();
            for (_ri, rv) in row_values.iter().enumerate() {
                let mut scales = self.build_free_scales_for_family(AestheticFamily::HorizontalPosition)?;
                for (_ci, cv) in col_values.iter().enumerate() {
                    let panel_data = self.subset_grid_panel(raw_data, row_var, rv, col_var, cv)?;
                    let panel_layer_aes = self.build_layer_aes(&panel_data)?;
                    for (i, _layer) in self.layers.iter().enumerate() {
                        for scale in scales.iter_mut() {
                            for a in Aesthetic::all() {
                                if a.family() == AestheticFamily::HorizontalPosition {
                                    if let Some(col) = panel_layer_aes[i].get(*a) {
                                        scale.append(col).ok();
                                    }
                                }
                            }
                        }
                    }
                }
                for s in scales.iter_mut() { s.fit()?; }
                result.push(scales);
            }
            Some(result)
        } else {
            None
        };

        // Build per-row Y scales (standard grid behavior, or COLS-only skips this)
        let row_y_scales: Option<Vec<Vec<Box<dyn Scale>>>> = if y_free_per_row {
            let mut result = Vec::new();
            for (_ri, rv) in row_values.iter().enumerate() {
                let mut scales = self.build_free_scales_for_family(AestheticFamily::VerticalPosition)?;
                for (_ci, cv) in col_values.iter().enumerate() {
                    let panel_data = self.subset_grid_panel(raw_data, row_var, rv, col_var, cv)?;
                    let panel_layer_aes = self.build_layer_aes(&panel_data)?;
                    for (i, _layer) in self.layers.iter().enumerate() {
                        for scale in scales.iter_mut() {
                            for a in Aesthetic::all() {
                                if a.family() == AestheticFamily::VerticalPosition {
                                    if let Some(col) = panel_layer_aes[i].get(*a) {
                                        scale.append(col).ok();
                                    }
                                }
                            }
                        }
                    }
                }
                for s in scales.iter_mut() { s.fit()?; }
                result.push(scales);
            }
            Some(result)
        } else {
            None
        };

        // Build per-column Y scales (fallback for COLS-only + free_y)
        let col_y_scales: Option<Vec<Vec<Box<dyn Scale>>>> = if y_free_per_col {
            let mut result = Vec::new();
            for (_ci, cv) in col_values.iter().enumerate() {
                let mut scales = self.build_free_scales_for_family(AestheticFamily::VerticalPosition)?;
                for (_ri, rv) in row_values.iter().enumerate() {
                    let panel_data = self.subset_grid_panel(raw_data, row_var, rv, col_var, cv)?;
                    let panel_layer_aes = self.build_layer_aes(&panel_data)?;
                    for (i, _layer) in self.layers.iter().enumerate() {
                        for scale in scales.iter_mut() {
                            for a in Aesthetic::all() {
                                if a.family() == AestheticFamily::VerticalPosition {
                                    if let Some(col) = panel_layer_aes[i].get(*a) {
                                        scale.append(col).ok();
                                    }
                                }
                            }
                        }
                    }
                }
                for s in scales.iter_mut() { s.fit()?; }
                result.push(scales);
            }
            Some(result)
        } else {
            None
        };

        // Render each panel
        for (ri, rv) in row_values.iter().enumerate() {
            for (ci, cv) in col_values.iter().enumerate() {
                let panel_idx = ri * num_grid_cols + ci;
                let panel_data = self.subset_grid_panel(raw_data, row_var, rv, col_var, cv)?;
                let panel_layer_aes = self.build_layer_aes(&panel_data)?;

                // Build composite scale refs for this panel
                let scale_refs = self.build_grid_panel_scale_refs(
                    ci, ri, &col_x_scales, &row_x_scales, &row_y_scales, &col_y_scales,
                );

                for (i, layer) in self.layers.iter().enumerate() {
                    let resolved = self.resolve_layer_with_scale_refs(layer, &panel_layer_aes[i], &scale_refs)?;
                    let mut geom_elements = layer.geometry.render(&resolved)?;
                    regions
                        .entry(RegionKey::panel(PlotRegion::DataArea, panel_idx))
                        .or_default()
                        .append(&mut geom_elements);
                }

                // Panel borders
                self.emit_panel_borders(
                    regions.entry(RegionKey::panel(PlotRegion::DataArea, panel_idx)).or_default()
                );

                // X-axis ticks: bottom row only (or all rows if x_free)
                let is_bottom_row = ri == num_grid_rows - 1;
                if is_bottom_row || x_free {
                    for scale in &scale_refs {
                        let (region, mut elements) = scale.render(self.theme)?;
                        if region == PlotRegion::XAxisGutter {
                            regions
                                .entry(RegionKey::panel(PlotRegion::XAxisGutter, panel_idx))
                                .or_default()
                                .append(&mut elements);
                        }
                    }
                }
            }

            // Row Y-axis gutter
            // When y_free_per_row: one Y scale per row (shared across columns in that row)
            // When y_free_per_col: one Y scale per column (each panel gets its own, rendered per-panel)
            if y_free_per_col {
                // Per-column Y: render Y gutter per panel
                for (ci2, _) in col_values.iter().enumerate() {
                    let panel_idx2 = ri * num_grid_cols + ci2;
                    if let Some(ref cys) = col_y_scales {
                        for scale in &cys[ci2] {
                            let (region, mut elements) = scale.render(self.theme)?;
                            if region == PlotRegion::YAxisGutter {
                                regions
                                    .entry(RegionKey::panel(PlotRegion::YAxisGutter, panel_idx2))
                                    .or_default()
                                    .append(&mut elements);
                            }
                        }
                    }
                }
            } else {
                let row_scales: &[Box<dyn Scale>] = if let Some(ref rys) = row_y_scales {
                    &rys[ri]
                } else {
                    &self.scales
                };
                for scale in row_scales {
                    let (region, mut elements) = scale.render(self.theme)?;
                    if region == PlotRegion::YAxisGutter {
                        regions
                            .entry(RegionKey::panel(PlotRegion::YAxisGutter, ri))
                            .or_default()
                            .append(&mut elements);
                    }
                }
            }

            // Row label (right side, rotated 90° to fit narrow strip)
            if row_var.is_some() {
                self.emit_facet_label_rotated(regions, RegionKey::panel(PlotRegion::FacetRowLabel, ri), rv, TextRotation::Cw90);
            }
        }

        // Column labels (top)
        if col_var.is_some() {
            for (ci, cv) in col_values.iter().enumerate() {
                self.emit_facet_label(regions, RegionKey::panel(PlotRegion::FacetColLabel, ci), cv);
            }
        }

        // Shared legend
        for scale in &self.scales {
            let (region, mut scale_elements) = scale.render(self.theme)?;
            if region == PlotRegion::Legend {
                regions
                    .entry(RegionKey::shared(PlotRegion::Legend))
                    .or_default()
                    .append(&mut scale_elements);
            }
        }

        // Labels
        self.emit_labels(regions, raw_data);

        Ok(())
    }

    /// Subset data for a grid panel based on row and column variable values.
    fn subset_grid_panel(
        &self,
        raw_data: &PlotData,
        row_var: Option<&str>,
        row_val: &str,
        col_var: Option<&str>,
        col_val: &str,
    ) -> Result<PlotData, GglangError> {
        let mut indices: Option<Vec<usize>> = None;

        if let Some(rv) = row_var {
            let col = raw_data.get(rv).ok_or_else(|| GglangError::Render {
                message: format!("Row facet column '{}' not found in data", rv),
            })?;
            let row_indices = col.indices_where_eq(row_val);
            indices = Some(row_indices);
        }
        if let Some(cv) = col_var {
            let col = raw_data.get(cv).ok_or_else(|| GglangError::Render {
                message: format!("Col facet column '{}' not found in data", cv),
            })?;
            let col_indices = col.indices_where_eq(col_val);
            indices = Some(match indices {
                Some(existing) => {
                    let col_set: HashSet<usize> = col_indices.into_iter().collect();
                    existing.into_iter().filter(|i| col_set.contains(i)).collect()
                }
                None => col_indices,
            });
        }

        match indices {
            Some(idx) => Ok(raw_data.subset(&idx)),
            None => Ok(raw_data.clone()),
        }
    }

    /// Compose scale refs for a wrap panel: shared axes from self.scales,
    /// free axes from the panel's free_scales vec.
    fn build_wrap_panel_scale_refs<'b>(
        &'b self,
        x_free: bool,
        y_free: bool,
        free_scales: &'b Option<Vec<Box<dyn Scale>>>,
    ) -> Vec<&'b dyn Scale> {
        if !x_free && !y_free {
            return self.scales.iter().map(|s| s.as_ref()).collect();
        }
        let mut refs: Vec<&dyn Scale> = Vec::new();
        let mut free_idx = 0;
        for s in &self.scales {
            let family = s.aesthetic_family();
            let is_free = (x_free && family == AestheticFamily::HorizontalPosition)
                || (y_free && family == AestheticFamily::VerticalPosition);
            if is_free {
                if let Some(ref fs) = free_scales {
                    if free_idx < fs.len() {
                        refs.push(fs[free_idx].as_ref());
                        free_idx += 1;
                    }
                }
            } else {
                refs.push(s.as_ref());
            }
        }
        refs
    }

    /// Build unfitted scale copies for a specific family.
    fn build_free_scales_for_family(&self, family: AestheticFamily) -> Result<Vec<Box<dyn Scale>>, GglangError> {
        Ok(self.scales.iter()
            .filter(|s| s.aesthetic_family() == family)
            .map(|s| s.clone_unfitted())
            .collect())
    }

    /// Build a composite scale set for a grid panel, referencing per-column X scales
    /// and per-row Y scales alongside shared scales for other families.
    fn build_grid_panel_scale_refs<'b>(
        &'b self,
        col_idx: usize,
        row_idx: usize,
        col_x_scales: &'b Option<Vec<Vec<Box<dyn Scale>>>>,
        row_x_scales: &'b Option<Vec<Vec<Box<dyn Scale>>>>,
        row_y_scales: &'b Option<Vec<Vec<Box<dyn Scale>>>>,
        col_y_scales: &'b Option<Vec<Vec<Box<dyn Scale>>>>,
    ) -> Vec<&'b dyn Scale> {
        let mut refs: Vec<&dyn Scale> = Vec::new();
        for s in &self.scales {
            let family = s.aesthetic_family();
            if family == AestheticFamily::HorizontalPosition {
                if let Some(ref cxs) = col_x_scales {
                    for cs in &cxs[col_idx] { refs.push(cs.as_ref()); }
                    continue;
                }
                if let Some(ref rxs) = row_x_scales {
                    for rs in &rxs[row_idx] { refs.push(rs.as_ref()); }
                    continue;
                }
            }
            if family == AestheticFamily::VerticalPosition {
                if let Some(ref rys) = row_y_scales {
                    for rs in &rys[row_idx] { refs.push(rs.as_ref()); }
                    continue;
                }
                if let Some(ref cys) = col_y_scales {
                    for cs in &cys[col_idx] { refs.push(cs.as_ref()); }
                    continue;
                }
            }
            refs.push(s.as_ref());
        }
        refs
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
                    .with_rotation(TextRotation::Ccw90)
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

/// Build a faceted wrap layout tree (replaces old faceted_plot_layout).
fn faceted_wrap_layout(
    num_panels: usize,
    facet_columns: Option<u32>,
    has_title: bool,
    has_caption: bool,
    has_legend: bool,
    y_free: bool,
    theme: &Theme,
) -> LayoutNode {
    let num_cols = facet_columns
        .map(|n| n as usize)
        .unwrap_or_else(|| (num_panels as f64).sqrt().ceil() as usize)
        .max(1);
    let num_rows = (num_panels + num_cols - 1) / num_cols;

    let mut grid_rows: Vec<(SizeSpec, LayoutNode)> = Vec::new();
    for row in 0..num_rows {
        let mut row_cells: Vec<(SizeSpec, LayoutNode)> = Vec::new();

        if !y_free {
            // Shared per-row y-axis gutter on the left
            let y_gutter_col = LayoutNode::Split {
                axis: SplitAxis::Vertical,
                children: vec![
                    (SizeSpec::Pixels(theme.facet_label_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
                    (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::YAxisGutter, row))),
                    (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
                ],
            };
            row_cells.push((SizeSpec::Pixels(theme.y_gutter_width), y_gutter_col));
        }

        for col in 0..num_cols {
            if col > 0 {
                row_cells.push((SizeSpec::Pixels(theme.facet_gap), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
            }
            let panel_idx = row * num_cols + col;
            if panel_idx >= num_panels {
                row_cells.push((SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
                continue;
            }

            let content_node = LayoutNode::Split {
                axis: SplitAxis::Vertical,
                children: vec![
                    (SizeSpec::Pixels(theme.facet_label_height), LayoutNode::Leaf(RegionKey::panel(PlotRegion::FacetLabel, panel_idx))),
                    (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::DataArea, panel_idx))),
                    (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::panel(PlotRegion::XAxisGutter, panel_idx))),
                ],
            };

            let panel_node = if y_free {
                // Each panel gets its own y-axis gutter on the left
                let y_gutter_node = LayoutNode::Split {
                    axis: SplitAxis::Vertical,
                    children: vec![
                        (SizeSpec::Pixels(theme.facet_label_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
                        (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::YAxisGutter, panel_idx))),
                        (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
                    ],
                };
                LayoutNode::Split {
                    axis: SplitAxis::Horizontal,
                    children: vec![
                        (SizeSpec::Pixels(theme.y_gutter_width), y_gutter_node),
                        (SizeSpec::Flex(1.0), content_node),
                    ],
                }
            } else {
                content_node
            };

            row_cells.push((SizeSpec::Flex(1.0), panel_node));
        }

        grid_rows.push((SizeSpec::Flex(1.0), LayoutNode::Split {
            axis: SplitAxis::Horizontal,
            children: row_cells,
        }));
    }

    let grid = LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: grid_rows,
    };

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
        (SizeSpec::Pixels(theme.y_axis_label_width), y_label_column),
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

/// Build a faceted grid layout tree.
///
/// ```text
/// Title
/// +-- Y-axis label | Grid body                              | Legend
///     Grid body:
///       Col labels (FacetColLabel per column, top)
///       For each row:
///         Y-axis gutter | DataArea cells... | FacetRowLabel (right)
///       X-axis label (shared, bottom)
/// ```
fn faceted_grid_layout(
    num_row_values: usize,
    num_col_values: usize,
    has_title: bool,
    has_caption: bool,
    has_legend: bool,
    theme: &Theme,
) -> LayoutNode {
    let has_col_labels = true; // always show labels in grid mode
    let has_row_labels = true;

    // Build grid body
    let mut grid_body_rows: Vec<(SizeSpec, LayoutNode)> = Vec::new();

    // Column labels row (top)
    if has_col_labels {
        let mut col_label_cells: Vec<(SizeSpec, LayoutNode)> = vec![
            // Spacer for Y-axis gutter width
            (SizeSpec::Pixels(theme.y_gutter_width), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
        ];
        for ci in 0..num_col_values {
            if ci > 0 {
                col_label_cells.push((SizeSpec::Pixels(theme.facet_gap), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
            }
            col_label_cells.push((SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::FacetColLabel, ci))));
        }
        if has_row_labels {
            col_label_cells.push((SizeSpec::Pixels(theme.facet_row_label_width), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
        }
        grid_body_rows.push((
            SizeSpec::Pixels(theme.facet_label_height),
            LayoutNode::Split { axis: SplitAxis::Horizontal, children: col_label_cells },
        ));
    }

    // Data rows
    for ri in 0..num_row_values {
        let y_gutter_col = LayoutNode::Split {
            axis: SplitAxis::Vertical,
            children: vec![
                (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::YAxisGutter, ri))),
                (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
            ],
        };

        let mut row_cells: Vec<(SizeSpec, LayoutNode)> = vec![
            (SizeSpec::Pixels(theme.y_gutter_width), y_gutter_col),
        ];
        for ci in 0..num_col_values {
            if ci > 0 {
                row_cells.push((SizeSpec::Pixels(theme.facet_gap), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))));
            }
            let panel_idx = ri * num_col_values + ci;
            row_cells.push((SizeSpec::Flex(1.0), LayoutNode::Split {
                axis: SplitAxis::Vertical,
                children: vec![
                    (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::DataArea, panel_idx))),
                    (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::panel(PlotRegion::XAxisGutter, panel_idx))),
                ],
            }));
        }
        if has_row_labels {
            let row_label = LayoutNode::Split {
                axis: SplitAxis::Vertical,
                children: vec![
                    (SizeSpec::Flex(1.0), LayoutNode::Leaf(RegionKey::panel(PlotRegion::FacetRowLabel, ri))),
                    (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::Spacer))),
                ],
            };
            row_cells.push((SizeSpec::Pixels(theme.facet_row_label_width), row_label));
        }

        grid_body_rows.push((
            SizeSpec::Flex(1.0),
            LayoutNode::Split { axis: SplitAxis::Horizontal, children: row_cells },
        ));
    }

    let grid_body = LayoutNode::Split {
        axis: SplitAxis::Vertical,
        children: grid_body_rows,
    };

    // Y-axis label on far left
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
            (SizeSpec::Flex(1.0), grid_body),
            (SizeSpec::Pixels(theme.x_gutter_height), LayoutNode::Leaf(RegionKey::shared(PlotRegion::XAxisGutter))),
        ],
    };

    let mut main_columns: Vec<(SizeSpec, LayoutNode)> = vec![
        (SizeSpec::Pixels(theme.y_axis_label_width), y_label_column),
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
        assert!(!output.regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
        let data_count = output.regions.get(&RegionKey::shared(PlotRegion::DataArea)).map_or(0, |v| v.len());
        assert!(data_count >= 3);
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
        let layer1 = Layer::new(
            Box::new(GeomPoint {}),
            vec![],
            HashMap::new(),
            Box::new(IdentityTransform {}),
            Box::new(IdentityTransform {}),
        );
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
        assert_eq!(points.len(), 6, "expected 6 points, got {}", points.len());
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
    fn facet_wrap_splits_data_into_panels() {
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
            .with_facet_spec(FacetSpec::Wrap {
                variable: "grp".into(),
                columns: None,
                scales: ScaleFreedom::Fixed,
            });

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]));
        data.insert("grp".into(), RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]));

        let output = bp.render(data).expect("faceted render should succeed");

        let panel0 = output.regions.get(&RegionKey::panel(PlotRegion::DataArea, 0))
            .expect("panel 0 should exist");
        let p0_points: Vec<_> = panel0.iter().filter(|e| matches!(e, Element::Point(_))).collect();
        assert_eq!(p0_points.len(), 2);

        let panel1 = output.regions.get(&RegionKey::panel(PlotRegion::DataArea, 1))
            .expect("panel 1 should exist");
        let p1_points: Vec<_> = panel1.iter().filter(|e| matches!(e, Element::Point(_))).collect();
        assert_eq!(p1_points.len(), 2);

        assert!(!output.regions.contains_key(&RegionKey::shared(PlotRegion::DataArea)));
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetLabel, 0)));
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetLabel, 1)));
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::YAxisGutter, 0)));
    }

    #[test]
    fn faceted_wrap_layout_resolves_all_panels() {
        let seg = layout_test_segment();
        let theme = Theme::default();
        let layout = faceted_wrap_layout(3, None, true, false, false, false, &theme);
        let regions = layout.resolve(&seg);

        for i in 0..3 {
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::DataArea, i)),
                "panel {} DataArea missing", i);
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::FacetLabel, i)),
                "panel {} FacetLabel missing", i);
        }
        assert!(regions.contains_key(&RegionKey::panel(PlotRegion::YAxisGutter, 0)));
        assert!(regions.contains_key(&RegionKey::panel(PlotRegion::YAxisGutter, 1)));
        assert!(regions.contains_key(&RegionKey::shared(PlotRegion::Title)));
    }

    #[test]
    fn facet_columns_override() {
        let seg = layout_test_segment();
        let theme = Theme::default();
        let layout = faceted_wrap_layout(4, Some(4), false, false, false, false, &theme);
        let regions = layout.resolve(&seg);
        for i in 0..4 {
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::DataArea, i)));
        }
    }

    #[test]
    fn facet_grid_rows_only() {
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
            .with_facet_spec(FacetSpec::Grid {
                row_var: Some("grp".into()),
                col_var: None,
                scales: ScaleFreedom::Fixed,
            });

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]));
        data.insert("grp".into(), RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]));

        let output = bp.render(data).expect("grid render should succeed");

        // 2 row values × 1 col → panels 0 and 1
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::DataArea, 0)));
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::DataArea, 1)));
        // Row labels
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetRowLabel, 0)));
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetRowLabel, 1)));
    }

    #[test]
    fn facet_grid_rows_and_cols() {
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
            .with_facet_spec(FacetSpec::Grid {
                row_var: Some("row".into()),
                col_var: Some("col".into()),
                scales: ScaleFreedom::Fixed,
            });

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0, 4.0]));
        data.insert("row".into(), RawColumn::StringArray(vec!["r1".into(), "r1".into(), "r2".into(), "r2".into()]));
        data.insert("col".into(), RawColumn::StringArray(vec!["c1".into(), "c2".into(), "c1".into(), "c2".into()]));

        let output = bp.render(data).expect("grid render should succeed");

        // 2 rows × 2 cols = 4 panels
        for i in 0..4 {
            assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::DataArea, i)),
                "panel {} missing", i);
        }
        // Col labels
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetColLabel, 0)));
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetColLabel, 1)));
        // Row labels
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetRowLabel, 0)));
        assert!(output.regions.contains_key(&RegionKey::panel(PlotRegion::FacetRowLabel, 1)));
    }

    #[test]
    fn facet_grid_layout_resolves() {
        let seg = layout_test_segment();
        let theme = Theme::default();
        let layout = faceted_grid_layout(2, 3, true, false, false, &theme);
        let regions = layout.resolve(&seg);

        // 2 rows × 3 cols = 6 panels
        for i in 0..6 {
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::DataArea, i)),
                "panel {} DataArea missing", i);
        }
        // Col labels
        for i in 0..3 {
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::FacetColLabel, i)),
                "col label {} missing", i);
        }
        // Row labels
        for i in 0..2 {
            assert!(regions.contains_key(&RegionKey::panel(PlotRegion::FacetRowLabel, i)),
                "row label {} missing", i);
        }
    }

    #[test]
    fn facet_wrap_free_y_produces_different_scales() {
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
            .with_facet_spec(FacetSpec::Wrap {
                variable: "grp".into(),
                columns: None,
                scales: ScaleFreedom::FreeY,
            });

        let mut data = PlotData::new();
        // Group "a" has y in [1, 2], group "b" has y in [100, 200]
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 1.0, 2.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 2.0, 100.0, 200.0]));
        data.insert("grp".into(), RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]));

        let output = bp.render(data).expect("free-y render should succeed");

        // Both panels should have 2 points
        let panel0_points: Vec<_> = output.regions.get(&RegionKey::panel(PlotRegion::DataArea, 0)).unwrap()
            .iter().filter_map(|e| match e { Element::Point(p) => Some(p), _ => None }).collect();
        let panel1_points: Vec<_> = output.regions.get(&RegionKey::panel(PlotRegion::DataArea, 1)).unwrap()
            .iter().filter_map(|e| match e { Element::Point(p) => Some(p), _ => None }).collect();
        assert_eq!(panel0_points.len(), 2);
        assert_eq!(panel1_points.len(), 2);

        // With free Y scales, each panel's points should span a similar NDC range
        // (since each panel is scaled to its own data range).
        let p0_y: Vec<f32> = panel0_points.iter().map(|p| match p.position[1] {
            Unit::NDC(v) => v, _ => panic!("expected NDC"),
        }).collect();
        let p1_y: Vec<f32> = panel1_points.iter().map(|p| match p.position[1] {
            Unit::NDC(v) => v, _ => panic!("expected NDC"),
        }).collect();

        // Both panels should have similar Y spreads (each fills its own scale)
        let p0_spread = (p0_y[1] - p0_y[0]).abs();
        let p1_spread = (p1_y[1] - p1_y[0]).abs();
        assert!(
            (p0_spread - p1_spread).abs() < 0.01,
            "free-Y panels should have similar Y spreads: p0={}, p1={}",
            p0_spread, p1_spread,
        );
    }

    #[test]
    fn facet_grid_cols_free_y_produces_different_scales() {
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
            .with_facet_spec(FacetSpec::Grid {
                row_var: None,
                col_var: Some("grp".into()),
                scales: ScaleFreedom::FreeY,
            });

        let mut data = PlotData::new();
        // Group "a" has y in [1, 2], group "b" has y in [100, 200]
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 1.0, 2.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 2.0, 100.0, 200.0]));
        data.insert("grp".into(), RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]));

        let output = bp.render(data).expect("grid cols free-y render should succeed");

        let panel0_points: Vec<_> = output.regions.get(&RegionKey::panel(PlotRegion::DataArea, 0)).unwrap()
            .iter().filter_map(|e| match e { Element::Point(p) => Some(p), _ => None }).collect();
        let panel1_points: Vec<_> = output.regions.get(&RegionKey::panel(PlotRegion::DataArea, 1)).unwrap()
            .iter().filter_map(|e| match e { Element::Point(p) => Some(p), _ => None }).collect();
        assert_eq!(panel0_points.len(), 2);
        assert_eq!(panel1_points.len(), 2);

        let p0_y: Vec<f32> = panel0_points.iter().map(|p| match p.position[1] {
            Unit::NDC(v) => v, _ => panic!("expected NDC"),
        }).collect();
        let p1_y: Vec<f32> = panel1_points.iter().map(|p| match p.position[1] {
            Unit::NDC(v) => v, _ => panic!("expected NDC"),
        }).collect();

        // With free Y on GRID COLS, each column/panel should have its own Y scale,
        // so both panels should span a similar NDC range despite very different data ranges.
        let p0_spread = (p0_y[1] - p0_y[0]).abs();
        let p1_spread = (p1_y[1] - p1_y[0]).abs();
        assert!(
            (p0_spread - p1_spread).abs() < 0.01,
            "grid cols free-Y panels should have similar Y spreads: p0={}, p1={}",
            p0_spread, p1_spread,
        );
    }
}
