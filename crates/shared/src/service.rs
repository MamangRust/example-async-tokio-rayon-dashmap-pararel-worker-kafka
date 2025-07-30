use csv::WriterBuilder;
use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::Arc;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    abstract_trait::{UserRepositoryTrait, UserServiceTrait},
    domain::{
        ApiResponse, ApiResponsePagination, CreateUserRequest, FindAllUserRequest, KafkaEvent,
        ServiceStats, UpdateUserRequest, User, UserResponse,
    },
    errors::AppError,
    kafka::producer::KafkaEventProducer,
};

#[derive(Clone)]
pub struct UserServiceImpl {
    pub repo: Arc<dyn UserRepositoryTrait>,
    pub stats: Arc<DashMap<(), ServiceStats>>,
    pub kafka_producer: Option<Arc<KafkaEventProducer>>,
}

impl std::fmt::Debug for UserServiceImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserServiceImpl")
            .field("repo", &"Arc<dyn UserRepositoryTrait>")
            .field("stats", &self.stats)
            .finish()
    }
}

impl UserServiceImpl {
    pub fn new(
        repo: Arc<dyn UserRepositoryTrait>,
        kafka_producer: Option<Arc<KafkaEventProducer>>,
    ) -> Self {
        Self {
            repo,
            stats: Arc::new(DashMap::new()),
            kafka_producer: kafka_producer,
        }
    }

    async fn increment_stat<F>(&self, f: F)
    where
        F: FnOnce(&mut ServiceStats),
    {
        let mut stats = self.stats.entry(()).or_default();
        stats.total_operations += 1;
        f(&mut stats);
    }

    pub async fn get_stats(&self) -> ServiceStats {
        self.stats
            .get(&())
            .map(|s| s.value().clone())
            .unwrap_or_default()
    }

    pub async fn send_kafka_event(&self, event: &KafkaEvent) -> Result<(), String> {
        if let Some(producer) = &self.kafka_producer {
            producer.send(event).await
        } else {
            Err("Kafka producer not enabled".to_string())
        }
    }
}

