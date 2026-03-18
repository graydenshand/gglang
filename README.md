# Grammar of Graphics

A ggplot2-inspired statistical graphics engine written in Rust, using wgpu for GPU-accelerated rendering. The system has two components: a DSL compiler for a language called GQL (Grammar of Graphics Language), and a rendering engine that produces plots from a `Blueprint` specification.

## Goals

- A declarative, language-independent visualization definition (GQL)
- High-performance native rendering suitable for large datasets and interactivity
- Grammar-of-graphics compositional model (layers, scales, aesthetics, stats, facets)

## Running

```bash
cargo run --bin plot file.gg data.csv   # compile + render
cargo run -- path.gg                    # parser (prints statement types)
```

## gglang

gglang is a language based on the ggplot2 API and the grammar of graphics.

```
MAP x=:x, y=:y
GEOM POINT

MAP x=:day, y=:price, group=:ticker, color=:ticker
GEOM LINE
```

# Reference

[1] https://byrneslab.net/classes/biol607/readings/wickham_layered-grammar.pdf
