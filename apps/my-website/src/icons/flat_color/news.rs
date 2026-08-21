use leptos::svg;
use leptos::prelude::*;

/// Flat-color news (newspaper) icon.
pub fn news_icon(class_name: &str) -> impl IntoView {
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("aria-hidden", "true")
        .attr("focusable", "false")
        .attr("class", class_name)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .attr("viewBox", "0 0 48 48")
        .child(
            svg::path()
                .attr("fill", "#FF5722")
                .attr("d", "M32 15v28H10c-2.2 0-4-1.8-4-4V15h26z"),
        )
        .child(
            svg::path()
                .attr("fill", "#FFCCBC")
                .attr("d", "M14 5v34c0 2.2-1.8 4-4 4h29c2.2 0 4-1.8 4-4V5H14z"),
        )
        .child(
            svg::g()
                .attr("fill", "#FF5722")
                .child(svg::path().attr("d", "M20 10h18v4H20z"))
                .child(svg::path().attr("d", "M20 17h8v2h-8z"))
                .child(svg::path().attr("d", "M30 17h8v2h-8z"))
                .child(svg::path().attr("d", "M20 21h8v2h-8z"))
                .child(svg::path().attr("d", "M30 21h8v2h-8z"))
                .child(svg::path().attr("d", "M20 25h8v2h-8z"))
                .child(svg::path().attr("d", "M30 25h8v2h-8z"))
                .child(svg::path().attr("d", "M20 29h8v2h-8z"))
                .child(svg::path().attr("d", "M30 29h8v2h-8z"))
                .child(svg::path().attr("d", "M20 33h8v2h-8z"))
                .child(svg::path().attr("d", "M30 33h8v2h-8z"))
                .child(svg::path().attr("d", "M20 37h8v2h-8z"))
                .child(svg::path().attr("d", "M30 37h8v2h-8z")),
        )
}