use leptos::html;
use leptos::prelude::*;
// use leptos::*;

mod components_and_props;
mod dynamic_attributes;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app);
}

fn app() -> impl IntoView {
    (
        dynamic_attributes::dynamic_attributes(),
        separator(),
        components_and_props::components_and_props(),
    )
}

fn separator() -> impl IntoView {
    html::hr()
        .style(("margin", "2rem 0"))
        .style(("border", "0.5rem solid black"))
}
