As someone with a small amount of data, its sometimes nice to just put it in a well formatted table.

Here's a hint for possible syntax
```gg
GEOM TABLE
	COLUMN :year
	COLUMN :market_cap TITLE "Market cap (in billions of dollars)"
	STRIPED
```

It's a deviation from the syntax used for other geometries; maybe it's not worth doing.

Alternatively, with a tidy row-col-value dataset you could do:

```gg
GEOM TABLE (x=:cols, y=:rows, label=:value)
```

# Future

Just to mention for architectural purposes (not to include now) it would be great to produce spark-lines in cells some day.
