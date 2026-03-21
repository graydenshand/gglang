use std::collections::HashMap;

use crate::aesthetic::{Aesthetic, AestheticFamily};
use crate::column::{AesData, MappedColumn, RawColumn, ResolvedData};
use crate::error::GglangError;
use crate::layout::Unit;
use crate::scale::{Axis, IdentityTransform, Scale, ScalePositionContinuous, StatTransform};
use crate::shape::{Element, PointData, PolylineData, Rectangle};

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
    fn render(&self, data: &ResolvedData) -> Result<Vec<Element>, GglangError>;

    /// The list of aesthetic families that may be used in this layer
    fn aesthetic_families(&self) -> Vec<AestheticFamily> {
        self.required_aesthetics()
            .iter()
            .chain(self.extra_aesthetics().iter())
            .map(|a| a.family())
            .collect()
    }

    /// Update scales using the aesthetic-keyed raw data for this layer.
    fn update_scales(
        &self,
        scales: &mut Vec<Box<dyn Scale>>,
        data: &AesData,
    ) -> Result<(), GglangError> {
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
                    scale.append(col)?;
                }
            }
        }
        Ok(())
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

    fn render(&self, data: &ResolvedData) -> Result<Vec<Element>, GglangError> {
        let x_mapped = match data.mapped.get(&Aesthetic::X).ok_or_else(|| GglangError::Render {
            message: "Missing required aesthetic X".to_string(),
        })? {
            MappedColumn::UnitArray(v) => v,
            _ => {
                return Err(GglangError::Render {
                    message: "Expected UnitArray from X position scale".to_string(),
                })
            }
        };
        let y_mapped = match data.mapped.get(&Aesthetic::Y).ok_or_else(|| GglangError::Render {
            message: "Missing required aesthetic Y".to_string(),
        })? {
            MappedColumn::UnitArray(v) => v,
            _ => {
                return Err(GglangError::Render {
                    message: "Expected UnitArray from Y position scale".to_string(),
                })
            }
        };
        let colors: Option<&Vec<[f32; 3]>> = match data.mapped.get(&Aesthetic::Color) {
            Some(MappedColumn::ColorArray(v)) => Some(v),
            Some(_) => {
                return Err(GglangError::Render {
                    message: "Expected ColorArray from color scale".to_string(),
                })
            }
            None => None,
        };

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
        Ok(points)
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

    fn render(&self, data: &ResolvedData) -> Result<Vec<Element>, GglangError> {
        let x_mapped = match data.mapped.get(&Aesthetic::X).ok_or_else(|| GglangError::Render {
            message: "Missing required aesthetic X".to_string(),
        })? {
            MappedColumn::UnitArray(v) => v,
            _ => {
                return Err(GglangError::Render {
                    message: "Expected UnitArray from X position scale".to_string(),
                })
            }
        };
        let y_mapped = match data.mapped.get(&Aesthetic::Y).ok_or_else(|| GglangError::Render {
            message: "Missing required aesthetic Y".to_string(),
        })? {
            MappedColumn::UnitArray(v) => v,
            _ => {
                return Err(GglangError::Render {
                    message: "Expected UnitArray from Y position scale".to_string(),
                })
            }
        };
        let colors: Option<&Vec<[f32; 3]>> = match data.mapped.get(&Aesthetic::Color) {
            Some(MappedColumn::ColorArray(v)) => Some(v),
            Some(_) => {
                return Err(GglangError::Render {
                    message: "Expected ColorArray from color scale".to_string(),
                })
            }
            None => None,
        };

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
        Ok(elements)
    }
}

/// Position adjustment strategy for bar charts.
#[derive(Debug, Clone)]
pub enum BarPosition {
    Stack,
    Dodge,
}

/// Compute the slope (NDC per raw unit) and intercept (NDC at raw=0) of the
/// linear Y scale by finding two distinct raw/mapped pairs.
/// Returns `(ndc_per_unit, y_zero_ndc)`.
fn linear_y_params(raw_y: &[f64], mapped_y: &[Unit]) -> (f32, f32) {
    for i in 0..raw_y.len() {
        for j in (i + 1)..raw_y.len() {
            let r1 = raw_y[i];
            let r2 = raw_y[j];
            if (r2 - r1).abs() < 1e-12 {
                continue;
            }
            let n1 = match mapped_y[i] {
                Unit::NDC(v) => v as f64,
                _ => continue,
            };
            let n2 = match mapped_y[j] {
                Unit::NDC(v) => v as f64,
                _ => continue,
            };
            let slope = (n2 - n1) / (r2 - r1);
            let intercept = n1 - slope * r1;
            return (slope as f32, intercept as f32);
        }
    }
    // Fallback: if all values are the same, assume unit slope from bottom
    (1.0, -1.0)
}

