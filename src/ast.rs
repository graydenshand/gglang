use pest::Parser;
use pest_derive::Parser;

use crate::error::GglangError;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct GGCParser;

#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScaleFreedom {
    Fixed,
    FreeX,
    FreeY,
    Free,
}

#[derive(Debug, Clone)]
pub enum FacetSpec {
    Wrap {
        variable: String,
        columns: Option<u32>,
        scales: ScaleFreedom,
    },
    Grid {
        row_var: Option<String>,
        col_var: Option<String>,
        scales: ScaleFreedom,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScaleType {
    Continuous,
    Discrete,
    Log,
}

#[derive(Debug)]
pub enum Statement {
    Map(Vec<DataMapping>),
    Geom(GeometryType, Vec<GeomAttribute>, Option<usize>, Option<PositionAdjustment>),
    Scale(AstAesthetic, ScaleType),
    Facet(FacetSpec),
    Title(String),
    Caption(String),
    XLabel(String),
    YLabel(String),
    Coord(CoordType),
    Theme(ThemeStatement),
}

#[derive(Debug)]
pub enum ThemeStatement {
    Inline(Vec<ThemeOverride>),
    File(String),
}

#[derive(Debug)]
pub struct ThemeOverride {
    pub key: String,
    pub value: LiteralValue,
}

#[derive(Debug)]
pub enum LiteralValue {
    Str(String),
    Number(f64),
}

#[derive(Debug)]
pub enum GeomAttribute {
    Mapped(AstAesthetic, String),
    Constant(AstAesthetic, LiteralValue),
}

#[derive(Debug)]
pub struct DataMapping {
    pub column: String,
    pub aesthetic: AstAesthetic,
}

#[derive(Debug)]
pub enum AstAesthetic {
    X,
    Y,
    Color,
    Fill,
    Group,
    Alpha,
    Label,
    Shape,
}

#[derive(Debug)]
pub enum GeometryType {
    Point,
    Line,
    Bar,
    Text,
    Histogram,
}

#[derive(Debug, Clone)]
pub enum PositionAdjustment {
    Stack,
    Dodge,
}

#[derive(Debug, Clone)]
pub enum CoordType {
    Cartesian,
    Polar { start: f64 },
}

fn parse_data_reference(pair: pest::iterators::Pair<Rule>) -> Result<String, GglangError> {
    Ok(pair
        .into_inner()
        .next()
        .ok_or_else(|| GglangError::Parse {
            message: "Expected identifier token in data_reference".to_string(),
        })?
        .as_str()
        .to_string())
}

fn parse_facet_scales(pair: pest::iterators::Pair<Rule>) -> Result<ScaleFreedom, GglangError> {
    let inner = pair.into_inner().next().ok_or_else(|| GglangError::Parse {
        message: "Expected scale freedom token in facet_scales".to_string(),
    })?;
    Ok(match inner.as_rule() {
        Rule::free_axis => {
            let axis = inner.as_str().trim();
            if axis.ends_with('X') {
                ScaleFreedom::FreeX
            } else {
                ScaleFreedom::FreeY
            }
        }
        Rule::free_both => ScaleFreedom::Free,
        Rule::fixed_scales => ScaleFreedom::Fixed,
        _ => unreachable!(),
    })
}

fn parse_theme_attributes(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<ThemeOverride>, GglangError> {
    let mut overrides = vec![];
    for attr_pair in pair.into_inner() {
        if attr_pair.as_rule() == Rule::theme_attribute {
            let mut inner = attr_pair.into_inner();
            let key = inner
                .next()
                .ok_or_else(|| GglangError::Parse {
                    message: "Expected key in theme_attribute".to_string(),
                })?
                .as_str()
                .to_string();
            let val_pair = inner.next().ok_or_else(|| GglangError::Parse {
                message: "Expected value in theme_attribute".to_string(),
            })?;
            let val_inner = val_pair.into_inner().next().ok_or_else(|| GglangError::Parse {
                message: "Expected value type token in theme_value".to_string(),
            })?;
            let value = match val_inner.as_rule() {
                Rule::string_literal => {
                    let s = val_inner.as_str();
                    LiteralValue::Str(s[1..s.len() - 1].to_string())
                }
                Rule::number => {
                    let n: f64 = val_inner.as_str().parse().map_err(|_| GglangError::Parse {
                        message: format!("Invalid number in theme attribute: {}", val_inner.as_str()),
                    })?;
                    LiteralValue::Number(n)
                }
                _ => unreachable!(),
            };
            overrides.push(ThemeOverride { key, value });
        }
    }
    Ok(overrides)
}

pub fn parse_theme_file(source: &str) -> Result<Vec<ThemeOverride>, GglangError> {
    let pairs =
        GGCParser::parse(Rule::theme_file_contents, source).map_err(|e| GglangError::Parse {
            message: e.to_string(),
        })?;
    let pair = pairs.into_iter().next().ok_or_else(|| GglangError::Parse {
        message: "Empty theme file".to_string(),
    })?;
    parse_theme_attributes(pair)
}

pub fn parse(source: &str) -> Result<Program, GglangError> {
    let pairs = GGCParser::parse(Rule::program, source).map_err(|e| GglangError::Parse {
        message: e.to_string(),
    })?;

    let pair = pairs.into_iter().next().ok_or_else(|| GglangError::Parse {
        message: "Empty program".to_string(),
    })?;

    let mut statements = vec![];

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::statement => {
                let stmt_inner = inner_pair
                    .into_inner()
                    .next()
                    .ok_or_else(|| GglangError::Parse {
                        message: "Expected statement body".to_string(),
                    })?;
                match stmt_inner.as_rule() {
                    Rule::map_statement => {
                        let mut mappings = vec![];
                        for pair in stmt_inner.into_inner() {
                            if pair.as_rule() == Rule::data_mappings {
                                for mapping_pair in pair.into_inner() {
                                    if mapping_pair.as_rule() == Rule::data_mapping {
                                        let mut inner = mapping_pair.into_inner();
                                        let aes_str = inner
                                            .next()
                                            .ok_or_else(|| GglangError::Parse {
                                                message: "Expected aesthetic name in data_mapping"
                                                    .to_string(),
                                            })?
                                            .as_str();
                                        let data_ref =
                                            inner.next().ok_or_else(|| GglangError::Parse {
                                                message:
                                                    "Expected data reference in data_mapping"
                                                        .to_string(),
                                            })?;
                                        let column = data_ref
                                            .into_inner()
                                            .next()
                                            .ok_or_else(|| GglangError::Parse {
                                                message:
                                                    "Expected identifier in data_reference"
                                                        .to_string(),
                                            })?
                                            .as_str()
                                            .to_string();
                                        let aesthetic = match aes_str {
                                            "x" => AstAesthetic::X,
                                            "y" => AstAesthetic::Y,
                                            "color" => AstAesthetic::Color,
                                            "fill" => AstAesthetic::Fill,
                                            "group" => AstAesthetic::Group,
                                            "alpha" => AstAesthetic::Alpha,
                                            "label" => AstAesthetic::Label,
                                            "shape" => AstAesthetic::Shape,
                                            other => {
                                                return Err(GglangError::Parse {
                                                    message: format!(
                                                        "Unsupported aesthetic: {}",
                                                        other
                                                    ),
                                                })
                                            }
                                        };
                                        mappings.push(DataMapping { column, aesthetic });
                                    }
                                }
                            }
                        }
                        statements.push(Statement::Map(mappings));
                    }
                    Rule::geom_statement => {
                        let mut inner = stmt_inner.into_inner();
                        let geom_type_pair =
                            inner.next().ok_or_else(|| GglangError::Parse {
                                message: "Expected geometry type in geom_statement".to_string(),
                            })?;
                        let geom_type = match geom_type_pair.as_str() {
                            "POINT" => GeometryType::Point,
                            "LINE" => GeometryType::Line,
                            "BAR" => GeometryType::Bar,
                            "TEXT" => GeometryType::Text,
                            "HISTOGRAM" => GeometryType::Histogram,
                            other => {
                                return Err(GglangError::Parse {
                                    message: format!("Unsupported geometry: {}", other),
                                })
                            }
                        };
                        let mut attrs = vec![];
                        let mut bins: Option<usize> = None;
                        let mut position = None;
                        for pair in inner {
                            if pair.as_rule() == Rule::position_adjustment {
                                position = Some(match pair.as_str() {
                                    "STACK" => PositionAdjustment::Stack,
                                    "DODGE" => PositionAdjustment::Dodge,
                                    _ => unreachable!(),
                                });
                            } else if pair.as_rule() == Rule::bins_modifier {
                                let n: usize = pair
                                    .into_inner()
                                    .next()
                                    .ok_or_else(|| GglangError::Parse {
                                        message: "Expected integer in BINS modifier".to_string(),
                                    })?
                                    .as_str()
                                    .parse()
                                    .map_err(|_| GglangError::Parse {
                                        message: "Invalid integer in BINS modifier".to_string(),
                                    })?;
                                bins = Some(n);
                            } else if pair.as_rule() == Rule::geom_attributes {
                                for attr_pair in pair.into_inner() {
                                    if attr_pair.as_rule() == Rule::geom_attribute {
                                        let mut attr_inner = attr_pair.into_inner();
                                        let aes_str = attr_inner
                                            .next()
                                            .ok_or_else(|| GglangError::Parse {
                                                message: "Expected aesthetic name in geom_attribute"
                                                    .to_string(),
                                            })?
                                            .as_str();
                                        let aes = match aes_str {
                                            "x" => AstAesthetic::X,
                                            "y" => AstAesthetic::Y,
                                            "color" => AstAesthetic::Color,
                                            "fill" => AstAesthetic::Fill,
                                            "group" => AstAesthetic::Group,
                                            "alpha" => AstAesthetic::Alpha,
                                            "label" => AstAesthetic::Label,
                                            "shape" => AstAesthetic::Shape,
                                            other => {
                                                return Err(GglangError::Parse {
                                                    message: format!(
                                                        "Unsupported aesthetic: {}",
                                                        other
                                                    ),
                                                })
                                            }
                                        };
                                        let val_pair =
                                            attr_inner.next().ok_or_else(|| GglangError::Parse {
                                                message: "Expected value in geom_attribute"
                                                    .to_string(),
                                            })?;
                                        let val_inner =
                                            val_pair.into_inner().next().ok_or_else(|| {
                                                GglangError::Parse {
                                                    message: "Expected value type token"
                                                        .to_string(),
                                                }
                                            })?;
                                        let attr = match val_inner.as_rule() {
                                            Rule::data_reference => {
                                                let col = val_inner
                                                    .into_inner()
                                                    .next()
                                                    .ok_or_else(|| GglangError::Parse {
                                                        message: "Expected identifier in data_reference".to_string(),
                                                    })?
                                                    .as_str()
                                                    .to_string();
                                                GeomAttribute::Mapped(aes, col)
                                            }
                                            Rule::string_literal => {
                                                let s = val_inner.as_str();
                                                GeomAttribute::Constant(
                                                    aes,
                                                    LiteralValue::Str(
                                                        s[1..s.len() - 1].to_string(),
                                                    ),
                                                )
                                            }
                                            Rule::number => {
                                                let n: f64 =
                                                    val_inner.as_str().parse().map_err(|_| {
                                                        GglangError::Parse {
                                                            message: format!(
                                                                "Invalid number: {}",
                                                                val_inner.as_str()
                                                            ),
                                                        }
                                                    })?;
                                                GeomAttribute::Constant(
                                                    aes,
                                                    LiteralValue::Number(n),
                                                )
                                            }
                                            _ => unreachable!(),
                                        };
                                        attrs.push(attr);
                                    }
                                }
                            }
                        }
                        statements.push(Statement::Geom(geom_type, attrs, bins, position));
                    }
                    Rule::scale_statement => {
                        let mut inner = stmt_inner.into_inner();
                        let target = inner
                            .next()
                            .ok_or_else(|| GglangError::Parse {
                                message: "Expected scale target in scale_statement".to_string(),
                            })?
                            .as_str();
                        let aes = match target {
                            "X" => AstAesthetic::X,
                            "Y" => AstAesthetic::Y,
                            "COLOR" => AstAesthetic::Color,
                            "FILL" => AstAesthetic::Fill,
                            other => {
                                return Err(GglangError::Parse {
                                    message: format!("Unsupported scale target: {}", other),
                                })
                            }
                        };
                        let type_str = inner
                            .next()
                            .ok_or_else(|| GglangError::Parse {
                                message: "Expected scale type in scale_statement".to_string(),
                            })?
                            .as_str();
                        let scale_type = match type_str {
                            "CONTINUOUS" => ScaleType::Continuous,
                            "DISCRETE" => ScaleType::Discrete,
                            "LOG" => ScaleType::Log,
                            other => {
                                return Err(GglangError::Parse {
                                    message: format!("Unsupported scale type: {}", other),
                                })
                            }
                        };
                        statements.push(Statement::Scale(aes, scale_type));
                    }
                    Rule::facet_statement => {
                        let facet_inner =
                            stmt_inner.into_inner().next().ok_or_else(|| GglangError::Parse {
                                message: "Expected facet type in facet_statement".to_string(),
                            })?;
                        let spec = match facet_inner.as_rule() {
                            Rule::facet_wrap => {
                                let mut inner = facet_inner.into_inner();
                                let variable = parse_data_reference(inner.next().ok_or_else(
                                    || GglangError::Parse {
                                        message: "Expected variable in facet_wrap".to_string(),
                                    },
                                )?)?;
                                let mut columns = None;
                                let mut scales = ScaleFreedom::Fixed;
                                for pair in inner {
                                    match pair.as_rule() {
                                        Rule::facet_columns => {
                                            let n: u32 = pair
                                                .into_inner()
                                                .next()
                                                .ok_or_else(|| GglangError::Parse {
                                                    message: "Expected column count in facet_columns".to_string(),
                                                })?
                                                .as_str()
                                                .parse()
                                                .map_err(|_| GglangError::Parse {
                                                    message: "Invalid column count in FACET WRAP COLUMNS".to_string(),
                                                })?;
                                            columns = Some(n);
                                        }
                                        Rule::facet_scales => {
                                            scales = parse_facet_scales(pair)?;
                                        }
                                        _ => {}
                                    }
                                }
                                FacetSpec::Wrap {
                                    variable,
                                    columns,
                                    scales,
                                }
                            }
                            Rule::facet_grid => {
                                let mut inner = facet_inner.into_inner();
                                let grid_spec =
                                    inner.next().ok_or_else(|| GglangError::Parse {
                                        message: "Expected grid spec in facet_grid".to_string(),
                                    })?;
                                let (row_var, col_var) = match grid_spec.as_rule() {
                                    Rule::facet_rows_cols => {
                                        let mut gi = grid_spec.into_inner();
                                        let row = parse_data_reference(gi.next().ok_or_else(
                                            || GglangError::Parse {
                                                message: "Expected row variable in FACET GRID ROWS COLS".to_string(),
                                            },
                                        )?)?;
                                        let col = parse_data_reference(gi.next().ok_or_else(
                                            || GglangError::Parse {
                                                message: "Expected col variable in FACET GRID ROWS COLS".to_string(),
                                            },
                                        )?)?;
                                        (Some(row), Some(col))
                                    }
                                    Rule::facet_rows_only => {
                                        let mut gi = grid_spec.into_inner();
                                        let row = parse_data_reference(gi.next().ok_or_else(
                                            || GglangError::Parse {
                                                message: "Expected row variable in FACET GRID ROWS".to_string(),
                                            },
                                        )?)?;
                                        (Some(row), None)
                                    }
                                    Rule::facet_cols_only => {
                                        let mut gi = grid_spec.into_inner();
                                        let col = parse_data_reference(gi.next().ok_or_else(
                                            || GglangError::Parse {
                                                message: "Expected col variable in FACET GRID COLS".to_string(),
                                            },
                                        )?)?;
                                        (None, Some(col))
                                    }
                                    _ => unreachable!(),
                                };
                                let mut scales = ScaleFreedom::Fixed;
                                for pair in inner {
                                    if pair.as_rule() == Rule::facet_scales {
                                        scales = parse_facet_scales(pair)?;
                                    }
                                }
                                FacetSpec::Grid {
                                    row_var,
                                    col_var,
                                    scales,
                                }
                            }
                            _ => unreachable!(),
                        };
                        statements.push(Statement::Facet(spec));
                    }
                    Rule::coord_statement => {
                        let mut inner = stmt_inner.into_inner();
                        let coord_type_str = inner
                            .next()
                            .ok_or_else(|| GglangError::Parse {
                                message: "Expected coord type in coord_statement".to_string(),
                            })?
                            .as_str();
                        let coord = match coord_type_str {
                            "CARTESIAN" => CoordType::Cartesian,
                            "POLAR" => {
                                let mut start = 0.0;
                                if let Some(start_pair) = inner.next() {
                                    if start_pair.as_rule() == Rule::coord_start {
                                        let n: f64 = start_pair
                                            .into_inner()
                                            .next()
                                            .ok_or_else(|| GglangError::Parse {
                                                message: "Expected number in COORD POLAR START".to_string(),
                                            })?
                                            .as_str()
                                            .parse()
                                            .map_err(|_| GglangError::Parse {
                                                message: "Invalid number in COORD POLAR START".to_string(),
                                            })?;
                                        start = n;
                                    }
                                }
                                CoordType::Polar { start }
                            }
                            other => {
                                return Err(GglangError::Parse {
                                    message: format!("Unsupported coord type: {}", other),
                                })
                            }
                        };
                        statements.push(Statement::Coord(coord));
                    }
                    Rule::title_statement => {
                        let s = stmt_inner
                            .into_inner()
                            .next()
                            .ok_or_else(|| GglangError::Parse {
                                message: "Expected string in title_statement".to_string(),
                            })?
                            .as_str();
                        statements.push(Statement::Title(s[1..s.len() - 1].to_string()));
                    }
                    Rule::caption_statement => {
                        let s = stmt_inner
                            .into_inner()
                            .next()
                            .ok_or_else(|| GglangError::Parse {
                                message: "Expected string in caption_statement".to_string(),
                            })?
                            .as_str();
                        statements.push(Statement::Caption(s[1..s.len() - 1].to_string()));
                    }
                    Rule::xlabel_statement => {
                        let s = stmt_inner
                            .into_inner()
                            .next()
                            .ok_or_else(|| GglangError::Parse {
                                message: "Expected string in xlabel_statement".to_string(),
                            })?
                            .as_str();
                        statements.push(Statement::XLabel(s[1..s.len() - 1].to_string()));
                    }
                    Rule::ylabel_statement => {
                        let s = stmt_inner
                            .into_inner()
                            .next()
                            .ok_or_else(|| GglangError::Parse {
                                message: "Expected string in ylabel_statement".to_string(),
                            })?
                            .as_str();
                        statements.push(Statement::YLabel(s[1..s.len() - 1].to_string()));
                    }
                    Rule::theme_statement => {
                        let inner = stmt_inner
                            .into_inner()
                            .next()
                            .ok_or_else(|| GglangError::Parse {
                                message: "Expected theme body in theme_statement".to_string(),
                            })?;
                        let ts = match inner.as_rule() {
                            Rule::theme_file => {
                                let s = inner
                                    .into_inner()
                                    .next()
                                    .ok_or_else(|| GglangError::Parse {
                                        message: "Expected path in THEME FILE".to_string(),
                                    })?
                                    .as_str();
                                ThemeStatement::File(s[1..s.len() - 1].to_string())
                            }
                            Rule::theme_inline => {
                                let overrides = parse_theme_attributes(inner)?;
                                ThemeStatement::Inline(overrides)
                            }
                            _ => unreachable!(),
                        };
                        statements.push(Statement::Theme(ts));
                    }
                    _ => {}
                }
            }
            Rule::EOI => break,
            _ => unreachable!(),
        }
    }

    Ok(Program { statements })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let source = "MAP x=:x, y=:y\nGEOM POINT";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 2);

