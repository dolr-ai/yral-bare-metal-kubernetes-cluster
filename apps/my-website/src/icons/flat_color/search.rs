use leptos::svg;
use leptos::prelude::*;

/// Flat-color search (magnifying glass) icon.
pub fn search_icon(class_name: &str) -> impl IntoView {
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("aria-hidden", "true")
        .attr("focusable", "false")
        .attr("class", class_name)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .attr("viewBox", "0 0 48 48")
        .child(
            svg::g()
                .attr("fill", "#616161")
                .child(
                    svg::path()
                        .attr("d", "M29.175 31.99l2.828-2.827l12.019 12.019l-2.828 2.827z"),
                )
                .child(svg::circle().attr("cx", "20").attr("cy", "20").attr("r", "16")),
        )
        .child(
            svg::path()
                .attr("fill", "#37474F")
                .attr("d", "M32.45 35.34l2.827-2.828l8.696 8.696l-2.828 2.828z"),
        )
        .child(svg::circle().attr("fill", "#64B5F6").attr("cx", "20").attr("cy", "20").attr("r", "13"))
        .child(
            svg::path()
                .attr("fill", "#BBDEFB")
                .attr(
                    "d",
                    "M26.9 14.2c-1.7-2-4.2-3.2-6.9-3.2s-5.2 1.2-6.9 3.2c-.4.4-.3 1.1.1 1.4c.4.4 1.1.3 1.4-.1C16 13.9 17.9 13 20 13s4 .9 5.4 2.5c.2.2.5.4.8.4c.2 0 .5-.1.6-.2c.4-.4.4-1.1.1-1.5z",
                ),
        )
}