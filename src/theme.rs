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
            y_gutter_width: 65,
            y_axis_label_width: 40,
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
