# ScalePositionDiscrete

A discrete position scale that maps categorical string values to evenly-spaced positions along an axis. This is the positional analog of `ScaleColorDiscrete`.

## Motivation

Required for bar charts (`GeomBar`), and useful for any plot where the x-axis is categorical (e.g. dot plots, lollipop charts). Currently, only `ScalePositionContinuous` exists — it expects numeric input.

## Design

- Implements `Scale` trait
- Input: `RawColumn::StringArray` (categorical values)
- Output: `MappedColumn::UnitArray` — maps each unique category to an evenly-spaced `Unit::NDC` position
- Maintains category order (insertion order from data, or optionally sorted)
- Provides band width for bar geoms: `band_width() -> f32` (spacing between categories)

## Axis rendering

- Tick marks placed at category centers
- Labels are the category strings (may need rotation for long labels)
- No interpolation between categories — discrete jumps only

## Dependencies

- Blocking `GeomBar` story

## Status

Not started.
