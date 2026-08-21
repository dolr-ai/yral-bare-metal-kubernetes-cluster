pub mod app;
pub mod components;
pub mod content;
pub mod icons;
pub mod islands;
pub mod page;
pub mod styles;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::app;

    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    // In islands mode, we hydrate individual islands rather than the whole app.
    // This keeps the WASM binary minimal — only #[island] components ship to the browser.
    leptos::mount::hydrate_islands();

    // Reference App to ensure wasm-bindgen generates bindings correctly
    // in workspace setups (see https://github.com/leptos-rs/leptos/issues/2083).
    let _ = app;
}