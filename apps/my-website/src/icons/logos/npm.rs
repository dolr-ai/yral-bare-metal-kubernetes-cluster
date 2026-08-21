use leptos::svg;
use leptos::prelude::*;

/// NPM logo icon (red square with white "n" box).
pub fn npm_icon(class_name: &str) -> impl IntoView {
    svg::svg()
        .attr("aria-hidden", "true")
        .attr("focusable", "false")
        .attr("class", class_name)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .attr("viewBox", "0 0 256 256")
        .child(svg::path().attr("fill", "#C12127").attr("d", "M0 256V0h256v256z"))
        .child(
            svg::path()
                .attr("fill", "#FFF")
                .attr("d", "M48 48h160v160h-32V80h-48v128H48z"),
        )
}