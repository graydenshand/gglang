As an analyst, I want to map a numeric variable to point size to create bubble charts where a third quantitative dimension is encoded visually.

# Examples

Bubble chart:

```gg
MAP x=:gdp, y=:life_expectancy, size=:population, color=:continent
GEOM POINT
```

Hardcoded size:

```gg
GEOM POINT { size=3.0 }
```

# Implementation notes

- **Aesthetic**: add `Aesthetic::Size` and `AestheticFamily::Size`
- **Scale**: `ScaleSizeContinuous` — maps a numeric range to a pixel-radius range (e.g. 2px–20px). Likely a sqrt scale by default so area is proportional to value.
- **GeomPoint**: the instanced SDF pipeline already sends a per-point radius. Currently hardcoded — needs to read from the `Size` mapped column in `ResolvedData`.
- **Legend**: size legend shows a few representative circle sizes with labels (similar to ggplot2's size legend)
- **Grammar**: add `size` to the `aesthetic` rule in `grammar.pest`

# Status

Not started.
