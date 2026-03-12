fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <script.gg> <data.csv>", args[0]);
        std::process::exit(1);
    }

    let source = std::fs::read_to_string(&args[1])?;
    let program = ggc::ast::parse(&source).map_err(|e| anyhow::anyhow!(e))?;
    let theme = ggc::theme::Theme::default();
    let blueprint = ggc::compile::compile(&program, &theme).map_err(|e| anyhow::anyhow!(e))?;
    let data = ggc::data::load_csv(std::path::Path::new(&args[2]))
        .map_err(|e| anyhow::anyhow!(e))?;

    ggc::app::run(blueprint, data)
}
