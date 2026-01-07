# Text

Ideally, text would just be another `Shape`. However, the `Shape` trait implements
`vertices()` and `indices()` methods which aren't compatible with the `wpgu_text`
`TextSection` elements from the library I am using. These `TextSection` elements
need to be rendered using a different method.

Two options to manage this.

A: One is to create an enum `Element` that is either a `Shape` or `Text` (or possibly
other variants in the future).

B: Another is to modify the trait `Shape` with a new method `text()` that returns any
text sections associated with the shape.

TODO: which is best?
* Do you expect any other types of elements beyond shapes and text?
* Does it conceptually make sense for the same object to have geometry and text?
* Impact to code base, how hard would this be to change later?

The enum seems to be a tighter solution.

# Foray into Theme

I added a `window_margin` theme setting to constrain the view area of the plot
somewhat.

In so doing, I changed the relationship between BluePrint and Theme. Instead of
owning a Theme, the blueprint now borrows it.

This inversion is useful because the theme can affect things beyond the scope
of the plot, such as window margin and background color. Additionally, a Window
could theoretically contain several plots, and you would most likely want to
use the same theme for each of them.
