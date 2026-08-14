//! Chat conversation page — `/chat/:influencer_identifier`.
//!
//! Creates a conversation with an AI influencer, loads message history,
//! and provides a chat UI with SSE streaming for AI responses.
//!
//! API (all require Bearer JWT auth):
//! - POST   /api/v1/chat/conversations                     → create conversation
//! - GET    /api/v1/chat/conversations/{identifier}/messages → list messages
//! - POST   /api/v1/chat/conversations/{identifier}/messages/stream → SSE stream
//!
//! SSE events: token (incremental text), done (final message), error (failure)

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use leptos_router::hooks::{use_navigate, use_params};
use leptos_router::params::Params;
use serde::{Deserialize, Serialize};
use utils::send_wrap;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// Rishi's agent backend base URL.
const AGENT_BACKEND_URL: &str = "https://agent.rishi.yral.com";

// ─── API types ─────────────────────────────────────────────────────────────

/// A chat message (user or assistant) — the public type used by the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub identifier: String,
    pub role: String,
    pub content: String,
    pub message_type: String,
    pub created_at: String,
}

/// A conversation with an influencer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Conversation {
    pub identifier: String,
    pub influencer_identifier: String,
    pub created_at: String,
    pub message_count: i32,
}

/// Response from creating a conversation.
#[derive(Debug, Deserialize)]
struct ConversationResponse {
    identifier: String,
    #[serde(rename = "influencer_id")]
    #[allow(dead_code)]
    influencer_identifier: Option<String>,
    #[allow(dead_code)]
    created_at: String,
    #[serde(rename = "message_count")]
    message_count: i32,
}

/// Response from listing messages.
#[derive(Debug, Deserialize)]
struct ConversationMessagesResponse {
    #[serde(rename = "conversation_id")]
    #[allow(dead_code)]
    conversation_identifier: String,
    messages: Vec<ChatMessageApi>,
    #[allow(dead_code)]
    total: i32,
    #[allow(dead_code)]
    limit: i32,
    #[allow(dead_code)]
    offset: i32,
}

/// Individual message from the API (raw, content is optional).
#[derive(Debug, Deserialize)]
struct ChatMessageApi {
    identifier: String,
    role: String,
    content: Option<String>,
    #[serde(rename = "message_type")]
    message_type: String,
    #[serde(rename = "created_at")]
    created_at: String,
}

/// Request to create a conversation.
#[derive(Debug, Serialize)]
struct CreateConversationRequest {
    #[serde(rename = "influencer_id")]
    influencer_identifier: String,
}

/// Request to send a message.
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    content: String,
    #[serde(rename = "message_type")]
    message_type: String,
}

// ─── Server functions ──────────────────────────────────────────────────────

/// Read the user's JWT from the ID_TOKEN cookie (server-side only).
#[cfg(feature = "ssr")]
async fn get_authentication_token() -> Result<String, ServerFnError> {
    use axum_extra::extract::{cookie::Key, SignedCookieJar};

    let cookie_key: Key = use_context().or_else(|| {
        let cookie_key_string =
            std::env::var("COOKIE_KEY").expect("`COOKIE_KEY` is required!");
        let raw_key = hex::decode(cookie_key_string).expect("Invalid `COOKIE_KEY`");
        Some(Key::from(&raw_key))
    }).unwrap();

    let signed_cookie_jar: SignedCookieJar =
        leptos_axum::extract_with_state(&cookie_key).await?;
    let cookie = signed_cookie_jar
        .get(consts::auth::ID_TOKEN_COOKIE)
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    Ok(cookie.value().to_string())
}

/// Server function: create a conversation with an influencer.
#[server(endpoint = "create_conversation", input = Json, output = Json)]
pub async fn create_conversation(
    influencer_identifier: String,
) -> Result<Conversation, ServerFnError> {
    let authentication_token = get_authentication_token().await?;

    let influencer_identifier_for_response = influencer_identifier.clone();

    let http_client = reqwest::Client::new();
    let response = http_client
        .post(format!("{}/api/v1/chat/conversations", AGENT_BACKEND_URL))
        .header("Authorization", format!("Bearer {authentication_token}"))
        .json(&CreateConversationRequest {
            influencer_identifier,
        })
        .send()
        .await
        .map_err(|error| {
            ServerFnError::new(format!("Create conversation failed: {error}"))
        })?;

    let conversation_response: ConversationResponse = response
        .json()
        .await
        .map_err(|error| {
            ServerFnError::new(format!("Parse conversation failed: {error}"))
        })?;

    Ok(Conversation {
        identifier: conversation_response.identifier,
        influencer_identifier: influencer_identifier_for_response,
        created_at: conversation_response.created_at,
        message_count: conversation_response.message_count,
    })
}

