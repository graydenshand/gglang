# Build the AST and parser-to-renderer bridge

## Problem

The parser (`main.rs`) and the renderer (`bin/plot.rs`) are completely disconnected. The parser reads `.gg` files and identifies statement types via pest, but produces no structured output. The renderer constructs a hardcoded `Blueprint` in `frame.rs` (lines 42-64). There is no way to go from a `.gg` source file to a rendered plot.

`lib.rs` documents the intended pipeline (Parser -> AST -> PlotSpec -> Renderer), but the AST and PlotSpec stages don't exist yet.

## Why this matters early

Every other feature (new geoms, scales, theming, facets) will need an end-to-end path to test and use. Without the bridge, the grammar and the rendering model evolve independently and risk diverging — you could define grammar constructs that don't map to the renderer, or build renderer features the grammar can't express.

## Suggested approach

1. **Define AST types** in a new `ast.rs` module — structs that directly represent parsed language constructs (e.g., `AstMapping`, `AstGeom`, `AstScale`, `AstThemeProperty`). These should mirror the grammar closely.

2. **Implement `pest Pairs -> AST`** — walk the pest parse tree and produce the AST. This replaces the current `main.rs` println loop.

3. **Implement `AST -> Blueprint`** — a lowering/compilation step that resolves an AST into a `Blueprint` with concrete `Layer`s, `Scale`s, and `Mapping`s. This is where validation happens (e.g., "geom_point requires x and y mappings").

4. **Wire it end-to-end** — the renderer binary loads a `.gg` file, parses it, compiles to Blueprint, and renders.

## Key files

- `src/main.rs` — current parser binary
- `src/grammar.pest` — pest grammar
- `src/plot.rs` — `Blueprint`, `Layer`, and related types
- `src/frame.rs` — hardcoded demo data (lines 42-64) to be replaced

## Open questions

- Should the AST support inline data (e.g., `DATA x = [1, 2, 3]`) or always reference external files?
- Should compilation errors be reported with source locations? (Pest provides span info that could be preserved in the AST.)
