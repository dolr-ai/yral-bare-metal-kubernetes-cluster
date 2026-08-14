//! AI influencer feed page — the default landing page at `/`.
//!
//! Shows a grid of AI influencer profile cards, matching the mobile app's
//! Chat Discover / ChatWall screen. Data comes from Rishi's agent backend
//! at `agent.rishi.yral.com/api/v2/discovery/influencer-feed`.
//!
//! Each card shows: avatar image, display name, message count, category.
//! Clicking a card will eventually open a chat conversation (future).

use leptos::ev;
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};
use utils::send_wrap;

/// Rishi's agent backend base URL (matches CHAT_BASE_URL in yral-mobile).
const AGENT_BASE_URL: &str = "https://agent.rishi.yral.com";
/// Discovery influencer feed path (v2, matches DISCOVERY_FEED_PATH in mobile).
const DISCOVERY_FEED_PATH: &str = "api/v2/discovery/influencer-feed";

/// An influencer profile card for the grid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InfluencerCard {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub avatar_url: String,
    pub description: String,
    pub category: String,
    pub created_at: String,
    pub message_count: Option<i64>,
    pub conversation_count: Option<i64>,
}

/// Response from the discovery influencer feed API.
#[derive(Debug, Deserialize)]
struct InfluencerFeedResponse {
    influencers: Vec<InfluencerFeedItem>,
    #[serde(rename = "total_count")]
    #[allow(dead_code)]
    total_count: i32,
    #[allow(dead_code)]
    offset: i32,
    #[allow(dead_code)]
    limit: i32,
    #[serde(rename = "has_more")]
    has_more: bool,
    #[serde(rename = "feed_generated_at")]
    #[allow(dead_code)]
    feed_generated_at: Option<String>,
}

/// Individual influencer from the feed API.
#[derive(Debug, Deserialize)]
struct InfluencerFeedItem {
    id: String,
    name: String,
    #[serde(rename = "display_name")]
    display_name: String,
    #[serde(rename = "avatar_url")]
    avatar_url: String,
    #[allow(dead_code)]
    description: String,
    category: String,
    #[serde(rename = "created_at")]
    created_at: String,
    #[allow(dead_code)]
    signals: Option<InfluencerFeedSignals>,
}

/// Engagement signals (optional, from the feed API).
#[derive(Debug, Deserialize)]
struct InfluencerFeedSignals {
    #[serde(rename = "conversation_count")]
    conversation_count: Option<i64>,
    #[serde(rename = "message_count")]
    message_count: Option<i64>,
}

/// Server function: fetch the AI influencer feed from Rishi's agent backend.
/// Returns a paginated list of influencer profile cards.
#[server(endpoint = "fetch_influencer_feed", input = Json, output = Json)]
pub async fn fetch_influencer_feed(
    limit: i32,
    offset: i32,
) -> Result<(Vec<InfluencerCard>, bool), ServerFnError> {
    let url = format!(
        "{}/{}?limit={}&offset={}",
        AGENT_BASE_URL, DISCOVERY_FEED_PATH, limit, offset
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Influencer feed request failed: {e}")))?;

    let feed: InfluencerFeedResponse = resp
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Influencer feed parse failed: {e}")))?;

    let cards = feed
        .influencers
        .into_iter()
        .map(|item| {
            let name = item.name;
            let display_name = if item.display_name.is_empty() {
                name.clone()
            } else {
                item.display_name
            };
            InfluencerCard {
                id: item.id,
                name,
                display_name,
                avatar_url: item.avatar_url,
                description: item.description,
                category: item.category,
                created_at: item.created_at,
                message_count: item.signals.as_ref().and_then(|s| s.message_count),
                conversation_count: item.signals.as_ref().and_then(|s| s.conversation_count),
            }
        })
        .collect();

    Ok((cards, feed.has_more))
}

/// Format a large number as an abbreviation (e.g., 1200 → "1.2K").
fn format_abbrev(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// The Home component — renders the AI influencer profile grid at `/`.
#[component]
pub fn Home() -> impl IntoView {
    use leptos::html;
    use leptos_router::hooks::use_navigate;

    let navigate = use_navigate();
    let feed_resource = Resource::new(
        || (),
        move |()| {
            send_wrap(async move {
                fetch_influencer_feed(20, 0)
                    .await
                    .unwrap_or((Vec::new(), false))
            })
        },
    );

    html::div()
        .class("bg-black min-h-screen text-white")
        .child(
            html::div()
                .class("max-w-2xl mx-auto px-4 py-6")
                .child(html::h1().class("text-2xl font-bold mb-4").child("Discover AI Influencers"))
                .child(
                    Suspend::new(async move {
                        let (cards, _has_more) = feed_resource.await;
                        if cards.is_empty() {
                            html::div()
                                .class("text-gray-400 py-8 text-center")
                                .child("No influencers available right now.")
                                .into_any()
                        } else {
                            let cards_view: Vec<AnyView> = cards
                                .into_iter()
                                .map(|influencer| {
                                    let display_name = if influencer.display_name.is_empty() {
                                        influencer.name.clone()
                                    } else {
                                        influencer.display_name.clone()
                                    };
                                    let msg_count = influencer
                                        .message_count
                                        .map(format_abbrev)
                                        .unwrap_or_default();
                                    let avatar_url = influencer.avatar_url.clone();
                                    let category = influencer.category.clone();
                                    let influencer_identifier = influencer.id.clone();
                                    let navigation = navigate.clone();

                                    let msg_span = if !msg_count.is_empty() {
                                        Some(
                                            html::span()
                                                .class(
                                                    "text-xs text-gray-400 flex items-center gap-1",
                                                )
                                                .child(msg_count),
                                        )
                                    } else {
                                        None
                                    };

                                    html::div()
                                        .class(
                                            "flex flex-col rounded-xl bg-neutral-800 overflow-hidden cursor-pointer hover:bg-neutral-700 transition-colors",
                                        )
                                        .on(ev::click, move |_| {
                                            let _ = navigation(&format!("/chat/{influencer_identifier}"), Default::default());
                                        })
                                        .child(
                                            html::div()
                                                .class("relative aspect-[3/4] w-full")
                                                .child(
                                                    html::img()
                                                        .attr("src", avatar_url)
                                                        .attr("alt", display_name.clone())
                                                        .class(
                                                            "absolute inset-0 w-full h-full object-cover p-1.5 rounded-xl",
                                                        ),
                                                ),
                                        )
                                        .child(
                                            html::div()
                                                .class("p-2.5 space-y-1")
                                                .child(
                                                    html::div()
                                                        .class("flex items-center justify-between")
                                                        .child(
                                                            html::span()
                                                                .class(
                                                                    "font-semibold text-sm text-yellow-400 truncate",
                                                                )
                                                                .child(display_name),
                                                        )
                                                        .child(msg_span),
                                                )
                                                .child(
                                                    html::span()
                                                        .class("text-xs text-gray-500")
                                                        .child(category),
                                                ),
                                        )
                                        .into_any()
                                })
                                .collect();

                            html::div()
                                .class("grid grid-cols-2 gap-4")
                                .child(cards_view)
                                .into_any()
                        }
                    }),
                ),
        )
}
