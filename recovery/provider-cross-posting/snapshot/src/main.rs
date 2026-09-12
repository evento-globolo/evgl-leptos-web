use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use leptos::{prelude::*, tachys::view::RenderHtml};
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use url::Url;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();
    let state = AppState {
        api_url: std::env::var("EVGL_API_URL")
            .unwrap_or_else(|_| "http://localhost:8080".into()).parse()?,
        http: reqwest::Client::new(),
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/providers", get(providers))
        .route("/v1/ws", get(websocket))
        .route("/healthz", get(|| async {
            Json(serde_json::json!({"status":"ok","service":"evgl-web-leptos"}))
        }))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let bind = std::env::var("APP_BIND").unwrap_or_else(|_| "0.0.0.0:3100".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn websocket(upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let connected = serde_json::json!({
        "type": "connected",
        "service": "evgl-web-leptos",
        "channel": "provider-jobs",
    });
    if socket.send(Message::Text(connected.to_string().into())).await.is_err() {
        return;
    }

    while let Some(message) = socket.recv().await {
        match message {
            Ok(Message::Ping(payload)) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Text(_)) => {
                let acknowledgement = serde_json::json!({
                    "type": "acknowledged",
                    "service": "evgl-web-leptos",
                });
                if socket.send(Message::Text(acknowledgement.to_string().into())).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
}

async fn index() -> Html<String> {
    Html(document(view! {
        <main class="shell">
            <p class="eyebrow">"LEPTOS OPERATIONS CONSOLE"</p>
            <h1>"Cross-post with capabilities, not assumptions."</h1>
            <p class="lede">
                "The Rust server reads the same provider matrix used by jobs, the CLI, and other clients."
            </p>
            <a class="button" href="/providers">"Inspect providers →"</a>
        </main>
    }))
}

async fn providers(State(state): State<AppState>) -> impl IntoResponse {
    let url = match state.api_url.join("v1/providers") {
        Ok(url) => url,
        Err(_) => return Html(document(view! { <p>"Invalid API URL"</p> })),
    };
    let result = state.http.get(url).send().await;
    let providers: Vec<ProviderView> = match result {
        Ok(response) if response.status().is_success() =>
            response.json().await.unwrap_or_default(),
        _ => vec![],
    };
    Html(document(view! {
        <main class="shell">
            <p class="eyebrow">"PROVIDER CAPABILITY MATRIX"</p>
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
                            <div class="top">
                                <h2>{provider.capabilities.provider.replace('_', " ")}</h2>
                                <span>{status}</span>
                            </div>
                            <strong>{provider.capabilities.delivery_mode.replace('_', " ")}</strong>
                            <p>{automation}</p>
                            <ul>
                                {provider.capabilities.notes.into_iter()
                                    .map(|note| view! { <li>{note}</li> })
                                    .collect_view()}
                            </ul>
                            <small>{if provider.capabilities.oauth { "OAuth account connection" } else { "Manual/secret connection" }}</small>
                        </article>
                    }
                }).collect_view()}
            </section>
        </main>
    }))
}

fn document(content: impl RenderHtml) -> String {
    let body = content.to_html();
    format!(r#"<!doctype html><html lang="en"><head><meta charset="utf-8">
        <meta name="viewport" content="width=device-width"><title>Evento Globolo · Leptos</title>
        <style>{}</style></head><body>{}</body></html>"#, STYLES, body)
}

const STYLES: &str = r#"
  :root{--ink:#111311;--paper:#f5f1e8;--acid:#d9ff43;--line:#cfccc4;--muted:#6a706c}
  *{box-sizing:border-box}body{margin:0;background:var(--paper);color:var(--ink);font:16px/1.5 system-ui,sans-serif}
  .shell{width:min(1120px,calc(100% - 40px));margin:auto;padding:90px 0}.eyebrow{font-size:12px;font-weight:800;letter-spacing:.18em}
  h1{font-size:clamp(54px,8vw,105px);line-height:.9;letter-spacing:-.065em;max-width:1000px;margin:25px 0}.lede{font-size:20px;color:var(--muted);max-width:700px}
  .button{display:inline-block;background:var(--ink);color:white;padding:15px 20px;text-decoration:none;font-weight:700;margin-top:30px}
  .providers{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:12px;margin-top:55px}.providers article{background:white;border:1px solid var(--line);padding:25px;min-height:280px}
  .top{display:flex;justify-content:space-between;gap:15px}.top h2{text-transform:capitalize;margin:0;font-size:27px}.top span{background:var(--acid);padding:5px 8px;font-size:11px;height:max-content}
  article>strong{text-transform:uppercase;font-size:11px;letter-spacing:.1em;color:var(--muted)}ul{padding-left:18px;color:var(--muted)}small{font-weight:700}
"#;
