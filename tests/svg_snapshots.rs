use std::path::Path;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 900;

fn render_example(gg_path: &str, csv_path: &str) -> String {
    let source = std::fs::read_to_string(gg_path)
        .unwrap_or_else(|e| panic!("Failed to read {gg_path}: {e}"));
    let program = ggc::ast::parse(&source)
        .unwrap_or_else(|e| panic!("Failed to parse {gg_path}: {e}"));
    let theme = ggc::theme::Theme::default();
    let mut blueprint = ggc::compile::compile(&program, &theme)
        .unwrap_or_else(|e| panic!("Failed to compile {gg_path}: {e}"));
    let data = ggc::data::load_csv(Path::new(csv_path))
        .unwrap_or_else(|e| panic!("Failed to load {csv_path}: {e}"));
    let plot_output = blueprint
        .render(data)
        .unwrap_or_else(|e| panic!("Failed to render {gg_path}: {e}"));
    ggc::svg::render_svg(&plot_output, &theme, WIDTH, HEIGHT)
}

macro_rules! snapshot_test {
    ($name:ident, $gg:expr, $csv:expr) => {
        #[test]
        fn $name() {
            let svg = render_example(
                concat!("examples/", $gg),
                concat!("examples/", $csv),
            );
            insta::assert_snapshot!(svg);
        }
    };
}

snapshot_test!(scatter, "scatter.gg", "iris-mock.csv");
snapshot_test!(scatter_color, "scatter_color.gg", "iris-mock.csv");
snapshot_test!(scatter_custom_labels, "scatter_custom_labels.gg", "iris-mock.csv");
snapshot_test!(scatter_alpha, "scatter_alpha.gg", "iris-mock.csv");
snapshot_test!(scatter_alpha_constant, "scatter_alpha_constant.gg", "iris-mock.csv");
snapshot_test!(categorical, "categorical.gg", "categorical.csv");
snapshot_test!(discrete_year, "discrete_year.gg", "discrete_year.csv");
snapshot_test!(timeseries, "timeseries.gg", "stocks.csv");
snapshot_test!(timeseries_facet, "timeseries_facet.gg", "state_population.csv");
snapshot_test!(bar, "bar.gg", "bar_data.csv");
snapshot_test!(bar_stacked, "bar_stacked.gg", "bar_data.csv");
snapshot_test!(bar_dodge, "bar_dodge.gg", "bar_data.csv");
snapshot_test!(bar_count_fill, "bar_count_fill.gg", "bar_data.csv");
snapshot_test!(bar_dodge_count, "bar_dodge_count.gg", "bar_data.csv");
