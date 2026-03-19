use crate::aesthetic::{parse_hex_color, Aesthetic, ConstantValue, Mapping};
use crate::ast::{AstAesthetic, GeomAttribute, GeometryType, LiteralValue, Program, Statement};
use crate::error::GglangError;
use crate::geom::{GeomLine, GeomPoint};
use crate::plot::{Blueprint, Layer};
use crate::scale::{default_scale_for, IdentityTransform};
use crate::theme::Theme;
use std::collections::HashMap;

fn ast_aesthetic_to_aesthetic(aes: &AstAesthetic) -> Aesthetic {
    match aes {
        AstAesthetic::X => Aesthetic::X,
        AstAesthetic::Y => Aesthetic::Y,
        AstAesthetic::Color => Aesthetic::Color,
        AstAesthetic::Group => Aesthetic::Group,
    }
}

pub fn compile<'a>(program: &Program, theme: &'a Theme) -> Result<Blueprint<'a>, GglangError> {
    let mut bp = Blueprint::new(theme);
    let mut mappings: Vec<Mapping> = vec![];
    let mut mapped_aesthetics: Vec<Aesthetic> = vec![];

    for stmt in &program.statements {
        match stmt {
            Statement::Map(data_mappings) => {
                for dm in data_mappings {
                    let aesthetic = ast_aesthetic_to_aesthetic(&dm.aesthetic);
                    mapped_aesthetics.push(aesthetic);
                    mappings.push(Mapping {
                        aesthetic,
                        variable: dm.column.clone(),
                    });
                }
            }
            Statement::Geom(geom_type, geom_attrs) => {
                let mut layer_mappings: Vec<Mapping> = vec![];
                let mut layer_constants: HashMap<Aesthetic, ConstantValue> = HashMap::new();
                for attr in geom_attrs {
                    match attr {
                        GeomAttribute::Mapped(aes, col) => {
                            let aesthetic = ast_aesthetic_to_aesthetic(aes);
                            mapped_aesthetics.push(aesthetic);
                            layer_mappings.push(Mapping {
                                aesthetic,
                                variable: col.clone(),
                            });
                        }
                        GeomAttribute::Constant(aes, val) => {
                            let aesthetic = ast_aesthetic_to_aesthetic(aes);
                            let constant = match val {
                                LiteralValue::Str(s) => {
                                    ConstantValue::Color(
                                        parse_hex_color(s).map_err(|e| GglangError::Compile {
                                            message: e,
                                        })?,
                                    )
                                }
                                LiteralValue::Number(n) => ConstantValue::Float(*n),
                            };
                            layer_constants.insert(aesthetic, constant);
                        }
                    }
                }
                let geom: Box<dyn crate::geom::Geometry> = match geom_type {
                    GeometryType::Point => Box::new(GeomPoint),
                    GeometryType::Line => Box::new(GeomLine),
                };
                bp = bp.with_layer(Layer::new(
                    geom,
                    layer_mappings,
                    layer_constants,
                    Box::new(IdentityTransform),
                    Box::new(IdentityTransform),
                ));
            }
            Statement::Facet(spec) => {
                bp = bp.with_facet_spec(spec.clone());
            }
            Statement::Title(s) => bp = bp.with_title(s.clone()),
            Statement::Caption(s) => bp = bp.with_caption(s.clone()),
            Statement::XLabel(s) => bp = bp.with_x_label(s.clone()),
            Statement::YLabel(s) => bp = bp.with_y_label(s.clone()),
        }
    }

    for m in mappings {
        bp = bp.with_mapping(m);
    }
    for aes in &mapped_aesthetics {
        let family = aes.family();
        if !bp.has_scale_for_family(family) {
            if let Some(scale) = default_scale_for(aes) {
                bp = bp.with_scale(scale);
            }
        }
    }

    Ok(bp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parse;
    use crate::column::{PlotData, RawColumn};
    use crate::layout::{PlotRegion, RegionKey};
    use crate::shape::Element;

    #[test]
    fn end_to_end_constant_color() {
        let source = "MAP x=:x, y=:y\nGEOM POINT { color=\"#FF0000\" }";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0]));

        let output = bp.render(data).unwrap();
        // Should render points with no legend (constant color, no color scale)
        assert!(!output.regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
        let points: Vec<_> = output
            .regions
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                Element::Point(p) => Some(p.color),
                _ => None,
            })
            .collect();
        assert_eq!(points.len(), 3);
        // #FF0000 → [1.0, 0.0, 0.0]
        for c in &points {
            assert!((c[0] - 1.0).abs() < 1e-5);
            assert!(c[1].abs() < 1e-5);
            assert!(c[2].abs() < 1e-5);
        }
    }

    #[test]
    fn end_to_end_per_layer_mapping_override() {
        let source = "MAP x=:x, y=:a\nGEOM POINT\nGEOM LINE { y=:b, color=\"#0000FF\" }";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0]));
        data.insert("a".into(), RawColumn::FloatArray(vec![10.0, 20.0, 30.0]));
        data.insert("b".into(), RawColumn::FloatArray(vec![100.0, 200.0, 300.0]));

        let output = bp.render(data).unwrap();
        let data_elements = output.regions.get(&RegionKey::shared(PlotRegion::DataArea)).unwrap();
        // Should have 3 points (from GEOM POINT) + 1 polyline (from GEOM LINE)
        let point_count = data_elements
            .iter()
            .filter(|e| matches!(e, Element::Point(_)))
            .count();
        let polyline_count = data_elements
            .iter()
            .filter(|e| matches!(e, Element::Polyline(_)))
            .count();
        assert_eq!(point_count, 3);
        assert_eq!(polyline_count, 1);
    }

    #[test]
    fn end_to_end_no_attributes_backward_compat() {
        let source = "MAP x=:x, y=:y\nGEOM POINT";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert("x".into(), RawColumn::FloatArray(vec![1.0, 2.0]));
        data.insert("y".into(), RawColumn::FloatArray(vec![3.0, 4.0]));

        let output = bp.render(data).unwrap();
        let data_count = output
            .regions
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .map_or(0, |v| v.len());
        assert!(data_count >= 2);
    }
}
