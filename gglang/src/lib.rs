/* GGLANG ENGINE

This package can be used to create data visualizations. It provides a
declarative language for defining plots, and supports a portable rendering
system for broad compatibility.

## Architecture

### Compiling

#### Parser

A pest parser for the language.

#### AST

Abstract Syntax Tree representation of the language.

#### PlotSpec

A more structured representation of the AST, suitable for rendering.

### Rendering

Layer 1
    - Window
    - Layout
Layer 2
    - Text
    - Geometry
    - Transform
    - Theme
Layer 3
    - Axis
    - Point
    - Line
    - Bar
    - Title
    - Legend
Layer 4
    - Plot Spec (from compiler)


*/

pub mod app;
mod geometry;
mod frame;
mod plot;
pub mod transform;
