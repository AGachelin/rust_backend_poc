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

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Item {
    pub time: String,
    pub nb_people: i32,
    pub source: String
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Item2 {
    pub nb_people: i32,
}

pub async fn get_item(nb: i64) -> Result<Vec<Item>, sqlx::Error> {
    let database_url = env::var("DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await?;
    let rows = query_as::<_, (OffsetDateTime, i32)>("SELECT time, nb_people FROM line ORDER BY time DESC LIMIT $1")
        .bind(nb)
        .fetch_all(&pool)
        .await
        .unwrap();

    let items = rows.into_iter().map(|(time, nb_people)| {
        Item {
            time: format!("{:02}:{:02}", time.hour(), time.minute()),
            nb_people,
        }
    }).collect();

    Ok(items)
}

async fn query_handler(axum::extract::Path(nb): axum::extract::Path<i64>) -> impl axum::response::IntoResponse {
    match get_item(nb).await {
        Ok(res) => axum::response::Json(json!(res)),
        Err(err) => {println!("{}",err); axum::response::Json(json!({"error": "Failed to get item"}))}
    }
}

pub async fn create_item(nb_people: i32) -> Result<Item, sqlx::Error> {
    let database_url = env::var("DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await?;
    let row = query!(
        "INSERT INTO line (time, nb_people, source) VALUES (NOW(), $1, $2) RETURNING time, nb_people, source",
        nb_people
    )
    .fetch_one(&pool)
    .await.unwrap();

    Ok(Item {
        time: format!("{:02}:{:02}", row.time.hour(), row.time.minute()),
        nb_people: row.nb_people,
    })
}

async fn create_handler(Json(payload): Json<Item2>) -> impl axum::response::IntoResponse {
    match create_item(payload.nb_people).await {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new().route("/web_socket", get(websocket_handler))
                           .route("/", get(|| async { "Hello, World!" }))
                           .route("/get_people/{nb}", get(query_handler))
                           .route("/new_data", post(create_handler))
                           ;
    
    let address = SocketAddr::from(([0, 0, 0, 0], 6942));
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Bound listener to: {}", listener.local_addr().unwrap());
    
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
