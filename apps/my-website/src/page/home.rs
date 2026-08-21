use leptos::html;
use leptos::prelude::*;

use crate::components::seo::seo_meta_header;
use crate::components::sections::{
    intro_section, find_me_on_section, about_me_section,
    what_im_learning_section, what_im_working_on_section,
};

pub fn home_page() -> impl IntoView {
    (
        seo_meta_header(
            "Saikat's Website",
            "This is a website where I blog about full stack JavaScript including things like SvelteJS, TailwindCSS, NodeJS, Linux, PostgreSQL among a few and showcase projects I've worked on",
        ),
        html::main()
            .attr("class", "max-w-screen-md mx-auto p-2 mb-8")
            .child(intro_section())
            .child(find_me_on_section())
            .child(about_me_section())
            .child(what_im_learning_section())
            .child(what_im_working_on_section()),
    )
}