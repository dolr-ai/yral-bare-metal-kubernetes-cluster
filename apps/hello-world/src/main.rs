use leptos::html::*;
use leptos::prelude::*;
use leptos::*;

fn main() {
    leptos::mount::mount_to_body(|| p().child("Hello, world!"));
}
