use leptos::html;
use leptos::prelude::*;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app);
}

fn app() -> impl IntoView {
    let (count, set_count) = signal(0);

    html::button()
        .on(ev::click, move |_| set_count.update(|c| *c += 10))
        .style("position: absolute")
        .style(("left", move || format!("{}px", count.get() + 100)))
        .style(("background-color", move || {
            format!("rgb({}, {}, 100)", count.get(), 100)
        }))
        .style(("max-width", "400px"))
        // Set a CSS variable for stylesheet use
        .style(("--columns", move || count.get().to_string()))
        .child("Click to Move")
}
