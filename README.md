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

## Testing

Visual regression is guarded by [insta](https://insta.rs/) snapshot tests in `tests/svg_snapshots.rs`. Each test renders an example from `examples/` to SVG and diffs it against the stored snapshot in `tests/snapshots/`.

```bash
cargo test                  # run all snapshot tests
cargo insta test --review   # run tests, then interactively accept/reject any changed snapshots
cargo insta review          # review pending snapshots from a prior failed test run
```

`cargo insta review` only surfaces snapshots that *failed* on the last test run (pending `.snap.new` files). If all tests pass, it will correctly report "no snapshots to review" — use `cargo insta test --review` to force a run-then-review cycle.

To add coverage for a new example, drop the `.gg` + `.csv` pair into `examples/`, add a `snapshot_test!` line at the bottom of `tests/svg_snapshots.rs`, run `cargo test` (the new test will fail on first run), and accept the new snapshot with `cargo insta review`.

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
