use std::env;
use std::net::SocketAddr;
use axum::extract::ws::CloseFrame;
use axum::Router;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Json,
    response::IntoResponse,
    routing::{get, post},
    http::StatusCode,
};
use sqlx::{PgPool, Error, query_as, query};
use time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Item {
    pub time: String,
    pub nb_people: i32,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Item2 {
    pub nb_people: i32,
    pub source: Option<String>,
}

pub async fn get_item(pool: &PgPool, nb: i64) -> Result<Vec<Item>, sqlx::Error> {
    let rows = query_as::<_, (OffsetDateTime, i32, Option<String>)>(
        "SELECT time, nb_people, source FROM line ORDER BY time DESC LIMIT $1",
    )
    .bind(nb)
    .fetch_all(pool)
    .await
    .unwrap();

    let items = rows
        .into_iter()
        .map(|(time, nb_people, source)| Item {
            time: format!("{:02}:{:02}", time.hour(), time.minute()),
            nb_people,
            source,
        })
        .collect();

    Ok(items)
}

async fn query_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(nb): axum::extract::Path<i64>
) -> impl axum::response::IntoResponse {
    match get_item(&state.pool, nb).await {
        Ok(res) => (StatusCode::OK, axum::response::Json(json!(res))).into_response(),
        Err(err) => {println!("{}",err); (StatusCode::INTERNAL_SERVER_ERROR, axum::response::Json(json!({"error": "Failed to get item"}))).into_response()}
    }
}

pub async fn create_item(
    pool: &PgPool,
    nb_people: i32,
    source: Option<String>,
) -> Result<Item, sqlx::Error> {
    let row = query!(
        "INSERT INTO line (time, nb_people, source) VALUES (NOW(), $1, $2) RETURNING time, nb_people, source",
        nb_people,
        source
    )
    .fetch_one(pool)
    .await
    .unwrap();

    Ok(Item {
        time: format!("{:02}:{:02}", row.time.hour(), row.time.minute()),
        nb_people: row.nb_people,
        source: row.source,
    })
}

async fn create_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<Item2>
) -> impl axum::response::IntoResponse {
    match create_item(&state.pool, payload.nb_people, payload.source).await {
        Ok(item) => (StatusCode::CREATED, axum::response::Json(json!(item))).into_response(),
        Err(err) => {println!("{}",err); (StatusCode::INTERNAL_SERVER_ERROR, axum::response::Json(json!({"error": "Failed to create item"}))).into_response()},
    }
}

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error))
        .on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(utf8_bytes) => {
                    println!("Text received: {}", utf8_bytes);
                    let result = socket
                        .send(Message::Text(
                            format!("Echo back text: {}", utf8_bytes).into(),
                        ))
                        .await;
                    if let Err(error) = result {
                        println!("Error sending: {}", error);
                        send_close_message(socket, 1011, &format!("Error occured: {}", error))
                            .await;
                        break;
                    }
                }
                Message::Binary(bytes) => {
                    println!("Received bytes of length: {}", bytes.len());
                    let result = socket
                        .send(Message::Text(
                            format!("Received bytes of length: {}", bytes.len()).into(),
                        ))
                        .await;
                    if let Err(error) = result {
                        println!("Error sending: {}", error);
                        send_close_message(socket, 1011, &format!("Error occured: {}", error))
                            .await;
                        break;
                    }
                }
                _ => {}
            }
        } else {
            let error = msg.err().unwrap();
            println!("Error receiving message: {:?}", error);
            send_close_message(socket, 1011, &format!("Error occured: {}", error)).await;
            break;
        }
    }
}

async fn send_close_message(mut socket: WebSocket, code: u16, reason: &str) {
    _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: code,
            reason: reason.into(),
        })))
        .await;
}

#[derive(Deserialize)]
pub struct DayRequest {
    pub date: String,
}

pub async fn get_day(pool: &PgPool, date: &str) -> Result<Vec<Item>, sqlx::Error> {
    let rows = query_as::<_, (OffsetDateTime, i32, Option<String>)>(
        "SELECT time, nb_people, source FROM line WHERE time >= $1::date AND time < ($1::date + INTERVAL '1 day') ORDER BY time DESC",
    )
    .bind(date)
    .fetch_all(pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|(time, nb_people, source)| Item {
            time: format!("{:02}:{:02}", time.hour(), time.minute()),
            nb_people,
            source,
        })
        .collect();

    Ok(items)
}

async fn query_day_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<DayRequest>,
) -> impl axum::response::IntoResponse {
    match get_day(&state.pool, &payload.date).await {
        Ok(res) => (StatusCode::OK, axum::response::Json(json!(res))).into_response(),
        Err(err) => {println!("{}",err); (StatusCode::INTERNAL_SERVER_ERROR, axum::response::Json(json!({"error": "Failed to get people for the day"}))).into_response()}
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    let state = AppState { pool };

    let app = Router::new()
        .route("/web_socket", get(websocket_handler))
        .route("/", get(|| async { "Hello, World!" }))
        .route("/get_people/{nb}", get(query_handler))
        .route("/get_day", post(query_day_handler))
        .route("/new_data", post(create_handler))
        .with_state(state);
    
    let address = SocketAddr::from(([0, 0, 0, 0], 6942));
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Bound listener to: {}", listener.local_addr().unwrap());
    
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
