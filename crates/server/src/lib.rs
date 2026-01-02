use axum::{Router, routing::get};

use crate::controller::root;

pub mod controller;

pub async fn run_server() {
    let app = Router::new().route("/", get(root));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
