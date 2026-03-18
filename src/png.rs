use crate::layout::PlotOutput;
use crate::svg::render_svg;
use crate::theme::Theme;

/// Render a `PlotOutput` to a PNG byte vector.
pub fn render_png(output: &PlotOutput, theme: &Theme, width: u32, height: u32) -> Vec<u8> {
    let svg = render_svg(output, theme, width, height);

    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(&svg, &opt).expect("valid SVG");
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).expect("valid pixel dimensions");
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().expect("PNG encoding succeeded")
}
