mod config;
mod db;
mod handlers;
mod models;

use std::env;
use std::net::SocketAddr;
use axum::Router;
use axum::routing::{get, post};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    let state = AppState { pool };

    let app = Router::new()
        .route(config::ROUTE_HEALTH, get(handlers::health_check))
        .route(config::ROUTE_GET_PEOPLE, get(handlers::get_most_recent_handler))
        .route(config::ROUTE_GET_DAY, post(handlers::get_day_handler))
        .route(config::ROUTE_NEW_DATA, post(handlers::create_handler))
        .route(config::ROUTE_FAKE_DATA, get(handlers::test_data_handler))
        .with_state(state);

    let address = SocketAddr::from((config::SERVER_HOST, config::SERVER_PORT));
    let listener = tokio::net::TcpListener::bind(address).await?;
    
    println!("Server bound to: {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
