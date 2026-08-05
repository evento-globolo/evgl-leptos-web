use std::env;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures_util::StreamExt;
use leptos::prelude::*;
use tower_http::trace::TraceLayer;

const DOCUMENT_HEAD: &str = r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Evento Globolo</title><style>body{font-family:system-ui;margin:0;background:#f7f7f8}.shell{max-width:960px;margin:auto;padding:4rem 1.5rem}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:1rem}article{background:white;padding:1.25rem;border-radius:14px}</style></head><body>"#;
const DOCUMENT_TAIL: &str = r#"<script>const el=document.getElementById('live');const ws=new WebSocket(`${location.protocol==='https:'?'wss':'ws'}://${location.host}/ws`);ws.onopen=()=>el.textContent='Connected';ws.onmessage=e=>el.textContent=e.data;ws.onclose=()=>el.textContent='Disconnected';</script></body></html>"#;

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
        </main>
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let app = Router::new()
        .route("/", get(index))
        .route("/healthz", get(health))
        .route("/ws", get(ws))
        .layer(TraceLayer::new_for_http());
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = env::var("PORT").unwrap_or_else(|_| "8082".into());
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<String> {
    use leptos::tachys::view::RenderHtml;

    let body = view! { <Dashboard/> }.to_html();
    Html([DOCUMENT_HEAD, &body, DOCUMENT_TAIL].concat())
}

async fn health() -> impl IntoResponse {
    axum::Json(serde_json::json!({"status":"ok","ui":"leptos-ssr"}))
}

async fn ws(upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    let _ = socket
        .send(Message::Text(
            "Evento Globolo realtime channel ready".into(),
        ))
        .await;
    while let Some(Ok(message)) = socket.next().await {
        match message {
            Message::Text(text) => {
                let _ = socket
                    .send(Message::Text(format!("ack:{text}").into()))
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
}