/// Server function: load message history for a conversation.
#[server(endpoint = "list_conversation_messages", input = Json, output = Json)]
pub async fn list_conversation_messages(
    conversation_identifier: String,
) -> Result<Vec<ChatMessage>, ServerFnError> {
    let authentication_token = get_authentication_token().await?;

    let http_client = reqwest::Client::new();
    let response = http_client
        .get(format!(
            "{}/api/v1/chat/conversations/{}/messages?limit=50&offset=0&order=asc",
            AGENT_BACKEND_URL, conversation_identifier
        ))
        .header("Authorization", format!("Bearer {authentication_token}"))
        .send()
        .await
        .map_err(|error| {
            ServerFnError::new(format!("List messages failed: {error}"))
        })?;

    let messages_response: ConversationMessagesResponse = response
        .json()
        .await
        .map_err(|error| {
            ServerFnError::new(format!("Parse messages failed: {error}"))
        })?;

    Ok(messages_response
        .messages
        .into_iter()
        .map(|message| ChatMessage {
            identifier: message.identifier,
            role: message.role,
            content: message.content.unwrap_or_default(),
            message_type: message.message_type,
            created_at: message.created_at,
        })
        .collect())
}

/// Server function: get the authentication token for client-side SSE streaming.
/// The client needs the JWT to make the streaming POST request directly to
/// the agent backend (SSE over POST can't be proxied through server functions).
#[server(endpoint = "get_chat_token", input = Json, output = Json)]
pub async fn get_chat_token() -> Result<String, ServerFnError> {
    get_authentication_token().await
}

// ─── SSE streaming via Fetch API + ReadableStream ──────────────────────────

/// SSE event parsed from the stream.
enum ServerSentEvent {
    Token(String),
    Done,
    Error(String),
}

/// Parse a single SSE event from its type and data fields.
fn parse_server_sent_event(event_type: &str, data: &str) -> ServerSentEvent {
    match event_type {
        "token" => {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(text) = value["text"].as_str() {
                    return ServerSentEvent::Token(text.to_string());
                }
            }
            ServerSentEvent::Token(data.to_string())
        }
        "done" => ServerSentEvent::Done,
        "error" => {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(message) = value["message"].as_str() {
                    return ServerSentEvent::Error(message.to_string());
                }
            }
            ServerSentEvent::Error(data.to_string())
        }
        _ => ServerSentEvent::Token(data.to_string()),
    }
}

/// Stream a chat message via SSE using the Fetch API's ReadableStream.
/// Calls `on_token` for each incremental token. Returns the full assistant
/// text on success, or an error message on failure.
#[cfg(feature = "hydrate")]
async fn stream_message_via_sse(
    conversation_identifier: &str,
    content: &str,
    authentication_token: &str,
    on_token: impl Fn(&str) + 'static,
) -> Result<String, String> {
    let url = format!(
        "{}/api/v1/chat/conversations/{}/messages/stream",
        AGENT_BACKEND_URL, conversation_identifier
    );

    let request_body = serde_json::to_string(&SendMessageRequest {
        content: content.to_string(),
        message_type: "text".to_string(),
    })
    .unwrap_or_default();

    let headers =
        web_sys::Headers::new().map_err(|error| format!("Headers error: {error:?}"))?;
    headers
        .set("Authorization", &format!("Bearer {authentication_token}"))
        .map_err(|error| format!("Set auth header: {error:?}"))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|error| format!("Set content-type: {error:?}"))?;
    headers
        .set("Accept", "text/event-stream")
        .map_err(|error| format!("Set accept: {error:?}"))?;

    let request_options = web_sys::RequestInit::new();
    request_options.set_method("POST");
    request_options.set_body(&wasm_bindgen::JsValue::from_str(&request_body));
    request_options.set_headers(&headers);

    let request = web_sys::Request::new_with_str_and_init(&url, &request_options)
        .map_err(|error| format!("Request creation: {error:?}"))?;

    let response = web_sys::window()
        .unwrap()
        .fetch_with_request(&request)
        .await
        .map_err(|error| format!("Fetch failed: {error:?}"))?;

    let http_response: web_sys::Response =
        response.dyn_into().map_err(|_| "Response cast failed")?;
    if !http_response.ok() {
        return Err(format!("HTTP {}", http_response.status()));
    }

    let readable_stream = http_response.body().ok_or("No response body")?;
    let reader = readable_stream.get_reader();

    let mut full_response_text = String::new();
    let mut buffer = String::new();

    loop {
        let chunk_result = reader
            .read()
            .await
            .map_err(|error| format!("Read error: {error:?}"))?;

        if chunk_result.done() {
            break;
        }

        let chunk = chunk_result.value();
        let chunk_text = String::from(js_sys::JsString::from(chunk));
        buffer.push_str(&chunk_text);

        // Process complete SSE events (separated by double newline)
        while let Some(separator_index) = buffer.find("\n\n") {
            let event_block = buffer[..separator_index].to_string();
            buffer = buffer[separator_index + 2..].to_string();

            let mut event_type = "message".to_string();
            let mut data_parts: Vec<&str> = Vec::new();

            for line in event_block.lines() {
                if let Some(rest) = line.strip_prefix("event:") {
                    event_type = rest.trim().to_string();
                } else if let Some(rest) = line.strip_prefix("data:") {
                    data_parts.push(rest.trim());
                }
            }

            let data = data_parts.join("\n");
            match parse_server_sent_event(&event_type, &data) {
                ServerSentEvent::Token(text) => {
                    full_response_text.push_str(&text);
                    on_token(&text);
                }
                ServerSentEvent::Done => {
                    return Ok(full_response_text);
                }
                ServerSentEvent::Error(message) => {
                    return Err(message);
                }
            }
        }
    }

    Ok(full_response_text)
}

