As an analyst, I want to control the visual appearance of my plots by overriding theme settings inline and by referencing a shared theme file, so that I can tune individual plots without repeating myself and enforce a consistent style across many plots (e.g. a company-wide theme).

# Examples

## Inline overrides

Override individual theme settings directly in a `.gg` file:

```gg
MAP x=:year, y=:population
GEOM POINT
TITLE "State populations"
FACET WRAP :state SCALES FREE Y
THEME {
    y_gutter_width = 100
    title_font_size = 36
    facet_label_font_size = 18
}
```

## File reference

Point to a `.ggtheme` file containing a full or partial theme:

```gg
MAP x=:year, y=:revenue
GEOM LINE { color=:region }
THEME FILE "company.ggtheme"
```

`company.ggtheme` contains key-value pairs (same syntax as the inline block):

```
title_font_size = 48
axis_color = "#333333"
facet_label_bg_color = "#E8F0FE"
window_margin_px = 20
```

## Stacking / inheritance

`THEME` clauses stack in order, with later settings winning. A file reference and inline overrides can be combined:

```gg
MAP x=:year, y=:revenue
GEOM LINE
THEME FILE "company.ggtheme"   // base: company-wide defaults
THEME {                         // plot-specific overrides on top
    title_font_size = 32
    facet_gap = 8
}
```

Multiple `THEME FILE` clauses are also allowed and stack in the same way.

# Settable fields

All fields on the `Theme` struct should be settable. Field names in the grammar mirror the Rust field names exactly to avoid a mapping layer. Color values are hex strings (`"#RRGGBB"` or `"#RRGGBBAA"`). Pixel values are plain integers. Float values (font sizes, ratios) are decimal literals.

# Implementation notes

- **Grammar**: add `theme_statement` supporting both `THEME { key=value ... }` and `THEME FILE "path"`. Allow multiple statements; they are applied left to right.
- **AST**: `ThemeStatement` enum — `Inline(Vec<(String, ThemeValue)>)` | `File(String)`. `ThemeValue` covers int, float, and color-string variants.
- **Compile**: accumulate `ThemeStatement`s on `Blueprint`. At render time, start from `Theme::default()` and apply each statement's overrides in order.
- **`.ggtheme` file format**: same key=value syntax as the inline block, one per line, `//` line comments. Parsed with a subset of the main grammar or a simple hand-rolled parser.
- **Theme struct**: no changes needed to `Theme` itself — the override mechanism sits entirely in the compiler/render path.

# Dependencies

None. Can be implemented independently.
