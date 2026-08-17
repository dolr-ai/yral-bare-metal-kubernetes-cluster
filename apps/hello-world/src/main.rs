use leptos::html::*;
use leptos::prelude::*;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| p().child("Hello, world!"));
}
