Should this grammar support interactivity or just render the static plot?

**Pros**

1. Interacting with a vis can make it more useful
    1. Hover info (popover)
    2. Filtering
    3. Fish eye - explore in detail
    4. Zoom / pan

**Cons**

1. Especially for filtering, you often want to hook into that event to change the data being delivered to the plot — e.g. make a new database request based on some filters. It would be somewhat frustrating as a user to support the filtering functionality, but only support datasets that fit in memory.
2. With many more components, it makes laying out the visualization more complicated, and generally suggests building mechanisms to allow end users more control over the layout.
    1. This could be useful anyway. what if I want to put my title on the side? Could think about CSS flexbox & grid as examples.
        1. This would impact how your language works. I.e. order matters for position. Would ultimately bring it closer to HTML/CSS rather than SQL.
    2. But… why not just leave all of that to the UI frameworks that will ultimately use this engine. I.e. we put the data on the screen and produce legends. Everything else (title, caption, interactive elements, data fetching) is up to the code surrounding the plot.
        1. Maybe title and caption (for example) are covered to support simple use cases. More complex requirements can skip those and manage them at the UI level
    3. Some of this probably needs to be some of this anyway just to lay out small multiples (facet)
        1. But, doesn’t need to impact the language.

Maybe compromise, could support certain interactive elements that would fit cleanly into this framework.

E.g. No slider, or inputs, but maybe zoom + pan, fisheye, & hover info?

e.g.

```jsx
// scatter plot of fish length<>height, with fisheye 
MAP :fish_length TO x, :fish_weight TO y
GEOM point
ACTION FISHEYE
```

```jsx
// scatter plot of fish length<>height, with zoom + pan 
MAP :fish_length TO x, :fish_weight TO y
GEOM point
ACTION ZOOM
ACTION PAN
```

Could also implement rudimentary interactive elements (e.g. inputs for filtering) without supporting a complex arrangement API. This would support the functionality for simple use cases, without preventing someone from handling interactions in the UI layer for more complex use cases.

```jsx
// scatter plot of fish length<>height, with selector to filter on fish type
MAP :fish_length TO x, :fish_weight TO y
GEOM point
ACTION FILTER USING :fish_type
```

Time mapping

```jsx
// scatter plot of fish length<>height, with an animation m
MAP :fish_length TO x, :fish_weight TO y
GEOM point
ACTION PLAY USING :year TICK '1s'
```

# 3D

Having interaction support is necessary for complex 3d visualization. Eg.

- MRI data
- Globe
