As a Python data scientist, I want to use gglang from Python so I can build plots using GQL syntax (or a Pythonic API) against dataframes and arrays without leaving my notebook or script.

# Motivation

Python is the dominant language for data analysis. Offering bindings makes gglang accessible to the largest audience of potential users and allows it to complement tools like pandas, polars, and DuckDB.

# Interface options

## Option A: GQL string API

```python
import gglang

gglang.plot("""
    MAP x=:year, y=:sales, color=:region
    GEOM POINT
    TITLE "Sales by year"
""", data=df)
```

Minimal surface area — just pass a GQL string and a dataframe/dict of arrays.

## Option B: Builder API

```python
from gglang import Plot, geom_point, aes

(Plot(df)
    .map(aes(x="year", y="sales", color="region"))
    .geom(geom_point())
    .title("Sales by year")
    .show())
```

More Pythonic, discoverable via autocomplete, but larger API surface to maintain.

## Option C: Both

Parse GQL strings for power users; builder API for discoverability. Builder compiles to the same `Blueprint` internally.

# Implementation notes

- **PyO3 + maturin**: standard Rust→Python binding toolchain. Expose `Blueprint`, `PlotData`, and a top-level `plot()` function.
- **Data ingestion**: accept `dict[str, list]`, pandas DataFrame, polars DataFrame, or Arrow arrays. Convert to `PlotData` (`HashMap<String, RawColumn>`) at the boundary.
- **Rendering**: for notebooks, render to SVG/PNG (depends on `issue-svg-export.md`) and return as IPython display object. For scripts, open a wgpu window or write to file.
- **Distribution**: publish as a pip-installable wheel via maturin. Prebuilt wheels for macOS/Linux/Windows.

# Dependencies

- SVG/PNG export (`issue-svg-export.md`) — needed for notebook inline display
- Stable public API — the Rust API should be relatively settled before binding it

# Status

Not started.
