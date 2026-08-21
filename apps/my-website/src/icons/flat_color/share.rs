use leptos::svg;
use leptos::prelude::*;

/// Flat-color share icon (three connected nodes).
pub fn share_icon(class_name: &str) -> impl IntoView {
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("aria-hidden", "true")
        .attr("focusable", "false")
        .attr("viewBox", "0 0 48 48")
        .attr("class", class_name)
        .child(
            svg::path()
                .attr("fill", "#1976D2")
                .attr(
                    "d",
                    "M38.1 31.2L19.4 24l18.7-7.2c1.5-.6 2.3-2.3 1.7-3.9c-.6-1.5-2.3-2.3-3.9-1.7l-26 10C8.8 21.6 8 22.8 8 24s.8 2.4 1.9 2.8l26 10c.4.1.7.2 1.1.2c1.2 0 2.3-.7 2.8-1.9c.6-1.6-.2-3.3-1.7-3.9z",
                ),
        )
        .child(
            svg::g()
                .attr("fill", "#1E88E5")
                .child(svg::circle().attr("cx", "11").attr("cy", "24").attr("r", "7"))
                .child(svg::circle().attr("cx", "37").attr("cy", "14").attr("r", "7"))
                .child(svg::circle().attr("cx", "37").attr("cy", "34").attr("r", "7")),
        )
}