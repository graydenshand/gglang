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
    Geom(GeometryType),
    Title(String),
    Caption(String),
    XLabel(String),
    YLabel(String),
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
}

#[derive(Debug)]
pub enum GeometryType {
    Point,
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
                        for pair in stmt_inner.into_inner() {
                            if pair.as_rule() == Rule::geometry_type {
                                match pair.as_str() {
                                    "POINT" => {
                                        statements.push(Statement::Geom(GeometryType::Point))
                                    }
                                    other => {
                                        return Err(format!("Unsupported geometry: {}", other))
                                    }
                                }
                                break;
                            }
                        }
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
            Statement::Geom(GeometryType::Point) => {}
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
}
