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
                                        let data_ref = inner.next().unwrap();
                                        let column = data_ref
                                            .into_inner()
                                            .next()
                                            .unwrap()
                                            .as_str()
                                            .to_string();
                                        let aes_str = inner.next().unwrap().as_str();
                                        let aesthetic = match aes_str {
                                            "x" => AstAesthetic::X,
                                            "y" => AstAesthetic::Y,
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
        let source = "MAP :x TO x, :y TO y\nGEOM POINT";
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
}
