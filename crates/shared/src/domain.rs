use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub age: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub age: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct FindAllUserRequest {
    pub page: i32,
    pub page_size: i32,
    pub search: Option<String>,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub age: u8,
}

#[derive(Debug, Default, Clone)]
pub struct ServiceStats {
    pub total_operations: u64,
    pub create_count: u64,
    pub read_count: u64,
    pub update_count: u64,
    pub delete_count: u64,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
}

#[derive(Serialize)]
pub struct ApiResponsePagination<T> {
    pub success: bool,
    pub data: T,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum KafkaEvent {
    ImportCsv { path: String },
    ExportCsv { path: String },
}
