use super::overlay::{ShadowOverlay, ShadowOverlayProps};
use crate::buttons::highlighted_button;
use leptos::{children::ToChildren, html, prelude::*};

pub fn popup(show: RwSignal<bool>, children: ChildrenFn) -> impl IntoView {
    let children_store = StoredValue::new(children);
    ShadowOverlay(
        ShadowOverlayProps::builder()
            .show(show)
            .children(ToChildren::to_children(move || {
                html::div()
                    .attr("style", "min-height: 500px; max-width:40rem;")
                    .attr("class", "flex relative flex-col gap-5 justify-between items-center py-4 mx-auto max-h-full rounded-md cursor-auto px-[20px] bg-neutral-900")
                    .child(
                        html::div()
                            .attr("class", "flex-1 pb-4 w-full")
                            .child(move || children_store.get_value()()),
                    )
                    .child(
                        html::div()
                            .attr("class", "flex justify-center items-center px-8 w-full")
                            .child(highlighted_button(
                                "Okay",
                                move || show.set(false),
                                "w-full".to_string(),
                                true,
                                false,
                            )),
                    )
            }))
            .build(),
    )
}
