use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct GGCParser;

#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Map(Vec<DataMapping>),
    Geom(GeometryType, Vec<GeomAttribute>),
    Facet(String, Option<u32>),
    Title(String),
    Caption(String),
    XLabel(String),
    YLabel(String),
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
    Group,
}

#[derive(Debug)]
pub enum GeometryType {
    Point,
    Line,
}

pub fn parse(source: &str) -> Result<Program, String> {
    let pairs = GGCParser::parse(Rule::program, source).map_err(|e| e.to_string())?;

    let pair = pairs.into_iter().next().ok_or("Empty program")?;

    let mut statements = vec![];

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::statement => {
                let stmt_inner = inner_pair.into_inner().next().unwrap();
                match stmt_inner.as_rule() {
                    Rule::map_statement => {
                        let mut mappings = vec![];
                        for pair in stmt_inner.into_inner() {
                            if pair.as_rule() == Rule::data_mappings {
                                for mapping_pair in pair.into_inner() {
                                    if mapping_pair.as_rule() == Rule::data_mapping {
                                        let mut inner = mapping_pair.into_inner();
                                        let aes_str = inner.next().unwrap().as_str();
                                        let data_ref = inner.next().unwrap();
                                        let column = data_ref
                                            .into_inner()
                                            .next()
                                            .unwrap()
                                            .as_str()
                                            .to_string();
                                        let aesthetic = match aes_str {
                                            "x" => AstAesthetic::X,
                                            "y" => AstAesthetic::Y,
                                            "color" => AstAesthetic::Color,
                                            "group" => AstAesthetic::Group,
                                            other => {
                                                return Err(format!(
                                                    "Unsupported aesthetic: {}",
                                                    other
                                                ))
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
                        let geom_type_pair = inner.next().unwrap();
                        let geom_type = match geom_type_pair.as_str() {
                            "POINT" => GeometryType::Point,
                            "LINE" => GeometryType::Line,
                            other => return Err(format!("Unsupported geometry: {}", other)),
                        };
                        let mut attrs = vec![];
                        for pair in inner {
                            if pair.as_rule() == Rule::geom_attributes {
                                for attr_pair in pair.into_inner() {
                                    if attr_pair.as_rule() == Rule::geom_attribute {
                                        let mut attr_inner = attr_pair.into_inner();
                                        let aes_str = attr_inner.next().unwrap().as_str();
                                        let aes = match aes_str {
                                            "x" => AstAesthetic::X,
                                            "y" => AstAesthetic::Y,
                                            "color" => AstAesthetic::Color,
                                            "group" => AstAesthetic::Group,
                                            other => return Err(format!("Unsupported aesthetic: {}", other)),
                                        };
                                        let val_pair = attr_inner.next().unwrap();
                                        let val_inner = val_pair.into_inner().next().unwrap();
                                        let attr = match val_inner.as_rule() {
                                            Rule::data_reference => {
                                                let col = val_inner
                                                    .into_inner()
                                                    .next()
                                                    .unwrap()
                                                    .as_str()
                                                    .to_string();
                                                GeomAttribute::Mapped(aes, col)
                                            }
                                            Rule::string_literal => {
                                                let s = val_inner.as_str();
                                                GeomAttribute::Constant(
                                                    aes,
                                                    LiteralValue::Str(s[1..s.len() - 1].to_string()),
                                                )
                                            }
                                            Rule::number => {
                                                let n: f64 = val_inner.as_str().parse().unwrap();
                                                GeomAttribute::Constant(aes, LiteralValue::Number(n))
                                            }
                                            _ => unreachable!(),
                                        };
                                        attrs.push(attr);
                                    }
                                }
                            }
                        }
                        statements.push(Statement::Geom(geom_type, attrs));
                    }
                    Rule::facet_statement => {
                        let mut inner = stmt_inner.into_inner();
                        let data_ref = inner.next().unwrap();
                        let variable = data_ref
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str()
                            .to_string();
                        let columns = inner.next().and_then(|pair| {
                            if pair.as_rule() == Rule::facet_columns {
                                let n: u32 = pair.into_inner().next().unwrap().as_str().parse().unwrap();
                                Some(n)
                            } else {
                                None
                            }
                        });
                        statements.push(Statement::Facet(variable, columns));
                    }
                    Rule::title_statement => {
                        let s = stmt_inner.into_inner().next().unwrap().as_str();
                        statements.push(Statement::Title(s[1..s.len() - 1].to_string()));
                    }
                    Rule::caption_statement => {
                        let s = stmt_inner.into_inner().next().unwrap().as_str();
                        statements.push(Statement::Caption(s[1..s.len() - 1].to_string()));
                    }
                    Rule::xlabel_statement => {
                        let s = stmt_inner.into_inner().next().unwrap().as_str();
                        statements.push(Statement::XLabel(s[1..s.len() - 1].to_string()));
                    }
                    Rule::ylabel_statement => {
                        let s = stmt_inner.into_inner().next().unwrap().as_str();
                        statements.push(Statement::YLabel(s[1..s.len() - 1].to_string()));
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
            Statement::Geom(GeometryType::Point, attrs) => assert!(attrs.is_empty()),
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
            Statement::Geom(GeometryType::Line, attrs) => assert!(attrs.is_empty()),
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
            Statement::Geom(GeometryType::Point, attrs) => {
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
            Statement::Geom(GeometryType::Point, attrs) => {
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
            Statement::Geom(GeometryType::Line, attrs) => {
                assert_eq!(attrs.len(), 2);
            }
            _ => panic!("Expected Geom Line"),
        }
    }

    #[test]
    fn test_parse_facet() {
        let source = "MAP x=:x, y=:y\nGEOM POINT\nFACET BY :region";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 3);
        match &program.statements[2] {
            Statement::Facet(var, cols) => {
                assert_eq!(var, "region");
                assert!(cols.is_none());
            }
            _ => panic!("Expected Facet statement"),
        }
    }

    #[test]
    fn test_parse_facet_with_columns() {
        let source = "FACET BY :group COLUMNS 3";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Facet(var, cols) => {
                assert_eq!(var, "group");
                assert_eq!(*cols, Some(3));
            }
            _ => panic!("Expected Facet statement"),
        }
    }

    #[test]
    fn test_parse_geom_no_attributes_backward_compat() {
        let source = "GEOM POINT";
        let program = parse(source).expect("Parse should succeed");
        match &program.statements[0] {
            Statement::Geom(GeometryType::Point, attrs) => assert!(attrs.is_empty()),
            _ => panic!("Expected Geom Point with no attrs"),
        }
    }
}
