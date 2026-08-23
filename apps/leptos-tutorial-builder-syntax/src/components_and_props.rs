use leptos::{html::*, prelude::*, *};

pub fn components_and_props() -> impl IntoView {
    let (count, set_count) = signal(0);

    (
        progress_bar(count),
        button()
            .child("Click me")
            .on(ev::click, move |_| set_count.update(|c| *c += 1)),
    )
}

fn progress_bar(progress: ReadSignal<i32>) -> impl IntoView {
    html::progress().attr("max", 50).attr("value", progress)
}