#[async_trait::async_trait]
impl UserServiceTrait for UserServiceImpl {
    async fn get_users(
        &self,
        req: FindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, AppError> {
        let (users, total) = self
            .repo
            .find_all(req.page, req.page_size, req.search.clone())
            .await?;
        let data = users
            .into_iter()
            .map(|u| UserResponse {
                id: u.id,
                name: u.name,
                email: u.email,
                age: u.age,
            })
            .collect();
        Ok(ApiResponsePagination {
            success: true,
            data,
            page: req.page,
            page_size: req.page_size,
            total,
        })
    }

    async fn create_user(
        &self,
        input: &CreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, AppError> {
        let user = self.repo.create_user(input).await?;
        self.increment_stat(|s| s.create_count += 1).await;
        Ok(ApiResponse {
            success: true,
            data: UserResponse {
                id: user.id,
                name: user.name,
                email: user.email,
                age: user.age,
            },
        })
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<ApiResponse<UserResponse>>, AppError> {
        match self.repo.find_by_id(id).await? {
            Some(user) => {
                self.increment_stat(|s| s.read_count += 1).await;
                Ok(Some(ApiResponse {
                    success: true,
                    data: UserResponse {
                        id: user.id,
                        name: user.name,
                        email: user.email,
                        age: user.age,
                    },
                }))
            }
            None => Ok(None),
        }
    }

    async fn update_user(
        &self,
        id: &str,
        input: &UpdateUserRequest,
    ) -> Result<Option<ApiResponse<UserResponse>>, AppError> {
        match self.repo.update_user(input, id).await {
            Ok(user) => {
                self.increment_stat(|s| s.update_count += 1).await;
                Ok(Some(ApiResponse {
                    success: true,
                    data: UserResponse {
                        id: user.id,
                        name: user.name,
                        email: user.email,
                        age: user.age,
                    },
                }))
            }
            Err(AppError::UserNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn delete_user(&self, email: &str) -> Result<ApiResponse<()>, AppError> {
        self.repo.delete_user(email).await?;
        self.increment_stat(|s| s.delete_count += 1).await;
        Ok(ApiResponse {
            success: true,
            data: (),
        })
    }

    async fn bulk_create_users(&self, inputs: Vec<CreateUserRequest>) -> Result<(), AppError> {
        println!("🎯 Processing {} users in bulk...", inputs.len());

        let futures: Vec<_> = inputs
            .into_par_iter()
            .map(|mut req| {
                req.name = req.name.to_uppercase();
                let service = self.clone();
                async move { service.create_user(&req).await }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        for result in results {
            if let Err(e) = result {
                eprintln!("Failed to create user: {}", e);
            }
        }

        Ok(())
    }

    async fn export_to_csv(&self, path: &str) -> Result<(), AppError> {
        println!("📦 Preparing to export users to CSV: {}", path);

        let users = self.repo.find_all(1, 1_000_000, None).await?.0;
        println!("📊 Retrieved {} users to export", users.len());

        let mut buffer = Vec::with_capacity(1024 * 1024);
        {
            let mut wtr = WriterBuilder::new()
                .has_headers(true)
                .from_writer(&mut buffer);

            for user in &users {
                wtr.serialize(user)
                    .map_err(|e| AppError::CsvError(format!("Failed to serialize user: {}", e)))?;
            }

            wtr.flush().map_err(|e| AppError::CsvError(e.to_string()))?;
        }

        let mut file = File::create(path)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        file.write_all(&buffer)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        file.flush()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        println!("✅ Successfully exported {} users to {}", users.len(), path);
        Ok(())
    }

    async fn import_from_csv(&self, path: &str) -> Result<(), AppError> {
        println!("📊 Reading CSV file: {}", path);

        let mut file = File::open(path)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let cursor = std::io::Cursor::new(contents);
        let mut rdr = csv::Reader::from_reader(cursor);

        let expected_headers = csv::StringRecord::from(vec![
            "id",
            "name",
            "email",
            "age",
            "created_at",
            "updated_at",
        ]);
        let actual_headers = rdr
            .headers()
            .map_err(|e| AppError::CsvError(e.to_string()))?;

        if actual_headers != &expected_headers {
            return Err(AppError::CsvError(
                "Invalid CSV header. Expected: id,name,email,age,created_at,updated_at".to_string(),
            ));
        }

        let mut requests = Vec::new();

        let mut iter = rdr.into_records();
        for result in &mut iter {
            let record = result.map_err(|e| AppError::CsvError(e.to_string()))?;

            if record.len() < 3 {
                return Err(AppError::CsvError(
                    "CSV row has too few fields (need at least name, email, age)".to_string(),
                ));
            }

            let name = record[1].trim();
            let email = record[2].trim();
            let age_str = record[3].trim();

            if name.is_empty() {
                return Err(AppError::ValidationError("Name is empty".to_string()));
            }
            if email.is_empty() {
                return Err(AppError::ValidationError("Email is empty".to_string()));
            }
            if age_str.is_empty() {
                return Err(AppError::ValidationError("Age is empty".to_string()));
            }

            if !email.contains('@') {
                return Err(AppError::ValidationError(
                    "Invalid email format".to_string(),
                ));
            }

            let age: u8 = age_str
                .parse()
                .map_err(|_| AppError::ValidationError("Invalid age: not a number".to_string()))?;

            requests.push(CreateUserRequest {
                name: name.to_string(),
                email: email.to_lowercase(),
                age,
            });
        }

        println!(
            "📦 Found {} records, starting bulk insert...",
            requests.len()
        );

        self.bulk_create_users(requests.clone())
            .await
            .map_err(|e| {
                eprintln!("❌ Bulk create failed: {}", e);
                e
            })?;

        println!(
            "✅ Successfully imported {} users from {}",
            requests.len(),
            path
        );

        Ok(())
    }
}
