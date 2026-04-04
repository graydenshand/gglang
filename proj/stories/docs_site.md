As a new user discovering gglang, I want a documentation site that teaches me how to think in grammar of graphics and gives me detailed reference for every feature, so I can go from zero to productive without reading Rust source code.

# Motivation

Currently the only docs are CLAUDE.md (internal) and inline code comments. The GQL language needs its own documentation separate from Rust crate docs — the audience is analysts and data scientists, not Rust developers.

# Structure

## Conceptual guide ("Building plots with GQL")

Teach the grammar-of-graphics mental model, inspired by ggplot2's layered grammar paper and docs. Sections:

1. **What is a grammar of graphics?** — Layers, aesthetics, scales, stats, coords, facets as composable building blocks. Contrast with "chart type" thinking.
2. **Your first plot** — Walk through a scatter plot from CSV to rendered output. Explain each line of GQL.
3. **Aesthetics and mappings** — How `MAP` connects data columns to visual channels. Plot-level vs layer-level mappings. Constants vs mapped values.
4. **Layers** — How multiple `GEOM` statements compose. Layering points over lines, text over bars, etc.
5. **Scales** — How data values become visual values. Continuous vs discrete, auto-detection, explicit overrides. Log scale.
6. **Statistical transforms** — `stat_count`, `stat_bin` — how geoms can transform data before rendering.
7. **Coordinate systems** — Cartesian (default) vs Polar. How coords transform the entire plot.
8. **Faceting** — Small multiples with `FACET WRAP` and `FACET GRID`. Scale freedom.
9. **Theming** — Controlling appearance with `THEME`. File-based themes.
10. **Putting it together** — A complex, multi-layer example combining several features.

Each section should have runnable `.gg` examples with rendered output (SVG screenshots).

## Reference

Detailed pages for each component:

- **Geoms**: `GEOM POINT`, `GEOM LINE`, `GEOM BAR`, `GEOM TEXT`, `GEOM HISTOGRAM` — required/optional aesthetics, attributes, position adjustments, examples
- **Aesthetics**: `x`, `y`, `color`, `fill`, `shape`, `size`, `alpha`, `group`, `label` — what they control, which geoms support them, constant vs mapped
- **Scales**: each scale type with domain/range behavior, auto-detection rules, explicit syntax
- **Stats**: `stat_count`, `stat_bin` — what they compute, when they're applied, interaction with geoms
- **Coords**: `CARTESIAN`, `POLAR` — how they transform rendering, axis behavior
- **Facets**: `WRAP` vs `GRID`, scale freedom options, column configuration
- **Theme**: every settable field with type, default value, and visual example
- **Data format**: CSV requirements, column type detection, missing values

## Gallery

A curated collection of example plots organized by chart type (scatter, line, bar, histogram, pie, radar, bubble, faceted, etc.), each with the `.gg` source, dataset, and rendered output. This is the "I want to make something like X" entry point.

# Implementation notes

- **Tooling**: mdBook, Zola, or similar static site generator. mdBook is simple and Rust-native. Preference TBD.
- **Rendered examples**: use `cargo run --bin plot` to generate SVG/PNG from each example as part of the docs build. Consider a script that renders all `docs/examples/*.gg` files.
- **Location**: `docs/` directory in-repo, deployable to GitHub Pages or similar.
- **Crate docs**: Rust API docs (`cargo doc`) remain separate and target library consumers. The docs site targets GQL authors.

# Dependencies

- Example datasets story (for meaningful examples throughout the docs)
- All v0.1.0 features (docs should cover the full feature set at release)

# Status

Not started.
