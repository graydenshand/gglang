use pest::Parser;
use pest_derive::Parser;

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

#[derive(Debug)]
pub enum Statement {
    Map(Vec<DataMapping>),
    Geom(GeometryType, Vec<GeomAttribute>),
    Facet(FacetSpec),
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

fn parse_data_reference(pair: pest::iterators::Pair<Rule>) -> String {
    pair.into_inner()
        .next()
        .unwrap()
        .as_str()
        .to_string()
}

fn parse_facet_scales(pair: pest::iterators::Pair<Rule>) -> ScaleFreedom {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
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
    }
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
                        let facet_inner = stmt_inner.into_inner().next().unwrap();
                        let spec = match facet_inner.as_rule() {
                            Rule::facet_wrap => {
                                let mut inner = facet_inner.into_inner();
                                let variable = parse_data_reference(inner.next().unwrap());
                                let mut columns = None;
                                let mut scales = ScaleFreedom::Fixed;
                                for pair in inner {
                                    match pair.as_rule() {
                                        Rule::facet_columns => {
                                            let n: u32 = pair.into_inner().next().unwrap().as_str().parse().unwrap();
                                            columns = Some(n);
                                        }
                                        Rule::facet_scales => {
                                            scales = parse_facet_scales(pair);
                                        }
                                        _ => {}
                                    }
                                }
                                FacetSpec::Wrap { variable, columns, scales }
                            }
                            Rule::facet_grid => {
                                let mut inner = facet_inner.into_inner();
                                let grid_spec = inner.next().unwrap();
                                let (row_var, col_var) = match grid_spec.as_rule() {
                                    Rule::facet_rows_cols => {
                                        let mut gi = grid_spec.into_inner();
                                        let row = parse_data_reference(gi.next().unwrap());
                                        let col = parse_data_reference(gi.next().unwrap());
                                        (Some(row), Some(col))
                                    }
                                    Rule::facet_rows_only => {
                                        let mut gi = grid_spec.into_inner();
                                        let row = parse_data_reference(gi.next().unwrap());
                                        (Some(row), None)
                                    }
                                    Rule::facet_cols_only => {
                                        let mut gi = grid_spec.into_inner();
                                        let col = parse_data_reference(gi.next().unwrap());
                                        (None, Some(col))
                                    }
                                    _ => unreachable!(),
                                };
                                let mut scales = ScaleFreedom::Fixed;
                                for pair in inner {
                                    if pair.as_rule() == Rule::facet_scales {
                                        scales = parse_facet_scales(pair);
                                    }
                                }
                                FacetSpec::Grid { row_var, col_var, scales }
                            }
                            _ => unreachable!(),
                        };
                        statements.push(Statement::Facet(spec));
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
    fn test_parse_facet_wrap() {
        let source = "MAP x=:x, y=:y\nGEOM POINT\nFACET WRAP :region";
        let program = parse(source).expect("Parse should succeed");
        assert_eq!(program.statements.len(), 3);
        match &program.statements[2] {
            Statement::Facet(FacetSpec::Wrap { variable, columns, scales }) => {
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
            Statement::Facet(FacetSpec::Wrap { variable, columns, scales }) => {
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
            Statement::Facet(FacetSpec::Wrap { variable, columns, scales }) => {
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
            Statement::Facet(FacetSpec::Grid { row_var, col_var, scales }) => {
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
            Statement::Facet(FacetSpec::Grid { row_var, col_var, scales }) => {
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
            Statement::Facet(FacetSpec::Grid { row_var, col_var, scales }) => {
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
            Statement::Facet(FacetSpec::Grid { row_var, col_var, scales }) => {
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
            Statement::Geom(GeometryType::Point, attrs) => assert!(attrs.is_empty()),
            _ => panic!("Expected Geom Point with no attrs"),
        }
    }
}
