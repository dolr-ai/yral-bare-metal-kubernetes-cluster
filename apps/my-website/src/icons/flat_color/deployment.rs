use leptos::svg;
use leptos::prelude::*;

/// Flat-color deployment (server rack) icon.
pub fn deployment_icon(class_name: &str) -> impl IntoView {
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("aria-hidden", "true")
        .attr("focusable", "false")
        .attr("class", class_name)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .attr("viewBox", "0 0 48 48")
        .child(
            svg::path()
                .attr("fill", "#B0BEC5")
                .attr("d", "M37 42H5V32h32c2.8 0 5 2.2 5 5s-2.2 5-5 5z"),
        )
        .child(
            svg::path()
                .attr("fill", "#37474F")
                .attr(
                    "d",
                    "M10 34c-1.7 0-3 1.3-3 3s1.3 3 3 3s3-1.3 3-3s-1.3-3-3-3zm0 4c-.6 0-1-.4-1-1s.4-1 1-1s1 .4 1 1s-.4 1-1 1z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#37474F")
                .attr(
                    "d",
                    "M19 34c-1.7 0-3 1.3-3 3s1.3 3 3 3s3-1.3 3-3s-1.3-3-3-3zm0 4c-.6 0-1-.4-1-1s.4-1 1-1s1 .4 1 1s-.4 1-1 1z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#37474F")
                .attr(
                    "d",
                    "M37 34c-1.7 0-3 1.3-3 3s1.3 3 3 3s3-1.3 3-3s-1.3-3-3-3zm0 4c-.6 0-1-.4-1-1s.4-1 1-1s1 .4 1 1s-.4 1-1 1z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#37474F")
                .attr(
                    "d",
                    "M28 34c-1.7 0-3 1.3-3 3s1.3 3 3 3s3-1.3 3-3s-1.3-3-3-3zm0 4c-.6 0-1-.4-1-1s.4-1 1-1s1 .4 1 1s-.4 1-1 1z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#FF9800")
                .attr(
                    "d",
                    "M35 31H11c-1.1 0-2-.9-2-2V7c0-1.1.9-2 2-2h24c1.1 0 2 .9 2 2v22c0 1.1-.9 2-2 2z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#8A5100")
                .attr(
                    "d",
                    "M26.5 13h-7c-.8 0-1.5-.7-1.5-1.5s.7-1.5 1.5-1.5h7c.8 0 1.5.7 1.5 1.5s-.7 1.5-1.5 1.5z",
                ),
        )
        .child(
            svg::path()
                .attr("fill", "#607D8B")
                .attr(
                    "d",
                    "M37 31H5v2h32c2.2 0 4 1.8 4 4s-1.8 4-4 4H5v2h32c3.3 0 6-2.7 6-6s-2.7-6-6-6z",
                ),
        )
}