# gglang / ggc

A ggplot2-inspired statistical graphics engine written in Rust, using wgpu for GPU-accelerated rendering. The system has two components: a DSL compiler for a language called GQL (Grammar of Graphics Language), and a rendering engine that produces plots from a `Blueprint` specification.

## Project goals

- A declarative, language-independent visualization definition (GQL)
- High-performance native rendering suitable for large datasets and interactivity
- Grammar-of-graphics compositional model (layers, scales, aesthetics, stats, facets)
- Deviations from ggplot2: data is decoupled from the plot definition (passed at render time, not embedded); lower-level channel mappings (e.g. `width`/`height` separate from `size`); less reliance on tidy data

## Current state

The renderer can display a hardcoded scatterplot (4 data points, `GeomPoint`, X/Y continuous scales, axis lines with max-value labels). The parser reads `.gg` files and identifies statement types but produces no structured output. **The parser and renderer are not yet connected.**

## Module map

| File | Role |
|------|------|
| `src/main.rs` | Parser binary — reads `.gg` file, identifies statement types |
| `src/bin/plot.rs` | Renderer binary — launches wgpu window |
| `src/app.rs` | wgpu window, surface, event loop, `AppState` |
| `src/frame.rs` | Bridges `Blueprint` to GPU — vertex/index buffers, text queuing. Contains **hardcoded demo data** (lines 42-64) |
| `src/plot.rs` | Core domain model: `Blueprint`, `Layer`, `Geometry` trait, `Scale` trait, `Aesthetic`, `PlotData`, `Theme` |
| `src/shape.rs` | GPU primitives: `Vertex`, `Unit` enum, `WindowSegment`, `Shape` trait, `Rectangle`, `Text`, `Element` enum |
| `src/transform.rs` | `ContinuousNumericScale` — linear interpolation between ranges |
| `src/layout.rs` | Non-compiling stub — future layout tree |
| `src/shader.wgsl` | Pass-through WGSL vertex + fragment shaders |
| `src/grammar.pest` | Pest grammar for GQL |

## Architecture

### Rendering pipeline

```
Raw data → Scale::map() → Unit::NDC   (data domain → NDC -1..1)
         → WindowSegment::abs_x/y()   (relative NDC → segment clip space)
         → Vertex position            (passed through shader)
```

### Coordinate system

- `Unit` enum: `Pixels(u32)`, `NDC(f32)`, `Percent(f32)` — polymorphic coordinate value
- `WindowSegment`: a rectangular sub-region of the window, holds NDC and pixel scales for both axes. `with_margin()` creates a sub-segment.
- All vertex positions are in NDC (clip space) by the time they reach the shader.

### Key abstraction boundary

`Blueprint::render(PlotData) -> Vec<Element>` is the clean seam between the plot model (domain logic, scales, geoms) and the rendering backend (wgpu). Keep wgpu types out of `plot.rs`.

## GQL language syntax

```
MAP :year TO x, :sales TO y    // default mappings
GEOM POINT                     // layer with default mappings

GEOM POINT { x=:year, y=:sales }  // layer with inline mappings (preferred shorthand)

SCALE X_CONTINUOUS
FACET BY :store
TITLE "My plot"
```

Data variables are referenced with `:` prefix. `MAP` sets plot-level defaults; geom-level `{ }` overrides per-layer.

## Key architectural decisions

- **Theme is borrowed, not owned** by `Blueprint` — themes affect things beyond the plot scope (window margin, background) and may be shared across multiple plots.
- **`Element` enum** (`Shape | Text`) unifies geometry and text at the render boundary — the right choice over a combined `Shape` trait with a `text()` method.
- **`Mapping` is currently `enum { X(String), Y(String) }`** — needs to be generalized to support all aesthetics (see `proj/issue-scale-generalization.md`).

## Issues and project planning

Open architectural issues are in `proj/`:
- `issue-ast-bridge.md` — connecting parser to renderer (highest priority)
- `issue-scale-generalization.md` — eliminating scale duplication
- `issue-layout-tree.md` — tree-based layout for axes/legends/facets
- `issue-render-backend-abstraction.md` — decoupling geom logic from wgpu
- `issue-plotdata-typing.md` — stronger typing through the data pipeline
- `issue-shader-architecture.md` — view transform uniform, instancing, SDF points

Active work tracked in `proj/backlog.md`. Design notes and language examples in `docs/` and `proj/ideas/`.

## Running

```bash
cargo run --bin plot     # renderer (hardcoded demo scatterplot)
cargo run -- path.gg    # parser (prints statement types)
```
