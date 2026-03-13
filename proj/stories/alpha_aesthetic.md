As an analyst working with overlapping points, I want to control transparency (alpha) so I can see density patterns in overplotted scatterplots.

# Examples

Map alpha to a variable:

```gg
MAP x=:age, y=:income, alpha=:confidence
GEOM POINT
```

Hardcoded alpha for all points in a layer:

```gg
GEOM POINT { alpha=0.3 }
```

# Implementation notes

- **Aesthetic**: add `Aesthetic::Alpha` and `AestheticFamily::Alpha`
- **Scale**: `ScaleAlphaContinuous` — maps a numeric range to 0.0–1.0 alpha. Simple linear scale.
- **Rendering**: alpha blending is already enabled on all pipelines. The point instance data and line vertex data already carry color with an alpha component — just need to set it from the mapped column instead of hardcoding 1.0.
- **Grammar**: add `alpha` to the `aesthetic` rule
- Pairs well with the hardcoded aesthetics story for the constant-alpha use case

# Status

Not started.
