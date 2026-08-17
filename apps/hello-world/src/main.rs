use leptos::html::*;
use leptos::prelude::*;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app);
}

fn app() -> impl IntoView {
    let (count, set_count) = signal(0);

    html::button()
        .on(ev::click, move |_| set_count.update(|v| *v = *v + 1))
        .child("Click me!")
}
