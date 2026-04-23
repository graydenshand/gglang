use crate::layout::{PlotOutput, PlotRegion, WindowSegment};
use crate::shape::{Element, HAlign, TextRotation, VAlign};
use crate::theme::Theme;

/// Render a `PlotOutput` to an SVG string.
pub fn render_svg(output: &PlotOutput, theme: &Theme, width: u32, height: u32) -> String {
    let root = WindowSegment::new_root(width, height);
    let margined = root.with_margin(theme.window_margin);
    let mut segments = output.layout.resolve(&margined);

    // For polar plots, square up DataArea segments so circles aren't distorted.
    if output.is_polar {
        for (key, seg) in segments.iter_mut() {
            if key.region == PlotRegion::DataArea {
                *seg = seg.squared();
            }
        }
    }

    // Pair each region with its resolved segment (drop any regions not in the layout).
    let mut regions: Vec<_> = output
        .regions
        .iter()
        .filter_map(|(k, elems)| segments.get(k).map(|s| (k, s, elems)))
        .collect();
    // Stable order for deterministic clip IDs.
    regions.sort_by_key(|(k, _, _)| format!("{:?}", k));

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    );

    // White background (outside any clip).
    svg.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"white\"/>\n"
    ));

    // One <clipPath> per region — mirrors the GPU scissor rectangle.
    // Axis gutter clips are expanded by the tick label font size so labels
    // whose anchor points sit at the region edge are not clipped.
    let tick_font = theme.tick_label_font_size;
    let tick_half = tick_font / 2.0;
    svg.push_str("<defs>\n");
    for (i, (k, seg, _)) in regions.iter().enumerate() {
        let mut x = seg.pixel_scale_x.min as f32;
        let mut y = seg.pixel_scale_y.min as f32;
        let mut w = seg.pixel_scale_x.span() as f32;
        let mut h = seg.pixel_scale_y.span() as f32;
        if k.region == PlotRegion::YAxisGutter {
            x -= tick_font;
            w += tick_font;
            y -= tick_half;
            h += tick_half * 2.0;
        }
        if k.region == PlotRegion::XAxisGutter {
            x -= tick_font;
            w += tick_font * 2.0;
        }
        svg.push_str(&format!(
            "  <clipPath id=\"cp{i}\"><rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"/></clipPath>\n"
        ));
    }
    svg.push_str("</defs>\n");

    // Elements, each group clipped to its region bounds.
    for (i, (_, seg, elements)) in regions.iter().enumerate() {
        if elements.is_empty() {
            continue;
        }
        svg.push_str(&format!("<g clip-path=\"url(#cp{i})\">\n"));
        for element in *elements {
            render_element(&mut svg, element, seg);
        }
        svg.push_str("</g>\n");
    }

    svg.push_str("</svg>\n");
    svg
}

