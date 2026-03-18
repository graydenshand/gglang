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

    // Axes
    pub axis_color: [f32; 4],

    // Faceting
    pub facet_label_height: u32,
    pub facet_gap: u32,
    pub facet_label_font_size: f32,
    pub facet_label_bg_color: [f32; 4],
    pub panel_border_color: [f32; 4],
    pub panel_border_thickness: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            window_margin: Unit::Percent(10.),

            title_font_size: 72.0,
            axis_label_font_size: 48.0,
            tick_label_font_size: 32.0,
            caption_font_size: 32.0,
            legend_label_font_size: 40.0,

            title_height: 200,
            caption_height: 150,
            x_gutter_height: 60,
            y_gutter_width: 100,
            legend_width: 200,
            legend_margin: 16,
            gutter_spacer_height: 60,

            axis_color: [0.7, 0.7, 0.7, 1.0],

            facet_label_height: 40,
            facet_gap: 16,
            facet_label_font_size: 32.0,
            facet_label_bg_color: [0.85, 0.85, 0.85, 1.0],
            panel_border_color: [0.7, 0.7, 0.7, 1.0],
            panel_border_thickness: 1.0,
        }
    }
}
