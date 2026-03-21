As an analyst, I want to annotate points on a plot with text labels so I can identify specific observations or add context directly on the chart.

### Examples

Label each point:

```gg
MAP x=:gdp, y=:life_expectancy, label=:country
GEOM TEXT
```

Points with labels (two layers):

```gg
MAP x=:gdp, y=:life_expectancy
GEOM POINT
GEOM TEXT { label=:country }
```

Hardcoded label:

```gg
GEOM TEXT { label="Threshold", x=0.5, y=100 }
```

### Implementation notes

- **Aesthetic**: add `Aesthetic::Label` and `AestheticFamily::Label`
- **Geometry**: `GeomText` implementing `Geometry` trait. Required: `x`, `y`, `label`. Optional: `color`, `size` (font size).
- **Rendering**: produces `Element::Text` positioned at mapped x/y coordinates. The text rendering path already exists in both GPU and SVG backends.
- **Offset/nudge**: a small default offset from the point position avoids overlapping the data mark. Could be a geom attribute (e.g. `GEOM TEXT { nudge_y=0.02 }`).
- **Grammar**: add `label` to the `aesthetic` rule in `grammar.pest`. `GeomText` parses as `GEOM TEXT`.

### Open questions

- Label collision/overlap avoidance is hard. Defer for now — just render all labels at their mapped positions.
- `GeomLabel` (text with a background rectangle) could be a variant or a separate geom. Defer.

### Status

Not started.
