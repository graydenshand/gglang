# Default mappings

```jsx
MAP x=:year, y=:sales
```

This statement maps the plot parameter year to the x aesthetic, and parameter sales to the y aesthetic. These mappings apply to all layers of the plot unless overridden explicitly by an individual layer.

Users can define implicit default mappings by using an aesthetic name as the key of a plot parameter. For example, if a user provides a parameter named x, it will automatically be mapped to the x aesthetic.


# Layers

A layer consists of a geometry, a statistical transform, and a position transform.

Layers are defined using `GEOM` statements. This example adds a point layer that uses the plot's default mappings.

```jsx
GEOM POINT
```

This example sets the x and y aesthetics explicitly.

```jsx
GEOM POINT { x=:year, y=:sales }
```

# Scales

```jsx
SCALE_X_CONTINUOUS
```

```jsx
SCALE_COLOR_DISCRETE
```
