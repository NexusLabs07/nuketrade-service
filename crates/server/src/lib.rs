use core::types::PlatformsFundingRate;
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use governor::{Quota, RateLimiter};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::num::NonZeroU32;
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

// Rate limiting middleware
async fn rate_limit_middleware(request: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Create a rate limiter that allows 3 requests per second per IP
    static LIMITER: once_cell::sync::Lazy<
        RateLimiter<
            SocketAddr,
            governor::state::keyed::DefaultKeyedStateStore<SocketAddr>,
            governor::clock::DefaultClock,
        >,
    > = once_cell::sync::Lazy::new(|| {
        let quota = Quota::per_second(NonZeroU32::new(3).unwrap());
        RateLimiter::keyed(quota)
    });

    // Get the client IP from the connection info
    let client_ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0)
        .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 0)));

    // Check rate limit
    if LIMITER.check_key(&client_ip).is_err() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

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
