use leptos::svg;
use leptos::prelude::*;

/// Flat-color home icon (house with red roof and blue window).
pub fn home_icon(class_name: &str) -> impl IntoView {
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("aria-hidden", "true")
        .attr("focusable", "false")
        .attr("class", class_name)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .attr("viewBox", "0 0 48 48")
        .child(svg::path().attr("fill", "#E8EAF6").attr("d", "M42 39H6V23L24 6l18 17z"))
        .child(
            svg::g()
                .attr("fill", "#C5CAE9")
                .child(svg::path().attr("d", "M39 21l-5-5V9h5z"))
                .child(svg::path().attr("d", "M6 39h36v5H6z")),
        )
        .child(
            svg::path()
                .attr("fill", "#B71C1C")
                .attr("d", "M24 4.3L4 22.9l2 2.2L24 8.4l18 16.7l2-2.2z"),
        )
        .child(svg::path().attr("fill", "#D84315").attr("d", "M18 28h12v16H18z"))
        .child(svg::path().attr("fill", "#01579B").attr("d", "M21 17h6v6h-6z"))
        .child(
            svg::path()
                .attr("fill", "#FF8A65")
                .attr(
                    "d",
                    "M27.5 35.5c-.3 0-.5.2-.5.5v2c0 .3.2.5.5.5s.5-.2.5-.5v-2c0-.3-.2-.5-.5-.5z",
                ),
        )
}