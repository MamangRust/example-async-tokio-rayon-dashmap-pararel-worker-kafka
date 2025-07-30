use crate::{
    domain::{
        ApiResponse, ApiResponsePagination, CreateUserRequest, FindAllUserRequest,
        UpdateUserRequest, User, UserResponse,
    },
    errors::AppError,
};

#[async_trait::async_trait]
pub trait UserRepositoryTrait: Send + Sync {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<User>, i64), AppError>;
    async fn find_by_email_exists(&self, email: &str) -> Result<bool, AppError>;
    async fn create_user(&self, input: &CreateUserRequest) -> Result<User, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AppError>;
    async fn update_user(&self, input: &UpdateUserRequest, id: &str) -> Result<User, AppError>;
    async fn delete_user(&self, email: &str) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait UserServiceTrait: Send + Sync {
    async fn get_users(
        &self,
        req: FindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, AppError>;
    async fn create_user(
        &self,
        input: &CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<ApiResponse<UserResponse>>, AppError>;
    async fn update_user(
        &self,
        id: &str,
        input: &UpdateUserRequest,
    ) -> Result<Option<ApiResponse<UserResponse>>, AppError>;
    async fn delete_user(&self, email: &str) -> Result<ApiResponse<()>, AppError>;
    async fn bulk_create_users(&self, inputs: Vec<CreateUserRequest>) -> Result<(), AppError>;
    async fn export_to_csv(&self, path: &str) -> Result<(), AppError>;
    async fn import_from_csv(&self, path: &str) -> Result<(), AppError>;
}
