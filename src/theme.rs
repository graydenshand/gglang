use crate::ast::LiteralValue;
use crate::error::GglangError;
use crate::layout::Unit;

#[derive(Clone)]
pub struct Theme {
    // Window
    pub window_margin: Unit,

    // Font sizes
    pub title_font_size: f32,
    pub axis_label_font_size: f32,
    pub tick_label_font_size: f32,
    pub caption_font_size: f32,
    pub legend_label_font_size: f32,

    // Layout region sizes (pixels)
    pub title_height: u32,
    pub caption_height: u32,
    pub x_gutter_height: u32,
    pub y_gutter_width: u32,
    pub y_axis_label_width: u32,
    pub legend_width: u32,
    pub legend_margin: u32,
    pub gutter_spacer_height: u32,

    // Axes
    pub axis_color: [f32; 4],

    // Faceting
    pub facet_label_height: u32,
    pub facet_gap: u32,
    pub facet_label_font_size: f32,
    pub facet_label_bg_color: [f32; 4],
    pub panel_border_color: [f32; 4],
    pub panel_border_thickness: f32,
    pub facet_row_label_width: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            window_margin: Unit::Pixels(30),

            title_font_size: 48.0,
            axis_label_font_size: 36.0,
            tick_label_font_size: 24.0,
            caption_font_size: 28.0,
            legend_label_font_size: 32.0,

            title_height: 110,
            caption_height: 50,
            x_gutter_height: 50,
            y_gutter_width: 80,
            y_axis_label_width: 55,
            legend_width: 200,
            legend_margin: 16,
            gutter_spacer_height: 20,

            axis_color: [0.7, 0.7, 0.7, 1.0],

            facet_label_height: 36,
            facet_gap: 16,
            facet_label_font_size: 22.0,
            facet_label_bg_color: [0.85, 0.85, 0.85, 1.0],
            panel_border_color: [0.7, 0.7, 0.7, 1.0],
            panel_border_thickness: 1.0,
            facet_row_label_width: 40,
        }
    }
}

impl Theme {
    pub fn apply_override(&mut self, key: &str, value: &LiteralValue) -> Result<(), GglangError> {
        match key {
            // f32 font-size fields
            "title_font_size" => self.title_font_size = as_f32(key, value)?,
            "axis_label_font_size" => self.axis_label_font_size = as_f32(key, value)?,
            "tick_label_font_size" => self.tick_label_font_size = as_f32(key, value)?,
            "caption_font_size" => self.caption_font_size = as_f32(key, value)?,
            "legend_label_font_size" => self.legend_label_font_size = as_f32(key, value)?,
            "facet_label_font_size" => self.facet_label_font_size = as_f32(key, value)?,
            "panel_border_thickness" => self.panel_border_thickness = as_f32(key, value)?,

            // u32 pixel-size fields
            "title_height" => self.title_height = as_u32(key, value)?,
            "caption_height" => self.caption_height = as_u32(key, value)?,
            "x_gutter_height" => self.x_gutter_height = as_u32(key, value)?,
            "y_gutter_width" => self.y_gutter_width = as_u32(key, value)?,
            "y_axis_label_width" => self.y_axis_label_width = as_u32(key, value)?,
            "legend_width" => self.legend_width = as_u32(key, value)?,
            "legend_margin" => self.legend_margin = as_u32(key, value)?,
            "gutter_spacer_height" => self.gutter_spacer_height = as_u32(key, value)?,
            "facet_label_height" => self.facet_label_height = as_u32(key, value)?,
            "facet_gap" => self.facet_gap = as_u32(key, value)?,
            "facet_row_label_width" => self.facet_row_label_width = as_u32(key, value)?,

            // Unit field (pixels only for now)
            "window_margin" => self.window_margin = Unit::Pixels(as_u32(key, value)?),

            // [f32; 4] color fields
            "axis_color" => self.axis_color = as_color(key, value)?,
            "facet_label_bg_color" => self.facet_label_bg_color = as_color(key, value)?,
            "panel_border_color" => self.panel_border_color = as_color(key, value)?,

            other => {
                return Err(GglangError::Compile {
                    message: format!("Unknown theme key: {other}"),
                })
            }
        }
        Ok(())
    }
}

fn as_f32(key: &str, value: &LiteralValue) -> Result<f32, GglangError> {
    match value {
        LiteralValue::Number(n) => Ok(*n as f32),
        LiteralValue::Str(_) => Err(GglangError::Compile {
            message: format!("Theme key '{key}' expects a number, got a string"),
        }),
    }
}

fn as_u32(key: &str, value: &LiteralValue) -> Result<u32, GglangError> {
    match value {
        LiteralValue::Number(n) => {
            if *n < 0.0 {
                return Err(GglangError::Compile {
                    message: format!("Theme key '{key}' expects a non-negative integer, got {n}"),
                });
            }
            Ok(*n as u32)
        }
        LiteralValue::Str(_) => Err(GglangError::Compile {
            message: format!("Theme key '{key}' expects a number, got a string"),
        }),
    }
}

fn as_color(key: &str, value: &LiteralValue) -> Result<[f32; 4], GglangError> {
    match value {
        LiteralValue::Str(s) => parse_hex_color_rgba(key, s),
        LiteralValue::Number(_) => Err(GglangError::Compile {
            message: format!("Theme key '{key}' expects a hex color string, got a number"),
        }),
    }
}

fn parse_hex_color_rgba(key: &str, s: &str) -> Result<[f32; 4], GglangError> {
    let s = s.trim_start_matches('#');
    let err = || GglangError::Compile {
        message: format!("Theme key '{key}': expected 6- or 8-digit hex color, got #{s}"),
    };
    let parse_byte = |hex: &str| -> Result<f32, GglangError> {
        u8::from_str_radix(hex, 16)
            .map(|b| b as f32 / 255.0)
            .map_err(|_| err())
    };
    match s.len() {
        6 => Ok([
            parse_byte(&s[0..2])?,
            parse_byte(&s[2..4])?,
            parse_byte(&s[4..6])?,
            1.0,
        ]),
        8 => Ok([
            parse_byte(&s[0..2])?,
            parse_byte(&s[2..4])?,
            parse_byte(&s[4..6])?,
            parse_byte(&s[6..8])?,
        ]),
        _ => Err(err()),
    }
}
