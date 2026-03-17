use std::collections::HashMap;

use crate::aesthetic::{Aesthetic, AestheticFamily};
use crate::column::{AesData, MappedColumn, RawColumn, ResolvedData};
use crate::layout::Unit;
use crate::scale::{IdentityTransform, Scale, StatTransform};
use crate::shape::{Element, PointData, PolylineData};

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
