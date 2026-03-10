Lower level channel mappings:

- Separate `size` into `width` and `height`
    - keep `size` as a way to uniformly scale width and height together
- Separate `color` into `hue`, `luminance` and `saturation`

Tilt / Angle channel mapping for things like wind maps

- Representing a uniform field
- Line tilt indicates direction
- Line color indicates magnitude

Unlike in ggplot, the dataset is detached and independent from the plot definition, mappings are defined using references and those references are mapped to actual data at render time.

Less focus on “tidy” data:

- Each data reference is an independent vector (technically a Nd matrix, but typically 1d)
    - e.g. an aesthetic like `edges` would expect 2d or 3d matrix for: node1, node2, label
