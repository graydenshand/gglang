# Improve shader architecture for interactivity and performance

## Problem

The current shader (`shader.wgsl`) is a pass-through: the vertex shader emits clip-space positions unchanged, and the fragment shader outputs a flat color. All geometry is fully tessellated on the CPU before upload — each point becomes 4 vertices + 6 indices computed in `GeomPoint::render()` and baked into a static vertex buffer in `frame.rs`.

This approach has three concrete consequences:

1. **No interactivity is possible.** Pan and zoom would require re-tessellating and re-uploading the entire scene on every frame. Smooth 60fps interaction is infeasible.

2. **Poor scaling with data size.** 100k points = 400k vertices + 600k indices uploaded each frame. CPU tessellation is the bottleneck.

3. **Poor visual quality.** Points are rendered as hard-edged pixel rectangles (`GeomPoint` creates 16×16 pixel squares). No anti-aliasing, no shape variety (circles, triangles, etc.).

## Recommended changes

### 1. View transform uniform (required for interactivity)

Add a uniform buffer containing a view transform matrix. The vertex shader applies it to all positions, allowing pan and zoom without touching the vertex buffer:

```wgsl
struct View {
    transform: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> view: View;

// in vs_main:
out.clip_position = view.transform * vec4<f32>(model.position, 1.0);
```

Data positions are stored in data-space NDC (their current form). The transform maps that space to the visible viewport and is updated cheaply each frame via `queue.write_buffer()`. At identity, the view matches the current behavior exactly.

This unblocks zoom, pan, and any other view-level interactivity.

### 2. GPU instancing for point geoms

Replace per-point CPU tessellation with a single quad template + one instance per data point.

Instead of uploading 4 vertices per point, upload:
- A shared quad with 4 vertices (one time, reused for all points)
- A per-instance buffer of point positions and properties (color, size, shape)

The vertex shader reads instance data by `@builtin(instance_index)` and offsets the quad corners accordingly. The draw call becomes `draw_indexed(0..6, 0, 0..num_points)`.

This is a ~6× reduction in data transferred per point and eliminates CPU-side tessellation of points entirely. It's the primary reason to use wgpu over a simpler 2D renderer for large datasets.

### 3. SDF circle rendering in the fragment shader

Pass UV coordinates through the vertex shader (a `vec2<f32>` ranging 0..1 across the quad). In the fragment shader, compute a signed distance field to render anti-aliased circles — and eventually other point shapes:

```wgsl
// fragment shader
let dist = length(in.uv - vec2(0.5));
let alpha = 1.0 - smoothstep(0.45, 0.5, dist);
if alpha < 0.01 { discard; }
return vec4<f32>(in.color, alpha);
```

This gives smooth, resolution-independent circles with correct anti-aliasing at no CPU cost. Point shape (circle, square, triangle, diamond) can be parameterized as a per-instance integer with a corresponding SDF in the fragment shader.

Requires enabling alpha blending on the render pipeline (`BlendState::ALPHA_BLENDING` instead of `REPLACE`).

## What stays on the CPU

- **Scale transforms and data mapping** (`ContinuousNumericScale`, `Scale::map()`) — serial, stateful, depends on full data range
- **Layout computation** (`WindowSegment`, margins, axis regions) — cheap, highly stateful
- **Statistical transforms** (future `stat_smooth`, binning, density) — not suitable for GPU

## Separate pipelines per geom type

Different geoms have different rendering needs and should eventually use different `RenderPipeline`s:

| Geom | Pipeline |
|------|----------|
| `GeomPoint` | Instanced quads + SDF circle fragment shader |
| `GeomBar` | CPU-tessellated quads (current approach is fine) |
| `GeomLine` | Thin lines: triangle strips; thick lines with miters: CPU tessellation into trapezoid quads (hard problem, defer) |

wgpu supports multiple pipelines per frame. `Frame` would hold a pipeline per geom category rather than a single shared pipeline.

## Key files

- `src/shader.wgsl` — current pass-through shader
- `src/frame.rs` — pipeline construction and vertex buffer upload
- `src/shape.rs` — `Vertex` struct and `Shape` trait
- `src/plot.rs` — `GeomPoint::render()` (lines ~264-310), which currently tessellates points

## Suggested order of implementation

1. **View transform uniform** — self-contained change, immediately unblocks interactivity
2. **Alpha blending** — needed before SDF rendering, trivial to add
3. **SDF point rendering** — visual quality improvement, no data pipeline changes
4. **GPU instancing** — larger refactor touching `GeomPoint`, `Frame`, and the `Shape` trait; do after the above are stable

## Open questions

- Should the view transform live in `AppState` (updated by input events) or be derived from the `Blueprint`'s coordinate system?
- Should instanced data (positions, colors, sizes) be stored in a storage buffer (`@storage`) or a vertex buffer with `step_mode: Instance`? Storage buffers are more flexible but require wgpu feature flags on some targets.
