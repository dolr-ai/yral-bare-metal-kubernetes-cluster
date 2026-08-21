use leptos::svg;
use leptos::prelude::*;

/// Twemoji Indian flag icon (saffron-white-green tricolour with Ashoka Chakra).
pub fn flag_india_icon(class_name: &str) -> impl IntoView {
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("aria-hidden", "true")
        .attr("focusable", "false")
        .attr("class", class_name)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .attr("viewBox", "0 0 36 36")
        .child(
            svg::path()
                .attr("fill", "#138808")
                .attr("d", "M0 27a4 4 0 0 0 4 4h28a4 4 0 0 0 4-4v-4H0v4z"),
        )
        .child(svg::path().attr("fill", "#EEE").attr("d", "M0 13h36v10H0z"))
        .child(
            svg::path()
                .attr("fill", "#F93")
                .attr("d", "M36 13V9a4 4 0 0 0-4-4H4a4 4 0 0 0-4 4v4h36z"),
        )
        .child(svg::circle().attr("fill", "navy").attr("cx", "18").attr("cy", "18").attr("r", "4"))
        .child(svg::circle().attr("fill", "#EEE").attr("cx", "18").attr("cy", "18").attr("r", "3"))
        .child(
            svg::path()
                .attr("fill", "#6666B3")
                .attr(
                    "d",
                    "M18 15l.146 2.264l1.001-2.035l-.73 2.147l1.704-1.498l-1.497 1.705l2.147-.731l-2.035 1.002L21 18l-2.264.146l2.035 1.001l-2.147-.73l1.497 1.704l-1.704-1.497l.73 2.147l-1.001-2.035L18 21l-.146-2.264l-1.002 2.035l.731-2.147l-1.705 1.497l1.498-1.704l-2.147.73l2.035-1.001L15 18l2.264-.146l-2.035-1.002l2.147.731l-1.498-1.705l1.705 1.498l-.731-2.147l1.002 2.035z",
                ),
        )
        .child(svg::circle().attr("fill", "navy").attr("cx", "18").attr("cy", "18").attr("r", "1"))
}