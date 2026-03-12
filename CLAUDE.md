# gglang / ggc

A ggplot2-inspired statistical graphics engine written in Rust, using wgpu for GPU-accelerated rendering. The system has two components: a DSL compiler for a language called GQL (Grammar of Graphics Language), and a rendering engine that produces plots from a `Blueprint` specification.

## Project goals

- A declarative, language-independent visualization definition (GQL)
- High-performance native rendering suitable for large datasets and interactivity
- Grammar-of-graphics compositional model (layers, scales, aesthetics, stats, facets)
- Deviations from ggplot2: data is decoupled from the plot definition (passed at render time, not embedded); lower-level channel mappings (e.g. `width`/`height` separate from `size`); less reliance on tidy data

## Current state

The parser, compiler, and renderer are connected end-to-end. A `.gg` file and CSV are parsed, compiled into a `Blueprint`, and rendered via wgpu. Supported features: `GeomPoint` and `GeomLine` with X/Y continuous scales, `group` aesthetic for partitioning line series, color segmentation via `ScaleColorDiscrete` (categorical string column → HSL-spaced colors with legend), axis tick marks/labels, plot titles/captions/axis labels. CSV loading auto-detects numeric vs. string columns. A tree-based layout system gives each plot region (data area, axis gutters, title, legend, caption) its own `WindowSegment`, replacing the previous out-of-bounds NDC positioning.

## Module map

| File | Role |
|------|------|
| `src/main.rs` | Parser binary — reads `.gg` file, identifies statement types |
| `src/bin/plot.rs` | Renderer binary — launches wgpu window, loads `.gg` + CSV |
| `src/lib.rs` | Library root — re-exports modules |
| `src/ast.rs` | AST types and Pest parser: `Program`, `Statement`, `AstAesthetic`, `DataMapping` |
| `src/compile.rs` | Compiles AST `Program` into a `Blueprint` — wires mappings, layers, scales |
| `src/data.rs` | CSV loader — auto-detects numeric (`FloatArray`) vs. string (`StringArray`) columns |
| `src/app.rs` | wgpu window, surface, event loop, `AppState` |
| `src/frame.rs` | Bridges `PlotOutput` to GPU — resolves layout tree, projects per-region elements through their `WindowSegment`, builds vertex/index buffers, queues text |
| `src/layout.rs` | Layout system: `Unit`, `WindowSegment` (with `slice_x`/`slice_y`), `PlotRegion`, `LayoutNode`, `SizeSpec`, `SplitAxis`, `PlotOutput`, `standard_plot_layout()` |
| `src/plot.rs` | Core domain model: `Blueprint`, `Layer`, `Geometry` trait, `Scale` trait, `Aesthetic`/`AestheticFamily` enums, `ScalePositionContinuous`, `ScaleColorDiscrete`, `GeomPoint`, `GeomLine`, `PlotData`, `Theme` |
| `src/shape.rs` | GPU primitives: `Vertex`, `Shape` trait, `Rectangle`, `LineSegment`, `Text`, `Element` enum (imports `Unit`/`WindowSegment` from `layout`) |
| `src/transform.rs` | `ContinuousNumericScale` — linear interpolation between ranges |
| `src/shader.wgsl` | Pass-through WGSL vertex + fragment shaders |
| `src/grammar.pest` | Pest grammar for GQL |

## Architecture

### Rendering pipeline

```
Raw data → Scale::map() → Unit::NDC        (data domain → NDC -1..1)
Blueprint::render()      → PlotOutput       (elements partitioned by PlotRegion + LayoutNode tree)
Frame::new()             → LayoutNode::resolve()  (layout tree + root segment → HashMap<PlotRegion, WindowSegment>)
                         → WindowSegment::abs_x/y()  (region-local NDC → absolute clip space)
                         → Vertex position   (passed through shader)
```

### Layout system

- `Unit` enum: `Pixels(u32)`, `NDC(f32)`, `Percent(f32)` — polymorphic coordinate value
- `WindowSegment`: a rectangular sub-region of the window, holds NDC and pixel scales for both axes. `with_margin()` creates a sub-segment. `slice_x()`/`slice_y()` subdivide along an axis.
- `LayoutNode`: tree of `Leaf(PlotRegion)` and `Split { axis, children: Vec<(SizeSpec, LayoutNode)> }`. Resolved against a root `WindowSegment` to produce per-region segments.
- `PlotRegion`: `DataArea`, `XAxisGutter`, `YAxisGutter`, `Title`, `Legend`, `Caption`, `Spacer`
- `PlotOutput`: `{ regions: HashMap<PlotRegion, Vec<Element>>, layout: LayoutNode }` — returned by `Blueprint::render()`, consumed by `Frame::new()`
- All vertex positions are in NDC (clip space) by the time they reach the shader.

### Key abstraction boundary

`Blueprint::render(PlotData) -> PlotOutput` is the clean seam between the plot model (domain logic, scales, geoms) and the rendering backend (wgpu). Keep wgpu types out of `plot.rs`.

## GQL language syntax

```
MAP x=:year, y=:sales              // default mappings
GEOM POINT                         // layer with default mappings
MAP x=:year, y=:sales, color=:region  // with color segmentation
GEOM POINT

MAP x=:day, y=:price, group=:ticker, color=:ticker
GEOM LINE                          // timeseries line plot

SCALE X_CONTINUOUS
FACET BY :store
TITLE "My plot"
```

Data variables are referenced with `:` prefix. `MAP` sets plot-level defaults; geom-level `{ }` overrides per-layer.

## Key architectural decisions

- **Theme is borrowed, not owned** by `Blueprint` — themes affect things beyond the plot scope (window margin, background) and may be shared across multiple plots.
- **`Element` enum** (`Shape | Text`) unifies geometry and text at the render boundary — the right choice over a combined `Shape` trait with a `text()` method.
- **`Mapping` is a struct** `{ aesthetic: Aesthetic, variable: String }` — extensible to any aesthetic channel. `Aesthetic` and `AestheticFamily` are enums, not traits.

## Issues and project planning

Open architectural issues are in `proj/issues/`:
- ~~`issue-layout-tree.md`~~ — ✅ Done
- `issue-render-backend-abstraction.md` — decoupling geom logic from wgpu
- `issue-plotdata-typing.md` — stronger typing through the data pipeline
- `issue-shader-architecture.md` — view transform uniform, instancing, SDF points

Active work tracked in `proj/backlog.md`. Design notes and language examples in `docs/` and `proj/ideas/`.

## Running

```bash
cargo run --bin plot file.gg data.csv   # compile + render
cargo run -- path.gg                    # parser (prints statement types)
```