/// GeomBar renders rectangular bars — either from stat count or identity y values.
///
/// Required aesthetics: `X`
///
/// Extra aesthetics: `Y`, `Fill`, `Color`
pub struct GeomBar {
    pub position: BarPosition,
}

impl Geometry for GeomBar {
    fn required_aesthetics(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn extra_aesthetics(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y, Aesthetic::Fill, Aesthetic::Color]
    }

    fn render(&self, data: &ResolvedData) -> Result<Vec<Element>, GglangError> {
        let x_mapped = match data.mapped.get(&Aesthetic::X).ok_or_else(|| GglangError::Render {
            message: "Missing required aesthetic X".to_string(),
        })? {
            MappedColumn::UnitArray(v) => v,
            _ => {
                return Err(GglangError::Render {
                    message: "Expected UnitArray from X position scale".to_string(),
                })
            }
        };
        let y_mapped = match data.mapped.get(&Aesthetic::Y) {
            Some(MappedColumn::UnitArray(v)) => v.clone(),
            Some(_) => {
                return Err(GglangError::Render {
                    message: "Expected UnitArray from Y position scale".to_string(),
                })
            }
            None => {
                return Err(GglangError::Render {
                    message: "Missing Y aesthetic for GeomBar".to_string(),
                })
            }
        };

        let fill_colors: Option<&Vec<[f32; 3]>> = match data.mapped.get(&Aesthetic::Fill) {
            Some(MappedColumn::ColorArray(v)) => Some(v),
            _ => None,
        };
        let raw_fill: Option<&Vec<String>> = match data.raw.get(&Aesthetic::Fill) {
            Some(RawColumn::StringArray(v)) => Some(v),
            _ => None,
        };

        // Get raw Y values for computing proper bar heights (needed for stacking)
        let raw_y: Vec<f64> = match data.raw.get(&Aesthetic::Y) {
            Some(col) => col.as_f64().unwrap_or_default(),
            None => vec![],
        };

        let n = x_mapped.len();
        // Compute band_width from distinct x NDC positions
        let mut distinct_x: Vec<f32> = vec![];
        for u in x_mapped {
            if let Unit::NDC(v) = u {
                if !distinct_x.iter().any(|dv| (*dv - v).abs() < 1e-9) {
                    distinct_x.push(*v);
                }
            }
        }
        let n_categories = distinct_x.len().max(1);
        let band_width = 2.0 / n_categories as f32;
        let bar_width = band_width * 0.8; // 80% of band for padding

        let default_color = [0.35, 0.55, 0.75, 1.0]; // steel blue default

        // Compute y_zero_ndc and ndc_per_unit from the linear Y scale
        let (ndc_per_unit, y_zero_ndc) = if !raw_y.is_empty() && !y_mapped.is_empty() {
            linear_y_params(&raw_y, &y_mapped)
        } else {
            (1.0, -1.0)
        };

        // Build ordered x-position index for stable stacking keys.
        // Maps each distinct x NDC to an index, avoiding float hashing.
        let x_index = |x_ndc: f32| -> usize {
            distinct_x.iter().position(|v| (*v - x_ndc).abs() < 1e-6).unwrap_or(0)
        };

        let mut elements = vec![];

        match (&self.position, raw_fill) {
            (BarPosition::Dodge, Some(fill_vals)) => {
                let mut fill_groups: Vec<String> = vec![];
                for f in fill_vals {
                    if !fill_groups.contains(f) {
                        fill_groups.push(f.clone());
                    }
                }
                let n_groups = fill_groups.len().max(1);
                let sub_width = bar_width / n_groups as f32;

                for i in 0..n {
                    let x_ndc = match x_mapped[i] {
                        Unit::NDC(v) => v,
                        _ => continue,
                    };
                    let y_ndc = match y_mapped[i] {
                        Unit::NDC(v) => v,
                        _ => continue,
                    };

                    let group_idx = fill_groups.iter().position(|g| g == &fill_vals[i]).unwrap_or(0);
                    let group_offset = (group_idx as f32 - (n_groups as f32 - 1.0) / 2.0) * sub_width;
                    let bar_x = x_ndc + group_offset;

                    let bar_height = y_ndc - y_zero_ndc;
                    let bar_center_y = y_zero_ndc + bar_height / 2.0;

                    let color = fill_colors.map_or(default_color, |c| {
                        let [r, g, b] = c[i];
                        [r, g, b, 1.0]
                    });

                    elements.push(Element::Rect(Rectangle::new(
                        [Unit::NDC(bar_x), Unit::NDC(bar_center_y)],
                        Unit::NDC(sub_width),
                        Unit::NDC(bar_height),
                        color,
                    )));
                }
            }
            (BarPosition::Stack, Some(fill_vals)) => {
                let mut fill_groups: Vec<String> = vec![];
                for f in fill_vals {
                    if !fill_groups.contains(f) {
                        fill_groups.push(f.clone());
                    }
                }

                // Track cumulative NDC offset per x category index
                let mut x_offsets: Vec<f32> = vec![y_zero_ndc; n_categories];

                for fill_group in &fill_groups {
                    for i in 0..n {
                        if &fill_vals[i] != fill_group {
                            continue;
                        }
                        let x_ndc = match x_mapped[i] {
                            Unit::NDC(v) => v,
                            _ => continue,
                        };

                        let xi = x_index(x_ndc);
                        let bar_bottom = x_offsets[xi];
                        let segment_height = raw_y.get(i).copied().unwrap_or(0.0) as f32 * ndc_per_unit;
                        let bar_top = bar_bottom + segment_height;
                        let bar_center_y = bar_bottom + segment_height / 2.0;

                        x_offsets[xi] = bar_top;

                        let color = fill_colors.map_or(default_color, |c| {
                            let [r, g, b] = c[i];
                            [r, g, b, 1.0]
                        });

                        elements.push(Element::Rect(Rectangle::new(
                            [Unit::NDC(x_ndc), Unit::NDC(bar_center_y)],
                            Unit::NDC(bar_width),
                            Unit::NDC(segment_height),
                            color,
                        )));
                    }
                }
            }
            _ => {
                // No fill — simple bars
                for i in 0..n {
                    let x_ndc = match x_mapped[i] {
                        Unit::NDC(v) => v,
                        _ => continue,
                    };
                    let y_ndc = match y_mapped[i] {
                        Unit::NDC(v) => v,
                        _ => continue,
                    };

                    let bar_height = y_ndc - y_zero_ndc;
                    let bar_center_y = y_zero_ndc + bar_height / 2.0;

                    let color = fill_colors.map_or(default_color, |c| {
                        let [r, g, b] = c[i];
                        [r, g, b, 1.0]
                    });

                    elements.push(Element::Rect(Rectangle::new(
                        [Unit::NDC(x_ndc), Unit::NDC(bar_center_y)],
                        Unit::NDC(bar_width),
                        Unit::NDC(bar_height),
                        color,
                    )));
                }
            }
        }

        Ok(elements)
    }