// ─── Chat component ────────────────────────────────────────────────────────

/// Route params for the chat page.
#[derive(Params, PartialEq, Clone)]
struct ChatRouteParams {
    influencer_identifier: String,
}

/// The Chat component — renders a full chat conversation at
/// `/chat/:influencer_identifier`.
#[component]
pub fn Chat() -> impl IntoView {
    let route_params = use_params::<ChatRouteParams>();
    let navigate = use_navigate();

    // Create conversation on load
    let conversation_resource = Resource::new(
        move || {
            route_params
                .get()
                .map(|params| params.influencer_identifier)
                .unwrap_or_default()
        },
        move |influencer_identifier| {
            send_wrap(async move {
                if influencer_identifier.is_empty() {
                    return None;
                }
                create_conversation(influencer_identifier).await.ok()
            })
        },
    );

    // State signals
    let messages: RwSignal<Vec<ChatMessage>> = RwSignal::new(Vec::new());
    let input_text: RwSignal<String> = RwSignal::new(String::new());
    let is_sending: RwSignal<bool> = RwSignal::new(false);
    let streaming_text: RwSignal<String> = RwSignal::new(String::new());
    let error_message: RwSignal<Option<String>> = RwSignal::new(None);

    // Load messages when conversation is created
    Effect::new({
        let messages = messages.clone();
        move |_| {
            if let Some(conversation) = conversation_resource.get().flatten().as_ref() {
                let conversation_identifier = conversation.identifier.clone();
                let messages = messages.clone();
                spawn_local(async move {
                    if let Ok(loaded_messages) =
                        list_conversation_messages(conversation_identifier).await
                    {
                        messages.set(loaded_messages);
                    }
                });
            }
        }
    });

    // Send message handler — uses SSE streaming via Fetch API
    let send_message = move || {
        let text = input_text.get().trim().to_string();
        if text.is_empty() || is_sending.get() {
            return;
        }

        is_sending.set(true);
        streaming_text.set(String::new());
        error_message.set(None);
        input_text.set(String::new());

        let conversation = conversation_resource.get().flatten();
        let Some(conversation) = conversation.as_ref() else {
            is_sending.set(false);
            return;
        };
        let conversation_identifier = conversation.identifier.clone();

        // Add user message immediately for instant feedback
        messages.update(|message_list| {
            message_list.push(ChatMessage {
                identifier: format!("local-user-{}", message_list.len()),
                role: "user".to_string(),
                content: text.clone(),
                message_type: "text".to_string(),
                created_at: String::new(),
            });
        });

        let messages_signal = messages.clone();
        let streaming_text_signal = streaming_text.clone();
        let is_sending_signal = is_sending.clone();
        let error_message_signal = error_message.clone();

        spawn_local(async move {
            // Get the JWT for client-side SSE streaming
            let authentication_token = match get_chat_token().await {
                Ok(token) => token,
                Err(error) => {
                    error_message_signal
                        .set(Some(format!("Authentication error: {error}")));
                    is_sending_signal.set(false);
                    return;
                }
            };

            // Stream the message via SSE — the on_token callback updates
            // streaming_text in real-time so the user sees tokens appear.
            // This only runs on the client (hydrate) since it uses the
            // Fetch API + ReadableStream for SSE streaming.
            #[cfg(feature = "hydrate")]
            {
                let on_token = move |chunk: &str| {
                    streaming_text_signal.update(|stream| stream.push_str(chunk));
                };

                match stream_message_via_sse(
                    &conversation_identifier,
                    &text,
                    &authentication_token,
                    on_token,
                )
                .await
                {
                    Ok(full_response) => {
                        streaming_text_signal.set(String::new());
                        messages_signal.update(|message_list| {
                            message_list.push(ChatMessage {
                                identifier: format!("assistant-{}", message_list.len()),
                                role: "assistant".to_string(),
                                content: full_response,
                                message_type: "text".to_string(),
                            created_at: String::new(),
                        });
                    });
                }
                Err(error) => {
                    streaming_text_signal.set(String::new());
                    error_message_signal.set(Some(format!("Stream error: {error}")));
                }
            }
            }
            is_sending_signal.set(false);
        });
    };

    html::div()
        .class("flex flex-col h-screen bg-black text-white")
        .child(
            // Header with back button
            html::div()
                .class("flex items-center gap-3 p-4 border-b border-neutral-800")
                .child(
                    html::button()
                        .class("text-2xl text-gray-400 hover:text-white")
                        .on(ev::click, move |_| {
                            let _ = navigate("/", Default::default());
                        })
                        .child("←"),
                )
                .child(
                    html::span()
                        .class("text-lg font-semibold")
                        .child("Chat"),
                ),
        )
        .child(
            // Messages list + streaming + loading
            html::div()
                .class("flex-1 overflow-y-auto p-4 space-y-4")
                .child(move || {
                    let current_messages = messages.get();
                    let mut message_views: Vec<AnyView> = current_messages
                        .into_iter()
                        .map(|message| {
                            let is_user_message = message.role == "user";
                            let bubble_class = if is_user_message {
                                "ml-auto max-w-[80%] bg-primary-600 rounded-2xl rounded-br-sm px-4 py-2"
                            } else {
                                "mr-auto max-w-[80%] bg-neutral-800 rounded-2xl rounded-bl-sm px-4 py-2"
                            };

                            html::div()
                                .class("flex")
                                .child(
                                    html::div()
                                        .class(bubble_class)
                                        .child(
                                            html::p()
                                                .class("text-sm whitespace-pre-wrap")
                                                .child(message.content),
                                        ),
                                )
                                .into_any()
                        })
                        .collect();

                    // Show streaming text (AI response in progress)
                    let current_stream = streaming_text.get();
                    if !current_stream.is_empty() {
                        message_views.push(
                            html::div()
                                .class("flex")
                                .child(
                                    html::div()
                                        .class(
                                            "mr-auto max-w-[80%] bg-neutral-800 rounded-2xl rounded-bl-sm px-4 py-2",
                                        )
                                        .child(
                                            html::p()
                                                .class("text-sm whitespace-pre-wrap")
                                                .child(current_stream),
                                        ),
                                )
                                .into_any(),
                        );
                    }

                    // Show "typing..." indicator when sending but no stream yet
                    if is_sending.get() && streaming_text.get().is_empty() {
                        message_views.push(
                            html::div()
                                .class("flex")
                                .child(
                                    html::div()
                                        .class(
                                            "mr-auto bg-neutral-800 rounded-2xl rounded-bl-sm px-4 py-2",
                                        )
                                        .child(
                                            html::span()
                                                .class("text-sm text-gray-400")
                                                .child("typing..."),
                                        ),
                                )
                                .into_any(),
                        );
                    }

                    // Show error message
                    if let Some(error) = error_message.get() {
                        message_views.push(
                            html::div()
                                .class("text-red-400 text-sm text-center")
                                .child(error)
                                .into_any(),
                        );
                    }

                    message_views
                })
                .child(
                    // Loading state while conversation is being created
                    Suspend::new(async move {
                        let conversation = conversation_resource.await;
                        if conversation.is_none() {
                            html::div()
                                .class("text-gray-400 text-center py-8")
                                .child("Creating conversation...")
                                .into_any()
                        } else {
                            ().into_any()
                        }
                    }),
                ),
        )
        .child(
            // Input area
            html::div()
                .class("flex items-center gap-2 p-4 border-t border-neutral-800")
                .child(
                    html::input()
                        .attr("type", "text")
                        .attr("placeholder", "Type a message...")
                        .class(
                            "flex-1 bg-neutral-800 text-white rounded-full px-4 py-2 focus:outline-none",
                        )
                        .prop("value", move || input_text.get())
                        .on(ev::input, move |event: ev::Event| {
                            let target = event.target().unwrap();
                            #[cfg(feature = "hydrate")]
                            {
                                let input_element: web_sys::HtmlInputElement =
                                    target.dyn_into().unwrap();
                                input_text.set(input_element.value());
                            }
                        })
                        .on(ev::keydown, move |event: ev::KeyboardEvent| {
                            if event.key() == "Enter" {
                                send_message();
                            }
                        }),
                )
                .child(
                    html::button()
                        .class(
                            "bg-primary-600 text-white rounded-full px-4 py-2 font-semibold disabled:opacity-50",
                        )
                        .prop("disabled", move || is_sending.get())
                        .on(ev::click, move |_| send_message())
                        .child("Send"),
                ),
        )
}