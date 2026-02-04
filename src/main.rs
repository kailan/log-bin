mod channel_manager;
mod models;
mod parsers;
use memorable_ids::{generate, suffix_generators, GenerateOptions};

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response, Sse},
    routing::get,
    Router,
};
use futures_util::StreamExt;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::{info, warn};

// Embed static files into the binary
static INDEX_HTML: &str = include_str!("../client/dist/index.html");

use channel_manager::ChannelManager;
use models::LogEvent;
use parsers::ParsedEvent;

const MAX_SUBSCRIBERS_PER_STREAM: usize = 30;
const MIN_BUCKET_ID_LENGTH: usize = 10;

// Security headers for HTML responses
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; base-uri 'self'; form-action 'self'";

fn security_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_SECURITY_POLICY, CSP.parse().unwrap());
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers
}

#[derive(Clone)]
struct AppState {
    channel_manager: Arc<RwLock<ChannelManager>>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let state = AppState {
        channel_manager: Arc::new(RwLock::new(ChannelManager::new())),
    };

    // Start garbage collection task
    let gc_manager = state.channel_manager.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            gc_manager.write().await.garbage_collect().await;
        }
    });

    // Build our application with routes
    // Routes defined after a layer are affected by that layer
    // Cache-Control applies to assets and bucket routes only
    let app = Router::new()
        .nest_service(
            "/assets",
            ServeDir::new("client/dist/assets").precompressed_gzip(),
        )
        .route("/{bucket_id}", get(get_bucket).post(post_events))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("public, max-age=31536000, immutable"),
        ))
        .route("/", get(redirect_to_random_bucket))
        .route("/liveness_check", get(health_check))
        .route("/readiness_check", get(health_check))
        .route(
            "/.well-known/fastly/logging/challenge",
            get(fastly_challenge),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .expose_headers([header::CONTENT_TYPE]),
        )
        .with_state(state);

    // Determine port from environment or use default
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    info!("Server shut down gracefully");
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            info!("Received SIGTERM, shutting down...");
        }
    }
}

async fn health_check() -> &'static str {
    "OK"
}

async fn fastly_challenge() -> &'static str {
    "*"
}

async fn redirect_to_random_bucket() -> impl IntoResponse {
    let bucket_id = generate(GenerateOptions {
        components: 2,
        suffix: Some(suffix_generators::number),
        ..Default::default()
    })
    .unwrap();

    let mut headers = security_headers();
    headers.insert(header::LOCATION, format!("/{}", bucket_id).parse().unwrap());

    (StatusCode::FOUND, headers)
}

async fn get_bucket(
    Path(bucket_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    if bucket_id.len() < MIN_BUCKET_ID_LENGTH {
        return Err(StatusCode::NOT_FOUND);
    }

    // Check if client wants event stream
    if let Some(accept) = headers.get(header::ACCEPT) {
        if accept == "text/event-stream" {
            // Parse max subscribers from bucket ID (if specified)
            let max_subs = bucket_id
                .split(";max-subs=")
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(MAX_SUBSCRIBERS_PER_STREAM);

            let channel = {
                let mut manager = state.channel_manager.write().await;
                manager.get_or_create_channel(&bucket_id)
            };

            if channel.subscriber_count() >= max_subs {
                warn!("Stream {} rejected: max subscribers reached", bucket_id);
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }

            info!("New subscriber to bucket: {}", bucket_id);

            // Subscribe and send stats update
            let stream = channel.subscribe().await;
            let stats = channel.get_stats();
            channel.publish_stats(stats).await;

            let sse_stream =
                stream.map(|event| -> Result<axum::response::sse::Event, Infallible> {
                    Ok(axum::response::sse::Event::default()
                        .event(&event.event_type)
                        .data(event.data))
                });

            // Add headers to prevent proxy/CDN caching or buffering
            let mut sse_headers = HeaderMap::new();
            sse_headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
            sse_headers.insert("X-Accel-Buffering", "no".parse().unwrap());

            let sse_response = Sse::new(sse_stream)
                .keep_alive(
                    axum::response::sse::KeepAlive::new()
                        .interval(std::time::Duration::from_secs(15)),
                )
                .into_response();

            let (mut parts, body) = sse_response.into_parts();
            parts.headers.extend(sse_headers);
            return Ok(Response::from_parts(parts, body));
        }
    }

    // Otherwise serve the HTML viewer with no caching to avoid CDN issues
    let mut headers = security_headers();
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=3600".parse().unwrap(),
    );
    headers.insert(header::VARY, "Accept".parse().unwrap());

    Ok((headers, Html(INDEX_HTML)).into_response())
}

async fn post_events(
    Path(bucket_id): Path<String>,
    State(state): State<AppState>,
    body: String,
) -> StatusCode {
    if body.is_empty() {
        return StatusCode::BAD_REQUEST;
    }

    let lines: Vec<&str> = body.split('\n').filter(|line| !line.is_empty()).collect();

    if lines.is_empty() {
        return StatusCode::BAD_REQUEST;
    }

    info!(
        "New events for bucket {}: {} events",
        bucket_id,
        lines.len()
    );

    let channel = {
        let mut manager = state.channel_manager.write().await;
        manager.get_or_create_channel(&bucket_id)
    };

    for line in lines {
        let mut event = ParsedEvent::new(line.to_string());
        event.parse();

        let log_event = LogEvent {
            time: event.time,
            raw: line.to_string(),
            fields: event.fields,
            parser: event.parser,
        };

        channel.publish_log(log_event).await;
    }

    StatusCode::NO_CONTENT
}
