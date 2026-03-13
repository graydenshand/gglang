As an analyst exploring a dense scatterplot, I want to zoom and pan the data area so I can inspect regions of interest without re-rendering the entire plot.

The view transform uniform already exists in the shader (currently identity). This story activates it with mouse/trackpad input.

# Examples

```gg
MAP x=:longitude, y=:latitude, color=:category
GEOM POINT
ACTION ZOOM
ACTION PAN
```

Or potentially enabled by default / via a flag, without language syntax initially.

# Implementation notes

- **Input handling** (`app.rs`): capture scroll events (zoom) and click-drag (pan) within the data area `WindowSegment`
- **View transform**: update the `ViewUniform` buffer with a scale+translate matrix. Zoom scales around the cursor position; pan translates.
- **Bounds**: clamp zoom level (e.g. 1x–100x). Optionally clamp pan to data extents.
- **Axis sync**: tick marks and labels need to update to reflect the visible data range. This means either re-rendering axis elements on zoom/pan, or transforming them in the shader too.
- **Performance**: the uniform update is cheap (just a 4x4 matrix write). No re-tessellation needed for points. Lines may need re-tessellation at extreme zoom levels for miter join quality, but this can be deferred.

# Open questions

- Should zoom/pan be opt-in via language syntax (ACTION statement) or always-on?
- Should axis labels update dynamically, or remain fixed?

# Status

Not started.
