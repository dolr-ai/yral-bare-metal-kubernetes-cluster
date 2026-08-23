use leptos::html;
use leptos::prelude::*;
use leptos::*;

pub fn dynamic_attributes() -> impl IntoView {
    let (count, set_count) = signal(0);
    let double_count = move || count.get() * 2;

    (
        html::button()
            .on(ev::click, move |_| set_count.update(|c| *c += 10))
            .style("position: absolute")
            .style(("left", move || format!("{}px", count.get() + 100)))
            .style(("background-color", move || {
                format!("rgb({}, {}, 100)", count.get(), 100)
            }))
            .style(("max-width", "400px"))
            // Set a CSS variable for stylesheet use
            .style(("--columns", move || count.get().to_string()))
            .child("Click to Move"),
        html::br(),
        html::progress()
            .attr("max", "50")
            .attr("value", move || count.get()),
        html::br(),
        html::progress()
            .attr("max", "50")
            .attr("value", double_count),
        html::br(),
        html::p().child("Double Count: ").child(double_count),
    )
}