    fn update_scales(
        &self,
        scales: &mut Vec<Box<dyn Scale>>,
        data: &AesData,
    ) -> Result<(), GglangError> {
        // If stat count produced Y data but no Y scale exists yet, create one
        if data.contains(Aesthetic::Y)
            && !scales.iter().any(|s| s.aesthetic_family() == AestheticFamily::VerticalPosition)
        {
            scales.push(Box::new(ScalePositionContinuous::new(Axis::Y)));
        }

        // Standard scale feeding for non-Y aesthetics
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
            if *aes == Aesthetic::Y {
                continue; // Handle Y separately below for stacking
            }
            if let Some(col) = data.get(*aes) {
                if let Some(scale) = family_scale_map.get_mut(&aes.family()) {
                    scale.append(col)?;
                }
            }
        }

        // For Y scale: if stacking with fill, compute per-x cumulative totals
        // and feed those to the scale so the domain covers the full stack height
        if let Some(y_scale) = family_scale_map.get_mut(&AestheticFamily::VerticalPosition) {
            if matches!(self.position, BarPosition::Stack) {
                if let (Some(y_col), Some(x_col)) = (data.get(Aesthetic::Y), data.get(Aesthetic::X)) {
                    let y_vals = y_col.as_f64().unwrap_or_default();
                    let x_strings: Vec<String> = match x_col {
                        RawColumn::StringArray(s) => s.clone(),
                        RawColumn::IntArray(v) => v.iter().map(|i| i.to_string()).collect(),
                        RawColumn::FloatArray(v) => v.iter().map(|f| f.to_string()).collect(),
                    };
                    // Sum y values per x category
                    let mut x_sums: Vec<(String, f64)> = vec![];
                    for (x, y) in x_strings.iter().zip(y_vals.iter()) {
                        if let Some(entry) = x_sums.iter_mut().find(|(k, _)| k == x) {
                            entry.1 += y;
                        } else {
                            x_sums.push((x.clone(), *y));
                        }
                    }
                    let max_vals: Vec<f64> = x_sums.iter().map(|(_, v)| *v).collect();
                    y_scale.append(&RawColumn::FloatArray(max_vals))?;
                } else if let Some(y_col) = data.get(Aesthetic::Y) {
                    y_scale.append(y_col)?;
                }
            } else if let Some(y_col) = data.get(Aesthetic::Y) {
                y_scale.append(y_col)?;
            }
            // Ensure y scale includes 0 in its domain for bar charts
            y_scale.append(&RawColumn::FloatArray(vec![0.0]))?;
        }

        Ok(())
    }
}
