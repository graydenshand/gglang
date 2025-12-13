This example shows how composition can be used to create reusable components.

`sales.xml` defines a frame that contains a single plot - a line plot that
shows `year` on the X axis, and `sales` on the Y axis.

Consider that you had a custom theme that you wanted to reuse for many plots.
In this example, `demo-theme.xml` contains a `<Theme>` tag that defines several
style definitions. The `sales.xml` line plot is easily able to use this 
theme definition using the tag `<DemoTheme>` -- the engine links the source file
by matching the name of the tag to the name of the file.

## Variables

Variables can be passed down through the templates. `sales.xml` defines
sets the plots variables using the `:` prefix.

```xml
<Plot x=":x" y=":y" >
```

The program driving the plot will supply any configuration needed by the root
template.

## Children

This example demonstrates how child elements can be passed to extend or override
the definition in the template.

```xml
<DemoTheme>
    <BgColor>#000000</BgColor>
</DemoTheme>
```

Here, we add an additional `<BgColor>` tag to the `<DemoTheme>`, overriding the
setting in the `demo-theme.xml` template.