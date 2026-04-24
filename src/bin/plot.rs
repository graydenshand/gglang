fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <script.gg> <data.csv> [--output <path.svg|path.png>] [--width N] [--height N]",
            args[0]
        );
        std::process::exit(1);
    }

    let script_path = std::path::Path::new(&args[1]);
    let base_dir = script_path.parent();
    let source = std::fs::read_to_string(script_path)?;
    let program = ggc::ast::parse(&source)?;
    let mut blueprint = ggc::compile::compile(&program, &ggc::theme::Theme::default(), base_dir)?;
    let data = ggc::data::load_csv(std::path::Path::new(&args[2]))?;

    let flag_val = |flag: &str| -> Option<String> {
        args.windows(2)
            .find(|w| w[0] == flag)
            .map(|w| w[1].clone())
    };

    let output_path = flag_val("--output");
    let width: u32 = flag_val("--width")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2400);
    let height: u32 = flag_val("--height")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);

    if let Some(path) = output_path {
        let plot_output = blueprint.render(data)?;
        let theme = blueprint.theme();
        if path.ends_with(".svg") {
            let svg = ggc::svg::render_svg(&plot_output, theme, width, height);
            std::fs::write(&path, svg)?;
            println!("Wrote {path}");
        } else if path.ends_with(".png") {
            let png = ggc::png::render_png(&plot_output, theme, width, height)?;
            std::fs::write(&path, png)?;
            println!("Wrote {path}");
        } else {
            eprintln!("Unknown output format: {path}");
            std::process::exit(1);
        }
    } else {
        ggc::app::run(blueprint, data)?;
    }

    Ok(())
}
