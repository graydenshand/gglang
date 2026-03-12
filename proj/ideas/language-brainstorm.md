Histogram

```gg
MAP x=:height
GEOM HISTOGRAM BINS 20
```

Scatter

```gg
MAP x=:year, y=:sales
GEOM POINT
```

Individual layer mappings

```gg
GEOM POINT (x=:year, y=:sales)
```

Even better, if you provide variables named according to channels (e.g. `x` and `y`), automatically set those as the default mappings. That would allow you to omit mappings from the plot entirely.

```gg
GEOM POINT
```

Scatter with colored lines for each group 

```gg
MAP x=:year, y=:sales   // default mappings
GEOM POINT

GEOM LINE (
	color=:store                // additional mappings
)
```

Bar chart, faceted

```gg
GEOM BAR ( x=:year, y=:sales )
FACET BY :store
```

Networks & Trees - how to declare links and layouts?

- Consider using a `links` aesthetic that takes a 2D array instead of a 1D array and captures relationships between items.
- Special geometries like `NETWORK` and `TREE` understand the links aesthetic and uses it to draw paths between nodes. Also implements position transformations for placing nodes in the tree or network.
    - Maybe they both use geom NETWORK, but tree uses stat TREE while network uses stat FORCE to figure out node positions

```gg
MAP links=:links
GEOM NETWORK DIRECTED
```

```gg
MAP links=:links, label=:labels
GEOM TREE
```

Log scale

```gg
GEOM POINT x=:year, y=:market_cap
SCALE Y_LOG_CONTINUOUS
```

Table

```gg
GEOM TABLE
	COLUMN :year
	COLUMN :market_cap TITLE "Market cap (in billions of dollars)"
	STRIPED
```

Glyph Field, e.g. for visualizing wind direction and speed through a field of arrows

```gg
MAP x=:latitude, y=:longitude, tilt=:wind_direction, length=:wind_speed
GEOM FIELD ARROWS
```
