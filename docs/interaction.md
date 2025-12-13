# Interaction

Data visualizations can be brought to life with animation and interaction.

How does our Grammar need to be modified and/or extended to support these concepts?

First and foremost, the rendering pipeline of a graphic changes from a linear
sequence to one that is circular. I.e. a render loop is constantly re-drawing
the plot with updated values.

Animation is just using time as another aesthetic mapping.
- E.g. a scatter plot whose points move every 1s to a new position
- Normally would be mapped to a temporal variable in the dataset, does it make
any sense to map a numeric or categorical value to the time channel?

Interaction provides the user with some control that can be used to alter the
state of the plot.
- A slider changes the value of some variable that the plot depends on (e.g. a filter on year)
- A set of checkboxes control the groups that are plotted
- Click and drag to shift the coordinate system (pan)
- Scroll to zoom (zoom)
- Does the plot designer need to be able to control how the interaction effects
    the data? Or are there sufficiently few patterns that this can be encapsulated
    declaratively?
- In some applications, you don't have all available data loaded upfront. Being
    able to hook into the render loop to customize how the plot data is updated
    for the particular frame seems useful.

All this interaction behavior could simply be a level above this library. i.e.
the animation loop just calls this library to render the frame with the correct
configuration for that time.