use crate::layout::Unit;

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
    pub legend_width: u32,
    pub legend_margin: u32,
    pub gutter_spacer_height: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            window_margin: Unit::Percent(10.),

            title_font_size: 42.0,
            axis_label_font_size: 30.0,
            tick_label_font_size: 24.0,
            caption_font_size: 20.0,
            legend_label_font_size: 24.0,

            title_height: 200,
            caption_height: 100,
            x_gutter_height: 40,
            y_gutter_width: 60,
            legend_width: 200,
            legend_margin: 16,
            gutter_spacer_height: 60,
        }
    }
}
