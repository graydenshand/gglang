As an analyst, I might want to change the default color of the points on my scatterplot.

I.e instead of using black, I want to use blue points.

Currently, color can only be set by mapping it to a variable. Instead, we should allow the user to hardcode the color, which would set every point on that layer to that color.

# Examples

```gg
GEOM POINT { color = "#0000FF" }
```

Two layers with different colors.

```gg
GEOM POINT { x=:year, y=:store_a_revenue, color = "#0000FF" }
GEOM POINT { x=:year, y=:store_b_revenue, color = "#FF0000" }
```
