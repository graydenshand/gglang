As a web developer, I want to embed gglang plots in a browser so I can use it in web applications, dashboards, or a web-based version of the data-viz studio.

# Motivation

A WASM target makes gglang usable anywhere there's a browser — no native install required. It also opens the door to a web-based studio UI (via Tauri or pure web).

# Implementation notes

- **wgpu already supports WebGPU/WebGL** — the rendering pipeline should mostly work via `wgpu` on WASM
- `app.rs` has an `unimplemented!("WASM target requires...")` stub — this is the entry point
- **Data passing**: accept data via JS interop (wasm-bindgen). Accept JSON, Arrow IPC, or typed arrays.
- **Canvas integration**: render to an HTML `<canvas>` element. The wgpu WASM backend handles this.
- **Bundle size**: may need feature flags to exclude unnecessary dependencies for web builds

# Approach

1. Get `cargo build --target wasm32-unknown-unknown` compiling
2. Wire up winit's web event loop (or use raw canvas events)
3. Expose a JS API via wasm-bindgen: `plot(gql_string, data_json) → canvas`
4. Package with wasm-pack for npm distribution

# Dependencies

- Stable library API
- May benefit from SVG export as a fallback for browsers without WebGPU

# Status

Not started. The `unimplemented!` stub in `app.rs` marks the entry point.
