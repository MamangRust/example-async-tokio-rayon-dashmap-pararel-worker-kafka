use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use crate::{
    abstract_trait::UserRepositoryTrait,
    database::Database,
    domain::{CreateUserRequest, UpdateUserRequest, User},
    errors::AppError,
};

pub struct InMemoryUserRepository {
    pub db: Database,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            db: Arc::new(DashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl UserRepositoryTrait for InMemoryUserRepository {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<User>, i64), AppError> {
        let users: Vec<User> = self
            .db
            .iter()
            .map(|kv| kv.value().clone())
            .filter(|user| {
                if let Some(ref q) = search {
                    let q = q.to_lowercase();
                    user.name.to_lowercase().contains(&q) || user.email.to_lowercase().contains(&q)
                } else {
                    true
                }
            })
            .collect();
        let total = users.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(users.len());
        let paginated = users[start..end].to_vec();
        Ok((paginated, total))
    }

    async fn find_by_email_exists(&self, email: &str) -> Result<bool, AppError> {
        Ok(self.db.iter().any(|u| u.value().email == email))
    }

    async fn create_user(&self, input: &CreateUserRequest) -> Result<User, AppError> {
        if self.find_by_email_exists(&input.email).await? {
            return Err(AppError::ValidationError(
                "Email already exists".to_string(),
            ));
        }
        let user = User {
            id: Uuid::new_v4().to_string(),
            name: input.name.clone(),
            email: input.email.to_lowercase(),
            age: input.age,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.db.insert(user.id.clone(), user.clone());
        Ok(user)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        Ok(self.db.iter().find_map(|u| {
            if u.value().email == email {
                Some(u.value().clone())
            } else {
                None
            }
        }))
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AppError> {
        Ok(self.db.get(id).map(|u| u.value().clone()))
    }

    async fn update_user(&self, input: &UpdateUserRequest, id: &str) -> Result<User, AppError> {
        {
            let mut user = match self.db.get_mut(id) {
                Some(u) => u,
                None => return Err(AppError::UserNotFound),
            };
            if let Some(name) = &input.name {
                user.name = name.clone();
            }
            if let Some(email) = &input.email {
                user.email = email.to_lowercase();
            }
            if let Some(age) = input.age {
                user.age = age;
            }
            user.updated_at = chrono::Utc::now();
        }
        self.find_by_id(id).await?.ok_or(AppError::UserNotFound)
    }

    async fn delete_user(&self, email: &str) -> Result<(), AppError> {
        let key = self.db.iter().find_map(|entry| {
            if entry.value().email == email {
                Some(entry.key().clone())
            } else {
                None
            }
        });
        if let Some(k) = key {
            self.db.remove(&k);
            Ok(())
        } else {
            Err(AppError::UserNotFound)
        }
    }
}
