use crate::aesthetic::{parse_hex_color, Aesthetic, ConstantValue, Mapping};
use crate::ast::{AstAesthetic, GeomAttribute, GeometryType, LiteralValue, PositionAdjustment, Program, ScaleType, Statement};
use crate::error::GglangError;
use crate::geom::{BarPosition, GeomBar, GeomLine, GeomPoint};
use crate::plot::{Blueprint, Layer};
use crate::scale::{Axis, ScalePositionContinuous, ScalePositionDiscrete, IdentityTransform, StatCount};
use crate::theme::Theme;
use std::collections::HashMap;

fn ast_aesthetic_to_aesthetic(aes: &AstAesthetic) -> Aesthetic {
    match aes {
        AstAesthetic::X => Aesthetic::X,
        AstAesthetic::Y => Aesthetic::Y,
        AstAesthetic::Color => Aesthetic::Color,
        AstAesthetic::Fill => Aesthetic::Fill,
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
            Statement::Geom(geom_type, geom_attrs, position) => {
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
                if position.is_some() && !matches!(geom_type, GeometryType::Bar) {
                    return Err(GglangError::Compile {
                        message: "Position adjustment (STACK/DODGE) is only supported on GEOM BAR".to_string(),
                    });
                }
                let (geom, stat): (Box<dyn crate::geom::Geometry>, Box<dyn crate::scale::StatTransform>) = match geom_type {
                    GeometryType::Point => (Box::new(GeomPoint), Box::new(IdentityTransform)),
                    GeometryType::Line => (Box::new(GeomLine), Box::new(IdentityTransform)),
                    GeometryType::Bar => {
                        let bar_position = match position {
                            Some(PositionAdjustment::Dodge) => BarPosition::Dodge,
                            _ => BarPosition::Stack,
                        };
                        // If y is not mapped (neither in global mappings nor layer), use StatCount
                        let has_y = mapped_aesthetics.contains(&Aesthetic::Y)
                            || layer_mappings.iter().any(|m| m.aesthetic == Aesthetic::Y);
                        let stat: Box<dyn crate::scale::StatTransform> = if has_y {
                            Box::new(IdentityTransform)
                        } else {
                            Box::new(StatCount)
                        };
                        (Box::new(GeomBar { position: bar_position }), stat)
                    }
                };
                bp = bp.with_layer(Layer::new(
                    geom,
                    layer_mappings,
                    layer_constants,
                    stat,
                    Box::new(IdentityTransform),
                ));
            }
            Statement::Scale(ast_aes, scale_type) => {
                let aes = ast_aesthetic_to_aesthetic(ast_aes);
                let scale: Box<dyn crate::scale::Scale> = match (aes, scale_type) {
                    (Aesthetic::X, ScaleType::Continuous) => Box::new(ScalePositionContinuous::new(Axis::X)),
                    (Aesthetic::X, ScaleType::Discrete) => Box::new(ScalePositionDiscrete::new(Axis::X)),
                    (Aesthetic::Y, ScaleType::Continuous) => Box::new(ScalePositionContinuous::new(Axis::Y)),
                    (Aesthetic::Y, ScaleType::Discrete) => Box::new(ScalePositionDiscrete::new(Axis::Y)),
                    _ => return Err(GglangError::Compile {
                        message: format!("Unsupported SCALE combination for aesthetic '{}'", aes.name()),
                    }),
                };
                bp = bp.with_scale(scale);
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
    // Default scale creation is deferred to render() where actual data types are known.
    // Explicit SCALE statements added above take priority.

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
    fn end_to_end_string_x_auto_detects_discrete() {
        let source = "MAP x=:category, y=:value\nGEOM POINT";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert("category".into(), RawColumn::StringArray(vec!["a".into(), "b".into(), "c".into()]));
        data.insert("value".into(), RawColumn::FloatArray(vec![1.0, 2.0, 3.0]));

        let output = bp.render(data).unwrap();
        // Should render without error with discrete X axis
        let x_gutter = output.regions.get(&RegionKey::shared(PlotRegion::XAxisGutter));
        assert!(x_gutter.is_some());
    }

    #[test]
    fn end_to_end_explicit_scale_x_discrete_on_numeric() {
        let source = "MAP x=:year, y=:value\nGEOM POINT\nSCALE X DISCRETE";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert("year".into(), RawColumn::IntArray(vec![2021, 2022, 2023]));
        data.insert("value".into(), RawColumn::FloatArray(vec![10.0, 20.0, 30.0]));

        let output = bp.render(data).unwrap();
        // Explicit SCALE X DISCRETE on numeric column should work
        let x_gutter = output.regions.get(&RegionKey::shared(PlotRegion::XAxisGutter));
        assert!(x_gutter.is_some());
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

    #[test]
    fn end_to_end_bar_count() {
        let source = "MAP x=:category\nGEOM BAR";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert(
            "category".into(),
            RawColumn::StringArray(vec!["a".into(), "b".into(), "a".into(), "c".into()]),
        );

        let output = bp.render(data).unwrap();
        let bars: Vec<_> = output
            .regions
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .unwrap()
            .iter()
            .filter(|e| matches!(e, Element::Rect(_)))
            .collect();
        // 3 distinct categories → 3 bars
        assert_eq!(bars.len(), 3);
    }

    #[test]
    fn end_to_end_bar_identity() {
        let source = "MAP x=:category, y=:value\nGEOM BAR";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert(
            "category".into(),
            RawColumn::StringArray(vec!["a".into(), "b".into(), "c".into()]),
        );
        data.insert("value".into(), RawColumn::FloatArray(vec![10.0, 20.0, 30.0]));

        let output = bp.render(data).unwrap();
        let bars: Vec<_> = output
            .regions
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .unwrap()
            .iter()
            .filter(|e| matches!(e, Element::Rect(_)))
            .collect();
        assert_eq!(bars.len(), 3);
    }

    #[test]
    fn end_to_end_bar_dodge_with_fill() {
        let source = "MAP x=:x, y=:y, fill=:g\nGEOM BAR DODGE\nSCALE X DISCRETE";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert(
            "x".into(),
            RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]),
        );
        data.insert("y".into(), RawColumn::FloatArray(vec![10.0, 20.0, 30.0, 40.0]));
        data.insert(
            "g".into(),
            RawColumn::StringArray(vec!["g1".into(), "g2".into(), "g1".into(), "g2".into()]),
        );

        let output = bp.render(data).unwrap();
        let bars: Vec<_> = output
            .regions
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .unwrap()
            .iter()
            .filter(|e| matches!(e, Element::Rect(_)))
            .collect();
        // 2 x-categories × 2 fill groups = 4 bars
        assert_eq!(bars.len(), 4);
        // Should have a legend (fill scale)
        assert!(output.regions.contains_key(&RegionKey::shared(PlotRegion::Legend)));
    }

    #[test]
    fn end_to_end_bar_stacked_with_fill() {
        let source = "MAP x=:x, y=:y, fill=:g\nGEOM BAR\nSCALE X DISCRETE";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let mut bp = compile(&program, &theme).unwrap();

        let mut data = PlotData::new();
        data.insert(
            "x".into(),
            RawColumn::StringArray(vec!["a".into(), "a".into(), "b".into(), "b".into()]),
        );
        data.insert("y".into(), RawColumn::FloatArray(vec![10.0, 20.0, 30.0, 40.0]));
        data.insert(
            "g".into(),
            RawColumn::StringArray(vec!["g1".into(), "g2".into(), "g1".into(), "g2".into()]),
        );

        let output = bp.render(data).unwrap();
        let bars: Vec<_> = output
            .regions
            .get(&RegionKey::shared(PlotRegion::DataArea))
            .unwrap()
            .iter()
            .filter(|e| matches!(e, Element::Rect(_)))
            .collect();
        // 4 data rows → 4 stacked bar segments
        assert_eq!(bars.len(), 4);
    }

    #[test]
    fn compile_rejects_position_on_point() {
        let source = "GEOM POINT DODGE";
        let program = parse(source).unwrap();
        let theme = Theme::default();
        let result = compile(&program, &theme);
        match result {
            Err(e) => assert!(e.to_string().contains("Position adjustment")),
            Ok(_) => panic!("Expected compile error for GEOM POINT DODGE"),
        }
    }
}
