use crate::aesthetic::{Aesthetic, Mapping};
use crate::ast::{AstAesthetic, GeometryType, Program, Statement};
use crate::geom::{GeomLine, GeomPoint};
use crate::plot::{Blueprint, Layer};
use crate::scale::{default_scale_for, IdentityTransform};
use crate::theme::Theme;

pub fn compile<'a>(program: &Program, theme: &'a Theme) -> Result<Blueprint<'a>, String> {
    let mut bp = Blueprint::new(theme);
    let mut mappings: Vec<Mapping> = vec![];
    let mut mapped_aesthetics: Vec<Aesthetic> = vec![];

    for stmt in &program.statements {
        match stmt {
            Statement::Map(data_mappings) => {
                for dm in data_mappings {
                    let aesthetic = match dm.aesthetic {
                        AstAesthetic::X => Aesthetic::X,
                        AstAesthetic::Y => Aesthetic::Y,
                        AstAesthetic::Color => Aesthetic::Color,
                        AstAesthetic::Group => Aesthetic::Group,
                    };
                    mapped_aesthetics.push(aesthetic);
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
                GeometryType::Line => {
                    bp = bp.with_layer(Layer::new(
                        Box::new(GeomLine),
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
