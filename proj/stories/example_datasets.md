As a developer writing docs and examples, I want a curated set of real-world datasets so that examples demonstrate meaningful patterns rather than toy data.

# Motivation

Current examples mostly use synthetic CSVs (`data.csv`, `categories.csv`, etc.) with columns like `x,y,z`. These don't convey *why* you'd reach for a grammar-of-graphics tool. Real datasets with recognizable domains make examples self-motivating — a reader sees a scatterplot of GDP vs life expectancy and immediately understands the value.

# Candidate datasets

Aim for 5-8 datasets that collectively exercise every feature in the system. Each should be:
- Small enough to ship in-repo (under ~50KB, ideally under 10KB)
- Real or realistic data from a recognizable domain
- Licensed for redistribution (public domain, CC0, or permissive)
- Useful for multiple plot types / aesthetics

Suggestions (not prescriptive — pick what works):

| Dataset | Domain | Exercises |
|---------|--------|-----------|
| **Gapminder excerpt** | Economics / health | scatter, color, size (bubble chart), facet by continent, log scale (GDP) |
| **Palmer penguins** | Biology | scatter, shape + color (species), histogram (body mass), facet by island |
| **Iris** | Biology (classic) | scatter, color, shape, size — the "hello world" of viz |
| **NYC weather** | Meteorology | timeseries (line), bar (monthly precip), facet by month |
| **US state populations** | Demographics | already have `state_population.csv` — keep and expand if needed |
| **Diamond prices (sample)** | Economics | histogram, log scale, color continuous, scatter |
| **Wind directions** | Meteorology | polar coordinates (rose diagram), bar chart |
| **Student survey / mtcars-like** | General | small categorical dataset for bar charts, dodge/stack, stat count |

# Deliverables

1. Curated CSV files in `examples/data/` (or similar), with a `README.md` noting sources and licenses
2. Update existing `.gg` example files to reference real datasets where appropriate
3. Add new `.gg` examples that showcase features using the new datasets
4. Update snapshot tests for any changed examples

# Guidelines

- Prefer fewer, richer datasets over many narrow ones — a single dataset that works for 4 plot types is better than 4 single-purpose datasets
- Include a mix of numeric and categorical columns so the same dataset can demonstrate multiple aesthetics
- Keep row counts reasonable (100-500 rows) — enough to see patterns, small enough to ship
- Document the source and license for each dataset

# Dependencies

None, but should be completed before the docs site story.

# Status

Not started.