        match &program.statements[0] {
            Statement::Map(mappings) => {
                assert_eq!(mappings.len(), 2);
                assert_eq!(mappings[0].column, "x");
                assert!(matches!(mappings[0].aesthetic, AstAesthetic::X));
                assert_eq!(mappings[1].column, "y");
                assert!(matches!(mappings[1].aesthetic, AstAesthetic::Y));
            }
            _ => panic!("Expected Map statement"),
        }

        match &program.statements[1] {
            Statement::Geom(GeometryType::Point, attrs, _, _) => assert!(attrs.is_empty()),
            _ => panic!("Expected Geom Point statement"),
        }
    }

    #[test]
    fn test_parse_color_aesthetic() {
        let source = "MAP x=:x, y=:y, color=:species\nGEOM POINT";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 2);

        match &program.statements[0] {
            Statement::Map(mappings) => {
                assert_eq!(mappings.len(), 3);
                assert_eq!(mappings[2].column, "species");
                assert!(matches!(mappings[2].aesthetic, AstAesthetic::Color));
            }
            _ => panic!("Expected Map statement"),
        }
    }

    #[test]
    fn test_parse_geom_line_with_group() {
        let source = "MAP x=:day, y=:price, group=:ticker\nGEOM LINE";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 2);

        match &program.statements[0] {
            Statement::Map(mappings) => {
                assert_eq!(mappings.len(), 3);
                assert_eq!(mappings[0].column, "day");
                assert!(matches!(mappings[0].aesthetic, AstAesthetic::X));
                assert_eq!(mappings[1].column, "price");
                assert!(matches!(mappings[1].aesthetic, AstAesthetic::Y));
                assert_eq!(mappings[2].column, "ticker");
                assert!(matches!(mappings[2].aesthetic, AstAesthetic::Group));
            }
            _ => panic!("Expected Map statement"),
        }

        match &program.statements[1] {
            Statement::Geom(GeometryType::Line, attrs, _, _) => assert!(attrs.is_empty()),
            _ => panic!("Expected Geom Line statement"),
        }
    }

    #[test]
    fn test_parse_labels() {
        let source = "MAP x=:x, y=:y\nGEOM POINT\nTITLE \"My Plot\"\nXLABEL \"X Axis\"\nYLABEL \"Y Axis\"\nCAPTION \"Source: data\"";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 6);

        match &program.statements[2] {
            Statement::Title(s) => assert_eq!(s, "My Plot"),
            _ => panic!("Expected Title statement"),
        }
        match &program.statements[3] {
            Statement::XLabel(s) => assert_eq!(s, "X Axis"),
            _ => panic!("Expected XLabel statement"),
        }
        match &program.statements[4] {
            Statement::YLabel(s) => assert_eq!(s, "Y Axis"),
            _ => panic!("Expected YLabel statement"),
        }
        match &program.statements[5] {
            Statement::Caption(s) => assert_eq!(s, "Source: data"),
            _ => panic!("Expected Caption statement"),
        }
    }

    #[test]
    fn test_parse_geom_with_constant_color() {
        let source = "MAP x=:x, y=:y\nGEOM POINT { color=\"#FF0000\" }";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 2);
        match &program.statements[1] {
            Statement::Geom(GeometryType::Point, attrs, _, _) => {
                assert_eq!(attrs.len(), 1);
                match &attrs[0] {
                    GeomAttribute::Constant(AstAesthetic::Color, LiteralValue::Str(s)) => {
                        assert_eq!(s, "#FF0000");
                    }
                    _ => panic!("Expected constant color attribute"),
                }
            }
            _ => panic!("Expected Geom Point"),
        }
    }

    #[test]
    fn test_parse_geom_with_per_layer_mapping() {
        let source = "MAP x=:year, y=:a\nGEOM POINT { y=:b }";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 2);
        match &program.statements[1] {
            Statement::Geom(GeometryType::Point, attrs, _, _) => {
                assert_eq!(attrs.len(), 1);
                match &attrs[0] {
                    GeomAttribute::Mapped(AstAesthetic::Y, col) => {
                        assert_eq!(col, "b");
                    }
                    _ => panic!("Expected mapped y attribute"),
                }
            }
            _ => panic!("Expected Geom Point"),
        }
    }

    #[test]
    fn test_parse_geom_mixed_attributes() {
        let source = "GEOM LINE { y=:revenue, color=\"#0000FF\" }";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Geom(GeometryType::Line, attrs, _, _) => {
                assert_eq!(attrs.len(), 2);
            }
            _ => panic!("Expected Geom Line"),
        }
    }

    #[test]
    fn test_parse_facet_wrap() {
        let source = "MAP x=:x, y=:y\nGEOM POINT\nFACET WRAP :region";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 3);
        match &program.statements[2] {
            Statement::Facet(FacetSpec::Wrap {
                variable,
                columns,
                scales,
            }) => {
                assert_eq!(variable, "region");
                assert!(columns.is_none());
                assert_eq!(*scales, ScaleFreedom::Fixed);
            }
            _ => panic!("Expected Facet Wrap statement"),
        }
    }

    #[test]
    fn test_parse_facet_wrap_with_columns() {
        let source = "FACET WRAP :group COLUMNS 3";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Wrap {
                variable,
                columns,
                scales,
            }) => {
                assert_eq!(variable, "group");
                assert_eq!(*columns, Some(3));
                assert_eq!(*scales, ScaleFreedom::Fixed);
            }
            _ => panic!("Expected Facet Wrap statement"),
        }
    }

    #[test]
    fn test_parse_facet_wrap_scales_free() {
        let source = "FACET WRAP :store SCALES FREE";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Wrap { variable, scales, .. }) => {
                assert_eq!(variable, "store");
                assert_eq!(*scales, ScaleFreedom::Free);
            }
            _ => panic!("Expected Facet Wrap with FREE scales"),
        }
    }

    #[test]
    fn test_parse_facet_wrap_scales_free_x() {
        let source = "FACET WRAP :store SCALES FREE X";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Wrap { scales, .. }) => {
                assert_eq!(*scales, ScaleFreedom::FreeX);
            }
            _ => panic!("Expected Facet Wrap with FREE X scales"),
        }
    }

    #[test]
    fn test_parse_facet_wrap_scales_free_y() {
        let source = "FACET WRAP :store SCALES FREE Y";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Wrap { scales, .. }) => {
                assert_eq!(*scales, ScaleFreedom::FreeY);
            }
            _ => panic!("Expected Facet Wrap with FREE Y scales"),
        }
    }

    #[test]
    fn test_parse_facet_wrap_scales_fixed() {
        let source = "FACET WRAP :store SCALES FIXED";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Wrap { scales, .. }) => {
                assert_eq!(*scales, ScaleFreedom::Fixed);
            }
            _ => panic!("Expected Facet Wrap with FIXED scales"),
        }
    }

    #[test]
    fn test_parse_facet_wrap_columns_and_scales() {
        let source = "FACET WRAP :store COLUMNS 3 SCALES FREE";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Wrap {
                variable,
                columns,
                scales,
            }) => {
                assert_eq!(variable, "store");
                assert_eq!(*columns, Some(3));
                assert_eq!(*scales, ScaleFreedom::Free);
            }
            _ => panic!("Expected Facet Wrap with columns and scales"),
        }
    }

    #[test]
    fn test_parse_facet_grid_rows_only() {
        let source = "FACET GRID ROWS :store";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Grid {
                row_var,
                col_var,
                scales,
            }) => {
                assert_eq!(row_var.as_deref(), Some("store"));
                assert!(col_var.is_none());
                assert_eq!(*scales, ScaleFreedom::Fixed);
            }
            _ => panic!("Expected Facet Grid with rows only"),
        }
    }

    #[test]
    fn test_parse_facet_grid_cols_only() {
        let source = "FACET GRID COLS :town";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Grid {
                row_var,
                col_var,
                scales,
            }) => {
                assert!(row_var.is_none());
                assert_eq!(col_var.as_deref(), Some("town"));
                assert_eq!(*scales, ScaleFreedom::Fixed);
            }
            _ => panic!("Expected Facet Grid with cols only"),
        }
    }

    #[test]
    fn test_parse_facet_grid_rows_cols() {
        let source = "FACET GRID ROWS :store COLS :town";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Grid {
                row_var,
                col_var,
                scales,
            }) => {
                assert_eq!(row_var.as_deref(), Some("store"));
                assert_eq!(col_var.as_deref(), Some("town"));
                assert_eq!(*scales, ScaleFreedom::Fixed);
            }
            _ => panic!("Expected Facet Grid with rows and cols"),
        }
    }

    #[test]
    fn test_parse_facet_grid_with_scales() {
        let source = "FACET GRID ROWS :store COLS :town SCALES FREE X";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(FacetSpec::Grid {
                row_var,
                col_var,
                scales,
            }) => {
                assert_eq!(row_var.as_deref(), Some("store"));
                assert_eq!(col_var.as_deref(), Some("town"));
                assert_eq!(*scales, ScaleFreedom::FreeX);
            }
            _ => panic!("Expected Facet Grid with scales"),
        }
    }

    #[test]
    fn test_parse_geom_no_attributes_backward_compat() {
        let source = "GEOM POINT";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Geom(GeometryType::Point, attrs, _, _) => assert!(attrs.is_empty()),
            _ => panic!("Expected Geom Point with no attrs"),
        }
    }

    #[test]
    fn test_parse_scale_x_discrete() {
        let source = "SCALE X DISCRETE";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Scale(AstAesthetic::X, ScaleType::Discrete) => {}
            _ => panic!("Expected Scale(X, Discrete)"),
        }
    }

    #[test]
    fn test_parse_scale_y_continuous() {
        let source = "SCALE Y CONTINUOUS";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Scale(AstAesthetic::Y, ScaleType::Continuous) => {}
            _ => panic!("Expected Scale(Y, Continuous)"),
        }
    }

    #[test]
    fn test_parse_scale_y_log() {
        let source = "SCALE Y LOG";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Scale(AstAesthetic::Y, ScaleType::Log) => {}
            _ => panic!("Expected Scale(Y, Log)"),
        }
    }

    #[test]
    fn test_parse_scale_x_log() {
        let source = "SCALE X LOG";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Scale(AstAesthetic::X, ScaleType::Log) => {}
            _ => panic!("Expected Scale(X, Log)"),
        }
    }

    #[test]
    fn test_parse_scale_with_other_statements() {
        let source = "MAP x=:category, y=:value\nGEOM POINT\nSCALE X DISCRETE";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 3);
        match &program.statements[2] {
            Statement::Scale(AstAesthetic::X, ScaleType::Discrete) => {}
            _ => panic!("Expected Scale statement"),
        }
    }

    #[test]
    fn test_parse_geom_bar() {
        let source = "MAP x=:category\nGEOM BAR";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 2);
        match &program.statements[1] {
            Statement::Geom(GeometryType::Bar, attrs, _, pos) => {
                assert!(attrs.is_empty());
                assert!(pos.is_none());
            }
            _ => panic!("Expected Geom Bar statement"),
        }
    }

    #[test]
    fn test_parse_geom_bar_dodge() {
        let source = "GEOM BAR DODGE";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Geom(GeometryType::Bar, _, _, pos) => {
                assert!(matches!(pos, Some(PositionAdjustment::Dodge)));
            }
            _ => panic!("Expected Geom Bar with Dodge"),
        }
    }

    #[test]
    fn test_parse_geom_bar_stack() {
        let source = "GEOM BAR STACK";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Geom(GeometryType::Bar, _, _, pos) => {
                assert!(matches!(pos, Some(PositionAdjustment::Stack)));
            }
            _ => panic!("Expected Geom Bar with Stack"),
        }
    }

    #[test]
    fn test_parse_geom_bar_with_attributes_and_dodge() {
        let source = "GEOM BAR { fill=:region } DODGE";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Geom(GeometryType::Bar, attrs, _, pos) => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(&attrs[0], GeomAttribute::Mapped(AstAesthetic::Fill, _)));
                assert!(matches!(pos, Some(PositionAdjustment::Dodge)));
            }
            _ => panic!("Expected Geom Bar with attrs and Dodge"),
        }
    }

    #[test]
    fn test_parse_fill_aesthetic() {
        let source = "MAP x=:year, y=:sales, fill=:region\nGEOM BAR";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Map(mappings) => {
                assert_eq!(mappings.len(), 3);
                assert!(matches!(mappings[2].aesthetic, AstAesthetic::Fill));
                assert_eq!(mappings[2].column, "region");
            }
            _ => panic!("Expected Map statement"),
        }
    }

    #[test]
    fn test_parse_point_no_position_adjustment() {
        let source = "GEOM POINT";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Geom(GeometryType::Point, _, _, pos) => {
                assert!(pos.is_none());
            }
            _ => panic!("Expected Geom Point"),
        }
    }

    #[test]
    fn test_parse_alpha_mapped() {
        let source = "MAP x=:x, y=:y, alpha=:density\nGEOM POINT";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Map(mappings) => {
                assert_eq!(mappings.len(), 3);
                assert!(matches!(mappings[2].aesthetic, AstAesthetic::Alpha));
                assert_eq!(mappings[2].column, "density");
            }
            _ => panic!("Expected Map statement"),
        }
    }

    #[test]
    fn test_parse_alpha_constant() {
        let source = "MAP x=:x, y=:y\nGEOM POINT { alpha=0.3 }";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[1] {
            Statement::Geom(GeometryType::Point, attrs, _, _) => {
                assert_eq!(attrs.len(), 1);
                match &attrs[0] {
                    GeomAttribute::Constant(AstAesthetic::Alpha, LiteralValue::Number(n)) => {
                        assert!((n - 0.3).abs() < 1e-9);
                    }
                    _ => panic!("Expected constant alpha attribute"),
                }
            }
            _ => panic!("Expected Geom Point"),
        }
    }

    #[test]
    fn test_parse_coord_polar() {
        let source = "MAP x=:x, y=:y\nGEOM POINT\nCOORD POLAR";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 3);
        match &program.statements[2] {
            Statement::Coord(CoordType::Polar { start }) => {
                assert!((start - 0.0).abs() < 1e-9);
            }
            _ => panic!("Expected Coord Polar statement"),
        }
    }

    #[test]
    fn test_parse_coord_polar_start() {
        let source = "COORD POLAR START 1.57";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Coord(CoordType::Polar { start }) => {
                assert!((start - 1.57).abs() < 1e-9);
            }
            _ => panic!("Expected Coord Polar with start angle"),
        }
    }

    #[test]
    fn test_parse_coord_cartesian() {
        let source = "COORD CARTESIAN";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Coord(CoordType::Cartesian) => {}
            _ => panic!("Expected Coord Cartesian statement"),
        }
    }
}
