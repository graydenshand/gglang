use crate::error::GglangError;
use crate::layout::PlotOutput;
use crate::svg::render_svg;
use crate::theme::Theme;

/// Render a `PlotOutput` to a PNG byte vector.
pub fn render_png(
    output: &PlotOutput,
    theme: &Theme,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, GglangError> {
    let svg = render_svg(output, theme, width, height);

    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(&svg, &opt).map_err(|e| GglangError::Export {
        message: format!("Invalid SVG: {}", e),
    })?;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or_else(|| GglangError::Export {
            message: format!("Invalid pixel dimensions: {}x{}", width, height),
        })?;
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| GglangError::Export {
        message: format!("PNG encoding failed: {}", e),
    })
}
