use leptos::html;
use leptos::prelude::*;

pub fn not_found_page() -> impl IntoView {
    html::main()
        .attr("class", "max-w-screen-md mx-auto p-2")
        .child(
            html::img()
                .attr("src", "/assets/images/routes/__error/pupper.jpg")
                .attr("alt", "Sorry looking pupper")
                .attr("class", "w-full"),
        )
        .child(
            html::p()
                .attr("class", "text-xl")
                .child("Hey there, time traveller! You seem to have visited this page in the past when it doesn't exist yet!"),
        )
        .child(
            html::p()
                .child("Why not visit again in the future when this is available? In the meantime, why not ")
                .child(
                    html::a()
                        .attr("href", "/blog")
                        .attr("class", "text-fuchsia-500 font-semibold")
                        .child("check out"),
                )
                .child(" some of our recent posts on full stack Javascript"),
        )
}