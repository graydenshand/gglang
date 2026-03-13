A local-first data visualization studio that combines DuckDB (query engine) and gglang (rendering engine) into a single interactive application.

# Vision

A lightweight desktop app where you:
1. Point it at local files (CSV, Parquet, JSON) or a database
2. Explore data with SQL (powered by DuckDB)
3. Visualize results with GQL (powered by gglang)
4. Iterate interactively — query, plot, refine, repeat

Think of it as a local alternative to Observable/Tableau/Grafana, but with a text-first interface (SQL + GQL) rather than drag-and-drop.

# Architecture

```
┌─────────────────────────────────────┐
│          Studio Application         │
│  ┌───────────┐    ┌──────────────┐  │
│  │  DuckDB   │───→│    gglang    │  │
│  │  (query)  │    │   (render)   │  │
│  └───────────┘    └──────────────┘  │
│        ↑                ↓           │
│  ┌───────────┐    ┌──────────────┐  │
│  │   Editor  │    │  Plot canvas │  │
│  │ (SQL+GQL) │    │   (wgpu)    │  │
│  └───────────┘    └──────────────┘  │
└─────────────────────────────────────┘
```

- **Separate crate/repo** — consumes gglang as a library dependency
- **DuckDB** via `duckdb-rs` — in-process OLAP engine, reads Parquet/CSV/JSON natively
- **UI**: could be egui (pure Rust, embeds wgpu), Tauri (web frontend + Rust backend), or a terminal UI
- **Data flow**: SQL query → DuckDB → Arrow columnar result → convert to `PlotData` → gglang `Blueprint::render()` → display

# What gglang needs to support this

1. **Stable library API** — `Blueprint`, `PlotData`, `PlotOutput` as the public interface
2. **SVG/PNG export** (`issue-svg-export.md`) — for saving figures, and for web-based UIs
3. **Arrow interop** — accept Arrow columnar arrays as input to avoid copying data from DuckDB. This could be a `From<ArrowArray> for RawColumn` impl or a thin adapter.
4. **Headless rendering** — render without opening a window (either SVG path or offscreen wgpu)
5. **Error handling** (`issue-error-handling.md`) — the studio needs structured errors to display to users, not panics

# Why separate from gglang

- gglang is a visualization engine/library — it should stay focused
- The studio is an application with its own UI, state management, file handling, and DuckDB dependency
- Keeping them separate means gglang remains embeddable in other contexts (Python, WASM, other apps)

# Phasing

1. First, build gglang into a solid library (current work)
2. Add SVG export + error handling
3. Build a minimal studio prototype — text editor + plot canvas, DuckDB for data
4. Iterate on the studio UX

# Status

Concept / future work. Documented here to inform architectural decisions in gglang (keep library API clean, support headless rendering, plan for Arrow interop).