fn render_element(svg: &mut String, element: &Element, seg: &WindowSegment) {
    match element {
        Element::Rect(r) => {
            let cx = seg.px_x(&r.position[0]);
            let cy = seg.px_y(&r.position[1]);
            let w = seg.px_width(&r.width);
            let h = seg.px_height(&r.height);
            let x = cx - w / 2.0;
            let y = cy - h / 2.0;
            let fill = rgba_to_css(r.color);
            svg.push_str(&format!(
                "  <rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"{fill}\"/>\n"
            ));
        }

        Element::Point(p) => {
            let cx = seg.px_x(&p.position[0]);
            let cy = seg.px_y(&p.position[1]);
            let r = seg.px_width(&p.size) / 2.0;
            let fill = rgba_to_css(p.color);
            svg.push_str(&format!(
                "  <circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\" fill=\"{fill}\"/>\n"
            ));
        }

        Element::Polyline(pl) => {
            if pl.points.is_empty() {
                return;
            }
            let uniform_color = pl.colors.windows(2).all(|w| w[0] == w[1]);
            if uniform_color && !pl.colors.is_empty() {
                let pts: Vec<String> = pl
                    .points
                    .iter()
                    .map(|pt| {
                        format!("{:.2},{:.2}", seg.px_x(&pt[0]), seg.px_y(&pt[1]))
                    })
                    .collect();
                let stroke = rgba_to_css(pl.colors[0]);
                svg.push_str(&format!(
                    "  <polyline points=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\" fill=\"none\" stroke-linejoin=\"miter\"/>\n",
                    pts.join(" "),
                    stroke,
                    pl.thickness
                ));
            } else {
                for i in 0..pl.points.len().saturating_sub(1) {
                    let x1 = seg.px_x(&pl.points[i][0]);
                    let y1 = seg.px_y(&pl.points[i][1]);
                    let x2 = seg.px_x(&pl.points[i + 1][0]);
                    let y2 = seg.px_y(&pl.points[i + 1][1]);
                    let stroke = rgba_to_css(pl.colors[i]);
                    svg.push_str(&format!(
                        "  <line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"{stroke}\" stroke-width=\"{:.2}\"/>\n",
                        pl.thickness
                    ));
                }
            }
        }

        Element::Arc(arc) => {
            let cx = seg.px_x(&arc.center[0]);
            let cy = seg.px_y(&arc.center[1]);
            let r_outer = seg.px_width(&arc.outer_radius);
            let r_inner = seg.px_width(&arc.inner_radius);
            let fill = rgba_to_css(arc.color);

            let sweep = arc.end_angle - arc.start_angle;
            if sweep.abs() >= std::f32::consts::TAU - 0.001 {
                // Full circle — SVG arcs can't draw a full circle in one command,
                // so draw two semicircles.
                if r_inner <= 0.5 {
                    // Simple filled circle
                    svg.push_str(&format!(
                        "  <circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r_outer:.2}\" fill=\"{fill}\"/>\n"
                    ));
                } else {
                    // Full annulus — two half-arcs for outer, two for inner
                    let mid = arc.start_angle + std::f32::consts::PI;
                    let (x1o, y1o) = (cx + r_outer * arc.start_angle.cos(), cy - r_outer * arc.start_angle.sin());
                    let (x2o, y2o) = (cx + r_outer * mid.cos(), cy - r_outer * mid.sin());
                    let (x1i, y1i) = (cx + r_inner * arc.start_angle.cos(), cy - r_inner * arc.start_angle.sin());
                    let (x2i, y2i) = (cx + r_inner * mid.cos(), cy - r_inner * mid.sin());
                    svg.push_str(&format!(
                        "  <path d=\"M {x1o:.2} {y1o:.2} A {r_outer:.2} {r_outer:.2} 0 0 0 {x2o:.2} {y2o:.2} A {r_outer:.2} {r_outer:.2} 0 0 0 {x1o:.2} {y1o:.2} M {x1i:.2} {y1i:.2} A {r_inner:.2} {r_inner:.2} 0 0 1 {x2i:.2} {y2i:.2} A {r_inner:.2} {r_inner:.2} 0 0 1 {x1i:.2} {y1i:.2} Z\" fill=\"{fill}\" fill-rule=\"evenodd\"/>\n"
                    ));
                }
            } else {
                // Partial arc wedge
                let large_arc = if sweep.abs() > std::f32::consts::PI { 1 } else { 0 };
                // SVG Y is flipped (down = positive), so negate sin for Y
                let x_start_o = cx + r_outer * arc.start_angle.cos();
                let y_start_o = cy - r_outer * arc.start_angle.sin();
                let x_end_o = cx + r_outer * arc.end_angle.cos();
                let y_end_o = cy - r_outer * arc.end_angle.sin();

                if r_inner <= 0.5 {
                    // Pie wedge (from center)
                    svg.push_str(&format!(
                        "  <path d=\"M {cx:.2} {cy:.2} L {x_start_o:.2} {y_start_o:.2} A {r_outer:.2} {r_outer:.2} 0 {large_arc} 0 {x_end_o:.2} {y_end_o:.2} Z\" fill=\"{fill}\"/>\n"
                    ));
                } else {
                    // Annular wedge (donut segment)
                    let x_start_i = cx + r_inner * arc.end_angle.cos();
                    let y_start_i = cy - r_inner * arc.end_angle.sin();
                    let x_end_i = cx + r_inner * arc.start_angle.cos();
                    let y_end_i = cy - r_inner * arc.start_angle.sin();
                    svg.push_str(&format!(
                        "  <path d=\"M {x_start_o:.2} {y_start_o:.2} A {r_outer:.2} {r_outer:.2} 0 {large_arc} 0 {x_end_o:.2} {y_end_o:.2} L {x_start_i:.2} {y_start_i:.2} A {r_inner:.2} {r_inner:.2} 0 {large_arc} 1 {x_end_i:.2} {y_end_i:.2} Z\" fill=\"{fill}\"/>\n"
                    ));
                }
            }
        }

        Element::Text(t) => {
            let x = seg.px_x(&t.position.0);
            let y = seg.px_y(&t.position.1);
            let text_anchor = match t.h_align {
                HAlign::Left => "start",
                HAlign::Center => "middle",
                HAlign::Right => "end",
            };
            let dominant_baseline = match t.v_align {
                VAlign::Top => "hanging",
                VAlign::Center => "central",
            };
            let transform_attr = match t.rotation {
                TextRotation::None => String::new(),
                TextRotation::Ccw90 => format!(" transform=\"rotate(-90,{x:.2},{y:.2})\""),
                TextRotation::Cw90 => format!(" transform=\"rotate(90,{x:.2},{y:.2})\""),
            };
            let [r, g, b, a] = t.color;
            let fill_attr = format!(
                " fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{a:.3}\"",
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
            );

            if t.wrap && t.rotation == TextRotation::None {
                let max_width = seg.pixel_scale_x.span() as f32 * 0.95;
                let lines = wrap_text(&t.value, max_width, t.font_size);
                let line_height = t.font_size * 1.2;
                svg.push_str(&format!(
                    "  <text x=\"{x:.2}\" y=\"{y:.2}\" font-size=\"{}\" font-family=\"sans-serif\" text-anchor=\"{text_anchor}\" dominant-baseline=\"{dominant_baseline}\"{fill_attr}{transform_attr}>\n",
                    t.font_size
                ));
                for (i, line) in lines.iter().enumerate() {
                    let dy = if i == 0 { 0.0 } else { line_height };
                    let escaped = escape_xml(line);
                    svg.push_str(&format!(
                        "    <tspan x=\"{x:.2}\" dy=\"{dy:.1}\">{escaped}</tspan>\n"
                    ));
                }
                svg.push_str("  </text>\n");
            } else {
                let escaped = escape_xml(&t.value);
                svg.push_str(&format!(
                    "  <text x=\"{x:.2}\" y=\"{y:.2}\" font-size=\"{}\" font-family=\"sans-serif\" text-anchor=\"{text_anchor}\" dominant-baseline=\"{dominant_baseline}\"{fill_attr}{transform_attr}>{escaped}</text>\n",
                    t.font_size
                ));
            }
        }
    }
}

/// Greedy word-wrap: split `text` into lines that fit within `max_width` pixels.
/// Uses a rough per-character width estimate of `font_size * 0.55`.
fn wrap_text(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    let char_width = font_size * 0.55;
    let chars_per_line = (max_width / char_width).max(1.0) as usize;

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= chars_per_line {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn rgba_to_css(color: [f32; 4]) -> String {
    let r = (color[0] * 255.0).round() as u8;
    let g = (color[1] * 255.0).round() as u8;
    let b = (color[2] * 255.0).round() as u8;
    let a = color[3];
    format!("rgba({r},{g},{b},{a:.4})")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
