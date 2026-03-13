# Geom attribute syntax (grammar + compiler)

Extend the grammar and compiler to support an optional attribute block on `GEOM` statements. This is the shared infrastructure needed by both the **per-layer mappings** and **hardcoded aesthetics** stories.

## Grammar change

```pest
geom_statement = { "GEOM" ~ geometry_type ~ geom_attributes? }
geom_attributes = { "{" ~ geom_attribute ~ ("," ~ geom_attribute)* ~ "}" }
geom_attribute = { aesthetic ~ "=" ~ (data_reference | literal_value) }
literal_value = { string_literal | number }
```

This allows both:
- `GEOM POINT { x=:year, y=:sales }` — per-layer mapping overrides
- `GEOM POINT { color="#0000FF", alpha=0.5 }` — hardcoded aesthetic values

## AST changes

- `GeomStatement` gains an `attributes: Vec<GeomAttribute>` field
- `GeomAttribute` is an enum: `Mapped(Aesthetic, String)` or `Constant(Aesthetic, LiteralValue)`

## Compiler changes

- `compile.rs` merges mapped attributes on top of plot-level defaults
- Constant attributes are stored on the `Layer` and passed through to geom rendering as fixed values (not scale-mapped)

## Status

Not started.
