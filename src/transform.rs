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

/// Compute the "nice" step size for a given range and target tick count.
pub fn nice_step(min: f64, max: f64, target_count: usize) -> f64 {
    let range = max - min;
    let rough_step = range / target_count as f64;
    let magnitude = rough_step.log10().floor();
    let power = 10f64.powf(magnitude);
    let normalized = rough_step / power;
    if normalized <= 1.0 {
        power
    } else if normalized <= 2.0 {
        2.0 * power
    } else if normalized <= 5.0 {
        5.0 * power
    } else {
        10.0 * power
    }
}

/// Expand [min, max] to "nice" boundaries (multiples of the step size).
///
/// This guarantees that `nice_ticks(nice_min, nice_max, target_count)` always
/// produces a consistent number of ticks regardless of where the raw data
/// boundaries fall relative to the step grid.
///
/// Degenerate case (min == max): expands by ±10% of the absolute value,
/// or ±1 if the value is zero.
pub fn nice_bounds(min: f64, max: f64, target_count: usize) -> (f64, f64) {
    if min == max {
        let delta = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
        return (min - delta, max + delta);
    }
    let step = nice_step(min, max, target_count);
    let nice_min = (min / step).floor() * step;
    let nice_max = (max / step).ceil() * step;
    (nice_min, nice_max)
}

/// Compute "nice" tick values in the range [min, max].
///
/// Uses a standard nice-numbers algorithm: round the step to the nearest
/// 1, 2, or 5 × power of 10, then enumerate ticks from the first multiple
/// above min to max.
pub fn nice_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if min >= max || target_count == 0 {
        return vec![];
    }
    let step = nice_step(min, max, target_count);

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

    #[test]
    fn nice_bounds_typical() {
        let (lo, hi) = nice_bounds(3.0, 47.0, 5);
        assert_eq!(lo, 0.0);
        assert_eq!(hi, 50.0);
    }

    #[test]
    fn nice_bounds_unit_range() {
        let (lo, hi) = nice_bounds(0.0, 1.0, 5);
        assert_eq!(lo, 0.0);
        assert_eq!(hi, 1.0);
    }

    #[test]
    fn nice_bounds_degenerate() {
        // Should not panic and must produce a non-zero range
        let (lo, hi) = nice_bounds(5.0, 5.0, 5);
        assert!(hi > lo);
    }

    #[test]
    fn nice_bounds_degenerate_zero() {
        let (lo, hi) = nice_bounds(0.0, 0.0, 5);
        assert_eq!(lo, -1.0);
        assert_eq!(hi, 1.0);
    }

    #[test]
    fn nice_bounds_negative_range() {
        let (lo, hi) = nice_bounds(-15.0, 35.0, 5);
        assert_eq!(lo, -20.0);
        assert_eq!(hi, 40.0);
    }

    #[test]
    fn nice_ticks_on_nice_bounds_covers_full_range() {
        // After fitting with nice_bounds, nice_ticks should start exactly at nice_min
        // and end exactly at nice_max (i.e. bounds are aligned to step boundaries).
        let ranges = [
            (3.0_f64, 47.0_f64),
            (0.5, 9.5),
            (1234.0, 5678.0),
            (-15.0, 35.0),
            (100.0, 200.0),
        ];
        for &(lo, hi) in &ranges {
            let (nlo, nhi) = nice_bounds(lo, hi, 5);
            let ticks = nice_ticks(nlo, nhi, 5);
            assert!(!ticks.is_empty(), "no ticks for ({lo}, {hi})");
            let first = ticks[0];
            let last = *ticks.last().unwrap();
            assert!(
                (first - nlo).abs() < 1e-9,
                "first tick {first} != nice_min {nlo} for range ({lo}, {hi})"
            );
            assert!(
                (last - nhi).abs() < 1e-9,
                "last tick {last} != nice_max {nhi} for range ({lo}, {hi})"
            );
        }
    }
}
