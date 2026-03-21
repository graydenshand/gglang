# Examples

Commands to run each example:

```bash
# Scatter plot (basic)
cargo run --bin plot examples/scatter.gg examples/iris-mock.csv

# Scatter plot with color by species
cargo run --bin plot examples/scatter_color.gg examples/iris-mock.csv

# Scatter plot with custom title, caption, and axis labels
cargo run --bin plot examples/scatter_custom_labels.gg examples/iris-mock.csv

# Bar chart (count by region)
cargo run --bin plot examples/bar.gg examples/bar_data.csv

# Stacked bar chart
cargo run --bin plot examples/bar_stacked.gg examples/bar_data.csv

# Dodged bar chart
cargo run --bin plot examples/bar_dodge.gg examples/bar_data.csv

# Dodged bar chart (count)
cargo run --bin plot examples/bar_dodge_count.gg examples/bar_data.csv

# Bar chart with count and fill
cargo run --bin plot examples/bar_count_fill.gg examples/bar_data.csv

# Categorical scatter plot
cargo run --bin plot examples/categorical.gg examples/categorical.csv

# Discrete year bar chart
cargo run --bin plot examples/discrete_year.gg examples/discrete_year.csv

# Timeseries line chart
cargo run --bin plot examples/timeseries.gg examples/stocks.csv

# Faceted timeseries
cargo run --bin plot examples/timeseries_facet.gg examples/state_population.csv
```

Append `--output out.svg` or `--output out.png` to any command to export to a file instead of opening an interactive window.
