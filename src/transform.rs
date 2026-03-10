/// A scale representing numeric data from some min to a max
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuousNumericScale {
    pub min: f64,
    pub max: f64,
}
impl ContinuousNumericScale {
    /// New from vec
    pub fn from_vec(values: &Vec<f64>) -> Self {
        let (xmin, xmax) = values.iter().fold((f64::MAX, f64::MIN), |(min, max), new| {
            (min.min(*new), max.max(*new))
        });

        Self {
            min: xmin,
            max: xmax,
        }
    }

    /// Does this scale contain the value
    pub fn contains(&self, value: f64) -> bool {
        self.min <= value && value <= self.max
    }

    /// Map a position on this scale to another scale.
    ///
    /// Extrapolates linearly for values outside [min, max].
    pub fn map_position(&self, other: &Self, position: f64) -> f64 {
        (position - self.min) / (self.max - self.min) * (other.max - other.min) + other.min
    }

    /// Translate a size in this scale to another scale
    pub fn map_size(&self, other: &Self, size: f64) -> f64 {
        size / self.span() * other.span()
    }

    /// Difference between max and min of this scale
    pub fn span(&self) -> f64 {
        return self.max - self.min;
    }

    /// The middle of this scale
    pub fn midpoint(&self) -> f64 {
        return self.min + self.span() / 2.;
    }

    /// Produce a new scale with the same midpoint, expanded or shrunk
    /// by a specified multiplier.
    ///
    /// For example:
    /// - Before: (0,10); span=10, midpoint=5;
    /// - Scale x10: (-45,55); span=100, midpoint=5;
    pub fn scale(&self, factor: f64) -> Self {
        let margin = self.span() * (factor / 2.);
        let midpoint = self.midpoint();

        Self {
            min: midpoint - margin,
            max: midpoint + margin,
        }
    }

    /// Reduce this scale on both sides by the specified margin
    pub fn shrink(&self, margin: f64) -> Self {
        assert!(margin > 0., "Margin: {margin}");
        Self {
            min: self.min + margin,
            max: self.max - margin,
        }
    }

    /// Expand this scale on both sides by the specified margin
    pub fn expand(&self, margin: f64) -> Self {
        assert!(margin > 0.);
        Self {
            min: self.min - margin,
            max: self.max + margin,
        }
    }

    /// Shift right by this amount
    pub fn shift(&self, by: f64) -> Self {
        Self {
            min: self.min + by,
            max: self.max + by,
        }
    }

    /// Produces a new scale with smallest min and largest max of the two and scales
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

pub const NDC_SCALE: ContinuousNumericScale = ContinuousNumericScale { min: -1., max: 1. };
pub const PERCENT_SCALE: ContinuousNumericScale = ContinuousNumericScale { min: 0., max: 100. };

/// Compute "nice" tick values in the range [min, max].
///
/// Uses a standard nice-numbers algorithm: round the step to the nearest
/// 1, 2, or 5 × power of 10, then enumerate ticks from the first multiple
/// above min to max.
pub fn nice_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if min >= max || target_count == 0 {
        return vec![];
    }
    let range = max - min;
    let rough_step = range / target_count as f64;
    let magnitude = rough_step.log10().floor();
    let power = 10f64.powf(magnitude);
    let normalized = rough_step / power;
    let step = if normalized <= 1.0 {
        power
    } else if normalized <= 2.0 {
        2.0 * power
    } else if normalized <= 5.0 {
        5.0 * power
    } else {
        10.0 * power
    };

    let first = (min / step).ceil() * step;
    let mut ticks = vec![];
    let mut tick = first;
    while tick <= max + step * 1e-10 {
        ticks.push(tick);
        tick += step;
    }
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_to() {
        let scale_a = ContinuousNumericScale { min: 0., max: 10. };
        let scale_b = ContinuousNumericScale {
            min: 100.,
            max: 200.,
        };
        assert_eq!(scale_a.map_position(&scale_b, 5.), 150.);

        let scale_b = ContinuousNumericScale { min: -1., max: 1. };
        assert_eq!(scale_a.map_position(&scale_b, 5.), 0.);
    }
}
