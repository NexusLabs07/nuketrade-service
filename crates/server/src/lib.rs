use core::types::PlatformsFundingRate;
use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use sqlx::PgPool;
use std::net::{IpAddr, SocketAddr};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::{
    controller::{
        add_to_waitlist, get_funding_rate, get_referral_count, get_total_points, get_total_users,
        get_user_position, root,
    },
    types::AppState,
};

pub mod controller;
pub mod error;
pub mod types;

// Helper function to extract real client IP from headers (for proxy/Railway support)
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

// Rate limiting middleware - allows only 5 requests per IP total
async fn rate_limit_middleware(request: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Track request count per IP address
    static IP_COUNTER: once_cell::sync::Lazy<RwLock<HashMap<IpAddr, u32>>> =
        once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

    // Get the real client IP (handles proxy/Railway forwarded headers)
    let client_ip = get_client_ip(&request);

    // Check and update request count
    let mut counter = IP_COUNTER.write().await;
    let count = counter.entry(client_ip).or_insert(0);

    if *count >= 5 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    *count += 1;
    drop(counter); // Release the lock before proceeding

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
        .route("/funding-rate", get(get_funding_rate))
        .route("/add-to-waitlist", post(add_to_waitlist))
        .route("/total-users", get(get_total_users))
        .route("/total-points/{user_id}", get(get_total_points))
        .route("/referral-count/{referral_code}", get(get_referral_count))
        .route("/user-position/{user_id}", get(get_user_position))
        .layer(cors)
        .layer(middleware::from_fn(rate_limit_middleware))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    log::info!("Starting Server...");

    axum::serve(listener, app).await.unwrap();
}
