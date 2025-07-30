use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};
use shared::{
    abstract_trait::UserServiceTrait,
    database::SharedState,
    domain::{
        ApiResponse, ApiResponsePagination, CreateUserRequest, FindAllUserRequest, KafkaEvent,
        SearchQuery, UpdateUserRequest, UserResponse,
    },
    errors::AppError,
    service::UserServiceImpl,
};
use std::sync::Arc;

async fn get_users(
    State(state): State<SharedState>,
    Query(req): Query<FindAllUserRequest>,
) -> Result<Json<ApiResponsePagination<Vec<UserResponse>>>, AppError> {
    Ok(Json(state.get_users(req).await?))
}

async fn create_user(
    State(state): State<SharedState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    Ok(Json(state.create_user(&req).await?))
}

async fn get_user_by_id(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    match state.find_by_id(&id).await? {
        Some(resp) => Ok(Json(resp)),
        None => Err(AppError::UserNotFound),
    }
}

async fn update_user(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    match state.update_user(&id, &req).await? {
        Some(resp) => Ok(Json(resp)),
        None => Err(AppError::UserNotFound),
    }
}

async fn delete_user(
    State(state): State<SharedState>,
    Path(email): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    Ok(Json(state.delete_user(&email).await?))
}

async fn search_users(
    State(state): State<SharedState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ApiResponsePagination<Vec<UserResponse>>>, AppError> {
    let req = FindAllUserRequest {
        page: 1,
        page_size: 100,
        search: Some(query.q),
    };
    Ok(Json(state.get_users(req).await?))
}

async fn export_csv(State(state): State<SharedState>) -> Result<String, AppError> {
    let event = KafkaEvent::ExportCsv {
        path: "data.csv".to_string(),
    };
    state
        .send_kafka_event(&event)
        .await
        .map_err(|e| AppError::Internal(format!("Kafka send failed: {}", e)))?;
    Ok("📨 Export job queued via Kafka".to_string())
}

async fn import_csv(State(state): State<SharedState>) -> Result<String, AppError> {
    let event = KafkaEvent::ImportCsv {
        path: "users_export.csv".to_string(),
    };
    state
        .send_kafka_event(&event)
        .await
        .map_err(|e| AppError::Internal(format!("Kafka send failed: {}", e)))?;
    Ok("📨 Import job queued via Kafka".to_string())
}

pub fn user_routes(state: Arc<UserServiceImpl>) -> Router {
    Router::new()
        .route("/users", get(get_users).post(create_user))
        .route("/users/{id}", get(get_user_by_id).put(update_user))
        .route("/users/email/{email}", delete(delete_user))
        .route("/users/search", get(search_users))
        .route("/users/export", post(export_csv))
        .route("/users/import", post(import_csv))
        .with_state(state)
}
