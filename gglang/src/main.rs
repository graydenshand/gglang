use pest::Parser;
use pest_derive::Parser;
use std::env;
use std::fs::File;
use std::io::Read;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct GGCParser;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Usage:\n\t{} FILE\n", args[0]);
    }

    let path = &args[1];
    let mut file = File::open(path).unwrap();
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    // println!("{}", buf);

    // A pair is a combination of the rule which matched and a span of input
    let pairs = GGCParser::parse(Rule::program, &buf).unwrap_or_else(|e| panic!("{}", e));
    // We only expect one per file
    if pairs.len() > 1 {
        panic!("Multiple programs found {}", pairs.len());
    } else if pairs.len() == 0 {
        panic!("Empty body");
    }
    let pair = pairs.into_iter().next().unwrap();

    // A pair can be converted to an iterator of the tokens which make it up:
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::statement => {
                match inner_pair.into_inner().next().unwrap().as_rule() {
                    Rule::title_statement => println!("Title Statement"),
                    Rule::facet_statement => println!("Facet Statement"),
                    Rule::coord_statement => println!("Coordinates Statement"),
                    Rule::map_statement => println!("Map Statement"),
                    Rule::geom_statement => println!("Geom Statement"),
                    Rule::scale_statement => println!("Scale Statement"),
                    Rule::theme_property_statement => println!("Theme Property Statement"),
                    _ => unreachable!(),
                }
                // println!("Statement: {:?}", inner_pair.as_str());
                // for j in inner_pair.into_inner() {
                //     println!("Statement: {:?}", j.as_rule());
                // }
            }
            Rule::EOI => {
                // END OF INPUT, for now just exit
                break;
            }
            // grammar should not allow this
            _ => unreachable!(),
        };
    }
}
