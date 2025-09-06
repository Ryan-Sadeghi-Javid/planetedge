use axum::{routing::{get, post}, Router, Json};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteRule {
    id: String,
    pattern: String,
    target: String,
    weight: u8,
}

#[tokio::main]
async fn main() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    fmt().with_env_filter(filter).init();

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/routes", post(save_route).get(list_routes));

    let addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();
    info!("control plane listening on http://{addr}");
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

async fn save_route(Json(rule): Json<RouteRule>) -> Json<RouteRule> {
    // TODO: persist to Raft state machine
    Json(rule)
}

async fn list_routes() -> Json<Vec<RouteRule>> {
    // TODO: read from Raft state machine
    Json(vec![])
}
