/// An aesthetic channel that maps data to a visual property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aesthetic {
    X,
    Y,
    Color,
    Group,
}

impl Aesthetic {
    pub fn all() -> &'static [Aesthetic] {
        &[
            Aesthetic::X,
            Aesthetic::Y,
            Aesthetic::Color,
            Aesthetic::Group,
        ]
    }

    pub fn family(&self) -> AestheticFamily {
        match self {
            Aesthetic::X => AestheticFamily::HorizontalPosition,
            Aesthetic::Y => AestheticFamily::VerticalPosition,
            Aesthetic::Color => AestheticFamily::Color,
            Aesthetic::Group => AestheticFamily::Group,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Aesthetic::X => "x",
            Aesthetic::Y => "y",
            Aesthetic::Color => "color",
            Aesthetic::Group => "group",
        }
    }
}

/// Aesthetic families group related aesthetics that share a scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AestheticFamily {
    HorizontalPosition,
    VerticalPosition,
    Color,
    Group,
}

/// A mapping from a data variable to an aesthetic channel.
#[derive(Clone, Debug)]
pub struct Mapping {
    pub aesthetic: Aesthetic,
    pub variable: String,
}
