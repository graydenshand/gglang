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

#### Compiler

Compiles the AST into a Blueprint, the domain model for rendering.

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

pub mod aesthetic;
pub mod app;
pub mod ast;
pub mod column;
pub mod compile;
pub mod data;
mod frame;
pub mod geom;
pub mod layout;
pub mod plot;
pub mod scale;
mod shape;
pub mod theme;
pub mod transform;
