Histogram

```jsx
LAYER {
  DATA { x = :value }
  GEOM HISTOGRAM { bins = 20 }
}
```

<aside>
⛔

`LAYER` isn’t adding anything here

</aside>

Scatter

```jsx
MAP :year=x, :sales=y
GEOM POINT
```

 Scatter shorthand

```jsx
GEOM POINT { x=:year, y=:sales }
```

<aside>
🔥

Yeah, this is the right amount of typing for a scatterplot

</aside>

Even better, if you provide variables named according to channels (e.g. `x` and `y`), automatically set those as the default mappings. That would allow you to omit mappings from the plot entirely.

```jsx
GEOM POINT
```

Scatter with colored lines for each group 

```jsx
MAP :year TO x, :sales TO y
GEOM POINT

MAP :store TO group         // additional mappings can be either here
GEOM LINE {
	color=:store              // or here, only mapped for this statement
}

UNMAP group                 // clear mappings if needed for subsequent layers
```

<aside>
⛔

Is this too “procedural”? the way the default mapping changes depending on where you are in the script.

It’s fine for the language to enforce an order — after all SQL has a strict ordering of clauses. Also, it’s not really saying *how* to build the plot, more just describing what the plot is.

That said, it would be more consistent with ggplot to support just one `MAP` statement per plot — one “default” mapping. Everything beyond is inside specific geoms? disabling a default geom is done by setting it to `null` in the geom override.

</aside>

```jsx
MAP { x=:year, y=:sales }   // default mappings
GEOM POINT

GEOM LINE {
	color=:store                // additional mappings
}
```

<aside>
🔥

Much better.

</aside>

Bar chart, faceted

```jsx
GEOM BAR { x=:year, y=:sales }
FACET BY :store
```

Networks & Trees - how to declare links and layouts?

- Consider using a `links` aesthetic that takes a 2D array instead of a 1D array and captures relationships between items.
- Special geometries like `NETWORK` and `TREE` understand the links aesthetic and uses it to draw paths between nodes. Also implements position transformations for placing nodes in the tree or network.
    - Maybe they both use geom NETWORK, but tree uses stat TREE while network uses stat FORCE to figure out node positions

```jsx
MAP :links TO links
GEOM NETWORK DIRECTED
```

```jsx
MAP :links TO links, :labels TO label
GEOM TREE
```

Log scale

```jsx
GEOM POINT x=:year, y=:market_cap
SCALE Y_LOG
```

Table

```jsx
GEOM TABLE
	COLUMN :year
	COLUMN :market_cap TITLE "Market cap (in billions of dollars)"
	STRIPED
```

```jsx
MAP x=:year, y=:market_cap
GEOM POINT
	
vs

GEOM POINT { x=:year, y=:market_cap }

vs

GEOM POINT WITH 
```
