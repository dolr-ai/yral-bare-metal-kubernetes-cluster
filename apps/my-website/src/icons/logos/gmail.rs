use leptos::svg;
use leptos::prelude::*;

/// Gmail (Google Mail) logo icon with the multi-colored M envelope.
pub fn gmail_icon(class_name: &str) -> impl IntoView {
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("aria-hidden", "true")
        .attr("focusable", "false")
        .attr("class", class_name)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .attr("viewBox", "0 0 256 193")
        .child(
            svg::path()
                .attr("fill", "#4285F4")
                .attr(
                    "d",
                    "M58.182 192.05V93.14L27.507 65.077L0 49.504v125.091c0 9.658 7.825 17.455 17.455 17.455h40.727z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#34A853")
                .attr(
                    "d",
                    "M197.818 192.05h40.727c9.659 0 17.455-7.826 17.455-17.455V49.505l-31.156 17.837l-27.026 25.798v98.91z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#EA4335")
                .attr(
                    "d",
                    "M58.182 93.14l-4.174-38.647l4.174-36.989L128 69.868l69.818-52.364l4.67 34.992l-4.67 40.644L128 145.504z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#FBBC04")
                .attr(
                    "d",
                    "M197.818 17.504V93.14L256 49.504V26.231c0-21.585-24.64-33.89-41.89-20.945l-16.292 12.218z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#C5221F")
                .attr(
                    "d",
                    "M0 49.504l26.759 20.07L58.182 93.14V17.504L41.89 5.286C24.61-7.66 0 4.646 0 26.23v23.273z",
                ),
        )
}