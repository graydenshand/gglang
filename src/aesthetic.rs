/// An aesthetic channel that maps data to a visual property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aesthetic {
    X,
    Y,
    Color,
    Fill,
    Group,
    Alpha,
}

impl Aesthetic {
    pub fn all() -> &'static [Aesthetic] {
        &[
            Aesthetic::X,
            Aesthetic::Y,
            Aesthetic::Color,
            Aesthetic::Fill,
            Aesthetic::Group,
            Aesthetic::Alpha,
        ]
    }

    pub fn family(&self) -> AestheticFamily {
        match self {
            Aesthetic::X => AestheticFamily::HorizontalPosition,
            Aesthetic::Y => AestheticFamily::VerticalPosition,
            Aesthetic::Color => AestheticFamily::Color,
            Aesthetic::Fill => AestheticFamily::Fill,
            Aesthetic::Group => AestheticFamily::Group,
            Aesthetic::Alpha => AestheticFamily::Alpha,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Aesthetic::X => "x",
            Aesthetic::Y => "y",
            Aesthetic::Color => "color",
            Aesthetic::Fill => "fill",
            Aesthetic::Group => "group",
            Aesthetic::Alpha => "alpha",
        }
    }
}

/// Aesthetic families group related aesthetics that share a scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AestheticFamily {
    HorizontalPosition,
    VerticalPosition,
    Color,
    Fill,
    Group,
    Alpha,
}

/// A mapping from a data variable to an aesthetic channel.
#[derive(Clone, Debug)]
pub struct Mapping {
    pub aesthetic: Aesthetic,
    pub variable: String,
}

/// A constant (hardcoded) visual value for an aesthetic channel.
#[derive(Clone, Debug)]
pub enum ConstantValue {
    Color([f32; 3]),
    Float(f64),
}

/// Parse a `#RRGGBB` hex string into a linear `[f32; 3]` RGB triple.
pub fn parse_hex_color(s: &str) -> Result<[f32; 3], String> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return Err(format!("Expected 6-digit hex color, got: #{}", s));
    }
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| e.to_string())?;
    Ok([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}
