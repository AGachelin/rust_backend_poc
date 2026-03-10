use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    db,
    models::{CreateItemRequest, DayRequest, ErrorResponse},
    AppState,
};

pub async fn health_check() -> impl IntoResponse {
    "Hello, World!"
}

pub async fn get_most_recent_handler(
    State(state): State<AppState>,
    Path(nb): Path<i64>,
) -> impl IntoResponse {
    match db::fetch_recent_items(&state.pool, nb).await {
        Ok(items) => (StatusCode::OK, Json(items)).into_response(),
        Err(err) => {
            eprintln!("Failed to fetch recent items: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get items".to_string(),
                }),
            )
                .into_response()
        }
    }
}

pub async fn get_day_handler(
    State(state): State<AppState>,
    Json(payload): Json<DayRequest>,
) -> impl IntoResponse {
    match db::fetch_day_items(&state.pool, &payload.date).await {
        Ok(items) => (StatusCode::OK, Json(items)).into_response(),
        Err(err) => {
            eprintln!("Failed to fetch items for a day: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get people for the day".to_string(),
                }),
            )
                .into_response()
        }
    }
}

pub async fn create_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateItemRequest>,
) -> impl IntoResponse {
    match db::create_item(&state.pool, payload.nb_people, payload.source).await {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(err) => {
            eprintln!("Failed to create item: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create item".to_string(),
                }),
            )
                .into_response()
        }
    }
}

pub async fn test_data_handler(
        State(state): State<AppState>
) -> impl IntoResponse {
    match db::test_data(&state.pool).await {
        Ok(_) => (StatusCode::OK, Json("Fake data generated successfully")).into_response(),
        Err(err) => {
            eprintln!("Failed to generate fake data: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to generate fake data".to_string(),
                }),
            )
                .into_response()
        }
    }
}   
