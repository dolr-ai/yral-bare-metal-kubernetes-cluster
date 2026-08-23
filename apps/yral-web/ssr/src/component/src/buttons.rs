use leptos::{ev, html, prelude::*};

pub fn highlighted_button(
    children: impl IntoView,
    on_click: impl Fn() + 'static,
    classes: String,
    alt_style: bool,
    disabled: bool,
) -> impl IntoView {
    let on_click = move |_| on_click();
    let button_style = if alt_style {
        "background: linear-gradient(73deg, #FFF 0%, #FFF 1000%)"
    } else {
        "background: linear-gradient(190.27deg, #FF6DC4 8%, #F7007C 38.79%, #690039 78.48%);"
    };
    let inner_class = move || {
        if alt_style {
            "bg-linear-to-r from-[#FF78C1] via-[#E2017B] to-[#5F0938] inline-block text-transparent bg-clip-text"
        } else {
            "text-white"
        }
    };
    html::button()
        .on(ev::click, on_click)
        .attr("disabled", disabled)
        .attr(
            "class",
            format!(
                "w-full px-5 py-3 rounded-lg flex items-center transition-all justify-center gap-8 font-kumbh font-bold hover:opacity-90 {}",
                classes,
            ),
        )
        .attr("style", button_style)
        .child(html::div().attr("class", inner_class).child(children))
}
