use std::env;

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{Html, IntoResponse},
    routing::get,
};
use futures_util::StreamExt;
use leptos::{prelude::*, tachys::view::RenderHtml};
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use url::Url;

const DOCUMENT_HEAD: &str = r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Evento Globolo</title><style>body{font-family:system-ui;margin:0;background:#f7f7f8}.shell{max-width:960px;margin:auto;padding:4rem 1.5rem}.grid,.providers{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:1rem}article{background:white;padding:1.25rem;border-radius:14px}.button{display:inline-block;margin:1rem 0;padding:.75rem 1rem;background:#111;color:white;text-decoration:none;border-radius:.5rem}.provider-name{text-transform:capitalize}</style></head><body>"#;
const DOCUMENT_TAIL: &str = r#"<script>const el=document.getElementById('live');const ws=new WebSocket(`${location.protocol==='https:'?'wss':'ws'}://${location.host}/ws`);ws.onopen=()=>el.textContent='Connected';ws.onmessage=e=>el.textContent=e.data;ws.onclose=()=>el.textContent='Disconnected';</script></body></html>"#;

#[derive(Clone)]
struct AppState {
    api_url: Url,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize)]
struct ProviderView {
    capabilities: Capabilities,
    configured: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct Capabilities {
    provider: String,
    delivery_mode: String,
    oauth: bool,
    publish: bool,
    requires_manual_step: bool,
    notes: Vec<String>,
}

#[component]
fn Dashboard() -> impl IntoView {
    view! {
        <main class="shell">
            <p class="eyebrow">"Rust server-rendered Leptos"</p>
            <h1>"Evento Globolo"</h1>
            <p>"A global events operating system combining event discovery, publishing, RSVP, ticketing, community, venue, and organizer workflows."</p>
            <section class="grid">
                <article><h2>"Live state"</h2><p id="live">"Connecting to WebSocket…"</p></article>
                <article><h2>"API"</h2><code>"/v1/events"</code></article>
                <article><h2>"Persistence"</h2><p>"SeaORM + Supabase/PostgreSQL boundary"</p></article>
            </section>
            <a class="button" href="/providers">"Open provider capability map"</a>
        </main>
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let state = AppState {
        api_url: env::var("EVGL_API_URL")
            .unwrap_or_else(|_| "http://localhost:8080".into())
            .parse()?,
        http: reqwest::Client::new(),
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/providers", get(providers))
        .route("/healthz", get(health))
        .route("/ws", get(ws))
        .route("/v1/ws", get(ws))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = env::var("PORT").unwrap_or_else(|_| "8082".into());
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<String> {
    let body = view! { <Dashboard/> }.to_html();
    Html([DOCUMENT_HEAD, &body, DOCUMENT_TAIL].concat())
}

async fn providers(State(state): State<AppState>) -> Html<String> {
    let providers: Vec<ProviderView> = match state.api_url.join("v1/providers") {
        Ok(url) => match state.http.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                response.json().await.unwrap_or_default()
            }
            _ => Vec::new(),
        },
        Err(_) => Vec::new(),
    };
    let body = view! {
        <main class="shell">
            <p>"PROVIDER CAPABILITY MATRIX"</p>
            <h1>"Choose the right delivery mode."</h1>
            <section class="providers">
                {providers.into_iter().map(|provider| {
                    let status = if provider.configured { "Configured" } else { "Needs credentials" };
                    let automation = if provider.capabilities.requires_manual_step {
                        "Manual completion"
                    } else if provider.capabilities.publish {
                        "Automated"
                    } else {
                        "Read only"
                    };
                    view! {
                        <article>
                            <h2 class="provider-name">{provider.capabilities.provider.replace('_', " ")}</h2>
                            <strong>{provider.capabilities.delivery_mode.replace('_', " ")}</strong>
                            <p>{automation}</p>
                            <p>{status}</p>
                            <p>{if provider.capabilities.oauth { "OAuth" } else { "Manual or HMAC secret" }}</p>
                            <ul>
                                {provider.capabilities.notes.into_iter()
                                    .map(|note| view! { <li>{note}</li> })
                                    .collect_view()}
                            </ul>
                        </article>
                    }
                }).collect_view()}
            </section>
        </main>
    }
    .to_html();
    Html([DOCUMENT_HEAD, &body, "</body></html>"].concat())
}

async fn health() -> impl IntoResponse {
    axum::Json(serde_json::json!({"status":"ok","ui":"leptos-ssr"}))
}

async fn ws(upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.max_message_size(64 * 1024).on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    let _ = socket
        .send(Message::Text(
            serde_json::json!({
                "type": "connected",
                "service": "evgl-leptos-web",
                "channel": "provider-jobs",
            })
            .to_string()
            .into(),
        ))
        .await;
    while let Some(Ok(message)) = socket.next().await {
        match message {
            Message::Ping(payload)
                if socket.send(Message::Pong(payload.clone())).await.is_err() =>
            {
                break;
            }
            Message::Text(_) => {
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({
                            "type": "acknowledged",
                            "service": "evgl-leptos-web",
                        })
                        .to_string()
                        .into(),
                    ))
                    .await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rendered_document_preserves_dashboard_and_websocket_bootstrap() {
        let Html(document) = index().await;

        assert!(document.starts_with("<!doctype html>"));
        assert!(document.contains("Evento Globolo"));
        assert!(document.contains("id=\"live\""));
        assert!(document.contains("new WebSocket(`${location.protocol"));
        assert!(document.contains("${location.host}/ws"));
        assert!(document.ends_with("</body></html>"));
    }

    #[test]
    fn websocket_control_channel_is_bounded_and_non_reflective() {
        let source = include_str!("main.rs");
        assert!(source.contains("max_message_size(64 * 1024)"));
        assert!(source.contains("\"type\": \"acknowledged\""));
        assert!(!source.contains("format!(\"ack:{text}\")"));
    }
}
