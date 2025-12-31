Foray into Theme

I added a `window_margin` theme setting to constrain the view area of the plot
somewhat.

In so doing, I changed the relationship between BluePrint and Theme. Instead of
owning a Theme, the blueprint now borrows it.

This inversion is useful because the theme can affect things beyond the scope
of the plot, such as window margin and background color. Additionally, a Window
could theoretically contain several plots, and you would most likely want to
use the same theme for each of them.