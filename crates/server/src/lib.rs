use perp_core::{
    config::{self, Config},
    types::LiveMarketFeed,
};
use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Method, Request, StatusCode},
    middleware as axum__middleware,
    response::Response,
    routing::get,
};
use sqlx::PgPool;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::controller::root;

pub mod controller;
pub mod error;
pub mod middleware;
pub mod routes;
pub mod services;
pub mod types;

const RATE_LIMIT_REQUESTS_PER_SECOND: usize = 20;
const RATE_LIMIT_CLEANUP_THRESHOLD_SECS: u64 = 60;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<PgPool>,
    pub live_market_feed: Arc<RwLock<LiveMarketFeed>>,
}

fn get_client_ip(request: &Request<Body>) -> IpAddr {
    // Try X-Forwarded-For header first (most common)
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // X-Forwarded-For can contain multiple IPs, take the first one (original client)
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // Try X-Real-IP header as fallback
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            if let Ok(ip) = real_ip_str.parse::<IpAddr>() {
                return ip;
            }
        }
    }

    // Fallback to connection info if headers are not present
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or_else(|| IpAddr::from([0, 0, 0, 0]))
}

async fn rate_limit_middleware(
    request: Request<Body>,
    next: axum__middleware::Next,
) -> Result<Response, StatusCode> {
    static IP_REQUESTS: once_cell::sync::Lazy<RwLock<HashMap<IpAddr, Vec<Instant>>>> =
        once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

    let client_ip = get_client_ip(&request);
    let now = Instant::now();
    let one_second_ago = now - Duration::from_secs(1);
    let cleanup_threshold = now - Duration::from_secs(RATE_LIMIT_CLEANUP_THRESHOLD_SECS);

    let mut requests = IP_REQUESTS.write().await;

    // Periodically clean up stale IP entries to prevent memory leak
    requests.retain(|_, timestamps| timestamps.last().is_some_and(|&t| t > cleanup_threshold));

    let timestamps = requests.entry(client_ip).or_default();

    // Remove timestamps older than 1 second
    timestamps.retain(|&timestamp| timestamp > one_second_ago);

    // Check if limit exceeded
    if timestamps.len() >= RATE_LIMIT_REQUESTS_PER_SECOND {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    timestamps.push(now);
    drop(requests);

    Ok(next.run(request).await)
}

fn get_cors_origins() -> Vec<HeaderValue> {
    let default_origins = [
        "https://nuketrade.xyz",
        "https://arbitrage-funding-landing-page.vercel.app",
        "http://localhost:3000",
    ];

    let origins_str =
        std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| default_origins.join(","));

    origins_str
        .split(',')
        .filter_map(|s| s.trim().parse::<HeaderValue>().ok())
        .collect()
}

pub async fn run_server(
    config: Config,
    db: Arc<PgPool>,
    live_market_feed: Arc<RwLock<LiveMarketFeed>>,
) -> anyhow::Result<()> {
    let app_state = AppState {
        config,
        db,
        live_market_feed,
    };

    let cors = CorsLayer::new()
        .allow_origin(get_cors_origins())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(true);

    let app = Router::new()
        .route("/", get(root))
        .nest("/user", routes::user::routes())
        .nest("/hyperliquid", routes::hyperliquid::routes())
        .nest("/pacifica", routes::pacifica::routes())
        .nest("/aggregated", routes::aggregated::routes())
        .layer(cors)
        .layer(axum__middleware::from_fn(rate_limit_middleware))
        .with_state(app_state);

    let bind_addr =
        std::env::var("SERVER_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    log::info!("Starting Server on {}...", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
