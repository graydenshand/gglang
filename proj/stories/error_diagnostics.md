As an analyst writing GQL, I want clear error messages with location information so I can quickly fix mistakes in my plot definitions.

# Motivation

Currently, parse errors surface raw Pest failure messages and compile/data errors lack source location context. For a v0.1.0 release, error messages should be helpful enough that a new user can self-diagnose common mistakes.

# Examples of desired behavior

**Unknown aesthetic:**
```
error: unknown aesthetic 'colour' at line 2
  MAP x=:year, y=:sales, colour=:region
                          ^^^^^^
  help: did you mean 'color'?
```

**Missing column in data:**
```
error: column 'sales' not found in data
  MAP x=:year, y=:sales
                  ^^^^^^
  available columns: year, revenue, region
```

**Invalid geom type:**
```
error: unknown geom type 'SCATTER' at line 3
  GEOM SCATTER
       ^^^^^^^
  help: available geoms: POINT, LINE, BAR, TEXT, HISTOGRAM
```

**Type mismatch:**
```
error: scale type mismatch at line 4
  SCALE Y DISCRETE
  column 'sales' is numeric — use SCALE Y CONTINUOUS or map a string column
```

# Scope

## Parse errors
- Wrap Pest errors to include the source line and a caret pointing to the error position
- Translate common Pest "expected ..." messages into user-friendly language

## Compile errors
- Name the offending statement/line when a mapping references an unknown aesthetic
- Name the offending geom when a required aesthetic is missing (e.g. `GeomText` without `label`)
- Report conflicting or duplicate statements clearly

## Data errors
- When a mapped column doesn't exist in the CSV, list available columns
- When a column type doesn't match the expected scale, explain the mismatch

## Render errors
- When a scale receives unexpected data (e.g. negative values for log scale), report the column and value

# Implementation notes

- **AST spans**: the Pest parser already tracks `Span` for each rule. Thread `(line, col)` through `Statement` and `AstAesthetic` so compile errors can reference source locations.
- **Error display**: implement `Display` for `GglangError` variants with formatted, multi-line output including the source line, caret, and help text.
- **Suggestions**: for unknown aesthetics/geoms, use edit distance to suggest corrections.
- **Column listing**: data errors should include the available column names from the loaded CSV.

# Dependencies

None. Orthogonal to all feature work.

# Status

Not started.
