use std::env;
use std::net::SocketAddr;
use axum::extract::ws::CloseFrame;
use axum::Router;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Json,
    response::IntoResponse,
    routing::{get, post},
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
        Ok(res) => axum::response::Json(json!(res)),
        Err(err) => {println!("{}",err); axum::response::Json(json!({"error": "Failed to get item"}))}
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
        Ok(item) => axum::response::Json(json!(item)),
        Err(err) => {println!("{}",err); axum::response::Json(json!({"error": "Failed to create item"}))},
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

pub async fn get_past_day(pool: &PgPool) -> Result<Vec<Item>, sqlx::Error> {
    let rows = query_as::<_, (OffsetDateTime, i32, Option<String>)>(
        "SELECT time, nb_people, source FROM line WHERE DATE(time) = DATE(NOW()) ORDER BY time DESC",
    )
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

pub async fn get_day_before(pool: &PgPool) -> Result<Vec<Item>, sqlx::Error> {
    let rows = query_as::<_, (OffsetDateTime, i32, Option<String>)>(
        "SELECT time, nb_people, source FROM line WHERE DATE(time) = DATE(NOW()) - INTERVAL '1 day' ORDER BY time DESC",
    )
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

#[derive(Deserialize)]
pub struct DayRequest {
    pub date: String,
}

pub async fn get_people_per_hour(pool: &PgPool, date: &str) -> Result<Vec<Item>, sqlx::Error> {
    let rows = query_as::<_, (OffsetDateTime, i32, Option<String>)>(
        "SELECT date_trunc('hour', time) AS time, SUM(nb_people)::int AS nb_people, NULL::text AS source FROM line WHERE time >= $1::date AND time < ($1::date + INTERVAL '1 day') GROUP BY date_trunc('hour', time) ORDER BY time ASC;",
    )
    .bind(date)
    .fetch_all(pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|(time, nb_people, source)| Item {
            time: format!("{:02}:00", time.hour()),
            nb_people,
            source,
        })
        .collect();

    Ok(items)
}

async fn past_day_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    match get_past_day(&state.pool).await {
        Ok(res) => axum::response::Json(json!(res)),
        Err(err) => {println!("{}",err); axum::response::Json(json!({"error": "Failed to get past day data"}))}
    }
}

async fn people_per_hour_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<DayRequest>,
) -> impl axum::response::IntoResponse {
    match get_people_per_hour(&state.pool, &payload.date).await {
        Ok(res) => axum::response::Json(json!(res)),
        Err(err) => {println!("{}",err); axum::response::Json(json!({"error": "Failed to get people per hour"}))}
    }
}

async fn day_before_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    match get_day_before(&state.pool).await {
        Ok(res) => axum::response::Json(json!(res)),
        Err(err) => {println!("{}",err); axum::response::Json(json!({"error": "Failed to get day before data"}))}
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
        .route("/get_today", get(past_day_handler))
        .route("/get_yesterday", get(day_before_handler))
        .route("/people_per_hour", post(people_per_hour_handler))
        .route("/new_data", post(create_handler))
        .with_state(state);
    
    let address = SocketAddr::from(([0, 0, 0, 0], 6942));
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Bound listener to: {}", listener.local_addr().unwrap());
    
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
