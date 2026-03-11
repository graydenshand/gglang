use crate::ast::{AstAesthetic, GeometryType, Program, Statement};
use crate::plot::{
    Aesthetic, Axis, Blueprint, GeomPoint, IdentityTransform, Layer, Mapping,
    ScaleColorDiscrete, ScalePositionContinuous, Theme,
};

pub fn compile<'a>(program: &Program, theme: &'a Theme) -> Result<Blueprint<'a>, String> {
    let mut bp = Blueprint::new(theme);
    let mut mappings: Vec<Mapping> = vec![];
    let mut has_x = false;
    let mut has_y = false;
    let mut has_color = false;

    for stmt in &program.statements {
        match stmt {
            Statement::Map(data_mappings) => {
                for dm in data_mappings {
                    let aesthetic = match dm.aesthetic {
                        AstAesthetic::X => {
                            has_x = true;
                            Aesthetic::X
                        }
                        AstAesthetic::Y => {
                            has_y = true;
                            Aesthetic::Y
                        }
                        AstAesthetic::Color => {
                            has_color = true;
                            Aesthetic::Color
                        }
                    };
                    mappings.push(Mapping {
                        aesthetic,
                        variable: dm.column.clone(),
                    });
                }
            }
            Statement::Geom(geom_type) => match geom_type {
                GeometryType::Point => {
                    bp = bp.with_layer(Layer::new(
                        Box::new(GeomPoint),
                        mappings.clone(),
                        Box::new(IdentityTransform),
                        Box::new(IdentityTransform),
                    ));
                }
            },
            Statement::Title(s) => bp = bp.with_title(s.clone()),
            Statement::Caption(s) => bp = bp.with_caption(s.clone()),
            Statement::XLabel(s) => bp = bp.with_x_label(s.clone()),
            Statement::YLabel(s) => bp = bp.with_y_label(s.clone()),
        }
    }

    for m in mappings {
        bp = bp.with_mapping(m);
    }
    if has_x {
        bp = bp.with_scale(Box::new(ScalePositionContinuous::new(Axis::X)));
    }
    if has_y {
        bp = bp.with_scale(Box::new(ScalePositionContinuous::new(Axis::Y)));
    }
    if has_color {
        bp = bp.with_scale(Box::new(ScaleColorDiscrete::new()));
    }

    Ok(bp)
}
