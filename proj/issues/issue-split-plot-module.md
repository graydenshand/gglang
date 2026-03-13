# Split plot.rs into focused modules

`plot.rs` is the largest file in the codebase and serves as the core domain model. As we add more geometries and scales, it will only grow. Splitting it into focused modules improves navigability and makes it easier to add new geom/scale types without touching unrelated code.

## Proposed split

| New module | Contents moved from `plot.rs` |
|---|---|
| `src/geom.rs` | `Geometry` trait, `GeomPoint`, `GeomLine`, and future geom implementations |
| `src/scale.rs` | `Scale` trait, `ScalePositionContinuous`, `ScaleColorDiscrete`, and future scale types |
| `src/aesthetic.rs` | `Aesthetic`, `AestheticFamily`, `Mapping`, and related enums/structs |
| `src/plot.rs` | `Blueprint`, `Layer`, `PlotData`, `RawColumn`, `MappedColumn`, `AesData`, `ResolvedData` — the orchestration layer |

## Constraints

- All these modules remain backend-agnostic (zero wgpu/winit imports)
- Public API stays the same — `lib.rs` re-exports everything
- Tests move with their code
- `Blueprint::render()` stays in `plot.rs` as the orchestrator

## Status

Not started.
