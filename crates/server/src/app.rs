use crate::features::{aggregated, auth, bridge, hedge, hyperliquid, pacifica, user};
use crate::middleware::auth::require_post_auth;
use crate::state::AppState;
// use axum::extract::State;
use axum::Router;
use axum::routing::get;
// use perp_core::LiveMarketFeed;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use axum::{
    body::Body,
    http::{HeaderValue, Method, Request, StatusCode},
    middleware as axum__middleware,
    response::Response,
};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

const RATE_LIMIT_REQUESTS_PER_SECOND: usize = 20;
const RATE_LIMIT_CLEANUP_THRESHOLD_SECS: u64 = 60;

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

pub async fn root() -> &'static str {
    "Perpetual Aggregator Server is running."
}

pub fn create_app(app_state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(get_cors_origins())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(true);

    let auth_state = app_state.clone();

    Router::new()
        .route("/", get(root))
        .nest("/auth", auth::routes::routes())
        .nest("/user", user::routes::routes())
        .nest("/hyperliquid", hyperliquid::routes::routes())
        .nest("/pacifica", pacifica::routes::routes())
        .nest("/aggregated", aggregated::routes::routes())
        .nest("/bridge", bridge::routes::routes())
        .nest("/hedge-intents", hedge::routes::routes())
        .layer(cors)
        .layer(axum__middleware::from_fn(rate_limit_middleware))
        .layer(axum__middleware::from_fn_with_state(
            auth_state,
            require_post_auth,
        ))
        .with_state(app_state)
}
