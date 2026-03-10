# Epic: End-to-end scatterplot demo

**Goal:** Render a scatterplot from a `.gg` source file and a CSV data file, with no hardcoded data in the binary.

**Target GQL** (`examples/scatter.gg`):
```
MAP :x TO x, :y TO y
GEOM POINT
```

**Target invocation:**
```bash
cargo run --bin plot -- examples/scatter.gg examples/scatter.csv
```

Changing either file changes the rendered output without recompiling.

---

## Milestones

### M1 — AST types and pest → AST
**Ticket:** `issue-ast-bridge.md` (steps 1–2: define AST types, implement `pest Pairs → AST`)

Scope for this milestone: `AstProgram`, `AstStatement`, `AstMap`, `AstGeom` types only. Replace the `println!` loop in `main.rs` with a structured parse result.

**Done when:** `cargo run -- examples/scatter.gg` prints a structured AST representation.

---

### M2 — CSV data loading
**Ticket:** `issue-csv-data-loading.md`

Load a CSV file into `PlotData` via a new `src/data.rs` module. First row is headers, all values as `f64`.

**Done when:** A CSV file can be loaded into a `PlotData` with named `FloatArray` columns.

---

### M3 — AST → Blueprint compilation
**Ticket:** `issue-ast-bridge.md` (step 3: implement `AST → Blueprint`)

Compile the parsed AST into a `Blueprint` with `Layer`, `Mapping`s, and auto-added `ScaleXContinuous`/`ScaleYContinuous`. Scope limited to `MAP` and `GEOM POINT` statements.

**Done when:** `compile(&ast)` returns an equivalent `Blueprint` to the one currently hardcoded in `frame.rs`.

---

### M4 — Wire up the renderer binary
**Ticket:** `issue-ast-bridge.md` (step 4: end-to-end wiring)

- `Frame::new()` accepts `Blueprint` and `PlotData` as parameters; remove hardcoded data (lines 42-64 of `frame.rs`)
- `bin/plot.rs` reads CLI args, calls `parse()` → `compile()` → `load_csv()` → renders

**Done when:** `cargo run --bin plot -- examples/scatter.gg examples/scatter.csv` renders the scatterplot from the CSV.

---

### M5 — Axis tick marks and labels
**Ticket:** `issue-axis-ticks.md`

Add 5 evenly spaced tick marks with numeric labels to both axes. Requires adding `ticks(n)` to `ContinuousNumericScale`.

**Done when:** Both axes show labeled tick marks at reasonable intervals.

---

## Out of scope

These issues exist but are deliberately deferred beyond the POC:

- `issue-scale-generalization.md` — scale refactor and `Mapping` generalization
- `issue-layout-tree.md` — proper axis gutters and legend regions
- `issue-render-backend-abstraction.md` — multi-backend rendering
- `issue-plotdata-typing.md` — stronger pipeline typing
- `issue-shader-architecture.md` — view transform, instancing, SDF points

---

## Example files

**`examples/scatter.gg`**
```
MAP :x TO x, :y TO y
GEOM POINT
```

**`examples/scatter.csv`**
```
x,y
0.5,1.2
1.0,3.4
1.5,2.1
2.0,4.8
2.5,3.9
3.0,5.5
3.5,4.2
4.0,6.1
```

---

## Success criteria

- [ ] No hardcoded data anywhere in the binary
- [ ] Changing `scatter.csv` changes the rendered plot without recompiling
- [ ] Changing mappings in `scatter.gg` (e.g. swapping x and y) changes the plot without recompiling
- [ ] Both axes show labeled tick marks
- [ ] Full pipeline runs: `.gg` → parse → AST → compile → Blueprint → render → window
