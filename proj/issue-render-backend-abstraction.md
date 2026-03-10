# Abstract the render backend

## Problem

The rendering pipeline is tightly coupled to wgpu throughout. `Blueprint::render()` produces `Vec<Element>` which is consumed directly in `frame.rs` by wgpu-specific buffer construction. Text rendering uses `wgpu_text`/`glyph_brush` types embedded in `AppState`. The `Vertex` struct in `shape.rs` is defined with `wgpu::VertexBufferLayout`.

This coupling means:
- There is no path to static output (SVG, PNG) for producing files rather than displaying a window
- If you ever want a second rendering target (e.g., a headless test renderer, a web canvas, or SVG export), the entire frame/app layer needs to be rebuilt
- It's difficult to write unit tests for the rendering output of geoms and scales, because any test would require a wgpu device

wgpu is a strong choice for **interactive native rendering** — it's fast, cross-platform, and supports the GPU compute that makes large-dataset interactivity feasible. But the higher-level chart logic (geoms, scales, layout) doesn't need to know about wgpu.

## Suggested approach

The `Element` enum in `shape.rs` (line 330) already defines a natural abstraction boundary:
```rust
pub enum Element {
    Shape(Box<dyn Shape>),
    Text(Text),
}
```

The goal is to make `Blueprint::render() -> Vec<Element>` the backend-agnostic output, and have multiple consumers of it:

1. **wgpu backend** (current `frame.rs`) — converts `Element`s to vertex/index buffers for GPU rendering
2. **SVG backend** — converts `Element`s to SVG elements for static file output
3. **Test/inspector backend** — captures `Element`s for assertions in unit tests

`Shape` would need to become backend-agnostic too — instead of `vertices(&WindowSegment) -> Vec<Vertex>`, shapes would describe themselves in abstract geometric terms (bounding box, fill color, border) that each backend interprets in its own way.

This does not require removing wgpu — it means isolating wgpu to the backend layer.

## Key files

- `src/shape.rs` — `Element`, `Shape` trait, `Vertex`, `Rectangle`, `Text`
- `src/frame.rs` — wgpu backend (the current only consumer of `Element`s)
- `src/plot.rs` — `Blueprint::render()` — the backend-agnostic producer

## Priority

This is lower priority than the AST bridge, scale generalization, and layout tree. It becomes urgent if:
- You want static file output (SVG/PNG export)
- You want to write geom/scale unit tests without a GPU

For now, the wgpu coupling is acceptable. But keeping `Blueprint::render() -> Vec<Element>` as the clean seam to preserve means avoiding leaking wgpu types into `plot.rs`.
