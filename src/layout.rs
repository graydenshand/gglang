/* Arrange elements on the window. 



The window is a rectangle. Positions within it are accessed via normalized
device coordinates (NDC), where the center of the window is (0, 0), the top
right is (1, 1), and the bottom left is (-1, -1).

A visualization requires arranging many elements precisely within the window, at
multiple levels. For example, at a granular level, individual points in a 
scatter plot must be drawn along with axes ticks and labels. At a higher level,
multiple plots may be arranged on the window, along with supplementary elements
like lengends and titles.

A unifying principle of this system is that of granting users a high degree of
flexibility.

Low level plot elements are best placed using coordinates. However, these
coordinates must be relative to that plot element, rather than global for the
entire window.

There are two common frameworks for higher level window layout:
- Grid: N column grid, with rows
- Flex: more fluid / dynamic based on element sizes

No need to decide right now, focus on low level first. Wrap with a plot element
to capture relative positioning and sizing.


Grid
Row
Span


FlexContainer
Item


Coordinate system:
- NDC
- PIXELS - (0,0) top left

Using NDC is needed to ensure the plot looks the same on every screen.
Pixel definitions are nice for things like the size of a square.

For points on a scatter plot, need to go from raw data like (50, 20) to window
coordinates like (-0.25, 0.1).
- Must consider total scale of axes (i.e. range of data points)
- Must consider parent containers relative position on window
- Must consider margin

*/



struct Container {
    elements: ...,
    position: ,
}