use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Item {
    pub time: String,
    pub nb_people: i32,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateItemRequest {
    pub nb_people: i32,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DayRequest {
    pub date: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
