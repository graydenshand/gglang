As an analyst, I want to create bar charts to visualize categorical data or compare values across groups.

Bar charts are a fundamental chart type missing from the current geometry set. This requires a new `GeomBar` geometry, a stat transform for counting (histogram-like mode), and position adjustments (stack, dodge).

# Examples

Counted bar chart (stat = count):

```gg
MAP x=:category
GEOM BAR
```

Pre-summarized bar chart (stat = identity):

```gg
MAP x=:category, y=:total
GEOM BAR
```

Stacked bar chart with color:

```gg
MAP x=:year, y=:sales, color=:region
GEOM BAR
```

Dodged bar chart:

```gg
MAP x=:year, y=:sales, color=:region
GEOM BAR DODGE
```

# Implementation notes

- **Geometry**: `GeomBar` implementing `Geometry` trait. Required aesthetic: `x`. Optional: `y` (if absent, stat = count), `color` (stacking/dodging).
- **Stat transform**: `StatCount` groups by x (and optionally color) and counts rows. `StatIdentity` passes y values through.
- **Position adjustment**: `PositionStack` (default) stacks bars; `PositionDodge` places bars side-by-side. These are general-purpose and will be reusable for other geoms.
- **Rendering**: bars are `Element::Rect` — already supported in the render pipeline. Need to compute bar width based on number of categories and any dodge groups.
- **Scale**: x-axis needs a discrete position scale (`ScalePositionDiscrete`) to map categorical values to evenly-spaced positions. This is a new scale type.

# Dependencies

- `ScalePositionDiscrete` (new scale type, could be a separate issue)

# Status

Not started.
