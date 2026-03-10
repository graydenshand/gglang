As an analyst who likes working with scatterplots, I want more flexibility in how I can display them. One approach is to segement the scatterplot so that different groups of points are rendered differently.

There are different segementation techniques we can use:
1. Shape: the shape of the points denotes the class
2. Color: the fill color (hue) of the points denotes the class
3. Facet: each group of points is rendered in a separate sub-plot

# Implementation notes

This requires adding `shape` and `color` aesthetics to `GEOM POINT`. Both of these aesthetics require legends. Their scales are (ScaleShapeDiscrete, ScaleColorDiscrete).

Faceting syntax:

```
FACET BY :var [, ...] [ROWS 2 | COLUMNS 3] [TRANSPOSE]
```

- Can specify one or more facet variables
- Can specify either number of rows, number of columns, or auto layout
- Can flip facet direction by transposing
