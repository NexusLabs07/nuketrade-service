use core::types::PlatformsFundingRate;
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
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::controller::{get_funding_rate, root};

pub mod controller;
pub mod error;
pub mod middleware;
pub mod routes;
pub mod types;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: Arc<PgPool>,
    pub platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
}

// Helper function to extract real client IP from headers (for proxy/Railway support)
fn get_client_ip(request: &Request<Body>) -> IpAddr {
    // Try X-Forwarded-For header first (most common)
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            log::info!("X-Forwarded-For header: {}", forwarded_str);

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
        log::info!("X-Real-IP header: {:?}", real_ip);
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

// Rate limiting middleware - allows 50 requests per second per IP
async fn rate_limit_middleware(
    request: Request<Body>,
    next: axum__middleware::Next,
) -> Result<Response, StatusCode> {
    use std::time::{Duration, Instant};

    // Track request timestamps per IP address
    static IP_REQUESTS: once_cell::sync::Lazy<RwLock<HashMap<IpAddr, Vec<Instant>>>> =
        once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

    // Get the real client IP (handles proxy/Railway forwarded headers)
    let client_ip = get_client_ip(&request);

    let now = Instant::now();
    let one_second_ago = now - Duration::from_secs(1);

    // Check and update request timestamps
    let mut requests = IP_REQUESTS.write().await;
    let timestamps = requests.entry(client_ip).or_insert_with(Vec::new);

    // Remove timestamps older than 1 second
    timestamps.retain(|&timestamp| timestamp > one_second_ago);

    // Check if limit exceeded
    if timestamps.len() >= 20 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Add current request timestamp
    timestamps.push(now);
    drop(requests); // Release the lock before proceeding

    Ok(next.run(request).await)
}

pub async fn run_server(
    db: Arc<PgPool>,
    platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
) {
    let app_state = AppState {
        db,
        platforms_funding_rate,
    };

    // Configure CORS to allow requests from specific frontend origins
    let cors = CorsLayer::new()
        .allow_origin([
            "https://nuketrade.xyz".parse::<HeaderValue>().unwrap(),
            "https://arbitrage-funding-landing-page.vercel.app"
                .parse::<HeaderValue>()
                .unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(), // For local development
        ])
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
        .route("/funding-rate", get(get_funding_rate))
        .layer(cors)
        .layer(axum__middleware::from_fn(rate_limit_middleware))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    log::info!("Starting Server...");

    axum::serve(listener, app).await.unwrap();
}
