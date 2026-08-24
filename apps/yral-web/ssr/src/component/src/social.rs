use super::ic_symbol::IcSymbol;
use leptos::{html, prelude::*};
use leptos_icons::{Icon, IconProps};

fn follow_item(href: String, icon: &'static icondata_core::IconData) -> impl IntoView {
    html::a()
        .attr("href", href)
        .attr("target", "_blank")
        .attr("class", "grid place-items-center w-12 h-12 text-2xl rounded-full border aspect-square border-primary-600")
        .child(Icon(IconProps::builder().icon(icon).build()))
}

pub fn domain_specific_href(base: &str) -> &'static str {
    match base {
        "TELEGRAM" => consts::social::TELEGRAM_YRAL,
        "TWITTER" => consts::social::TWITTER_YRAL,
        "DISCORD" => consts::social::DISCORD,
        "IC_WEBSITE" => consts::social::IC_WEBSITE,
        _ => panic!("Unknown base name"),
    }
}

pub fn telegram() -> impl IntoView {
    let href = domain_specific_href("TELEGRAM");
    follow_item(href.to_string(), icondata::TbBrandTelegramOutline)
}

pub fn discord() -> impl IntoView {
    follow_item(consts::social::DISCORD.to_string(), icondata::BiDiscordAlt)
}

pub fn twitter() -> impl IntoView {
    let href = domain_specific_href("TWITTER");
    follow_item(href.to_string(), icondata::BiTwitter)
}

pub fn ic_website() -> impl IntoView {
    follow_item(consts::social::IC_WEBSITE.to_string(), IcSymbol)
}
