use dashmap::DashMap;
use server::api::user_routes;
use shared::{
    kafka::{consumer::KafkaEventConsumer, producer::KafkaEventProducer},
    repository::InMemoryUserRepository,
    service::UserServiceImpl,
};
use std::{env, sync::Arc};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();
    let db = Arc::new(DashMap::new());
    let repo = Arc::new(InMemoryUserRepository { db: db.clone() });

    let kafka_producer: Option<Arc<KafkaEventProducer>> = Some(Arc::new(KafkaEventProducer::new(
        "172.17.0.2:9092",
        "user-jobs",
    )));

    let service = Arc::new(UserServiceImpl::new(repo, kafka_producer));

    match args.get(1).map(|s| s.as_str()) {
        Some("worker") => {
            println!("👷 Worker mode: consuming from Kafka");
            let consumer = KafkaEventConsumer::new(
                "172.17.0.2:9092",
                "user-worker-group",
                "user-jobs",
                service.clone(),
            );
            consumer.await.start_listening().await;
        }
        Some("server") | None => {
            let addr = "0.0.0.0:5000";
            let listener = TcpListener::bind(addr).await?;
            println!("🚀 Server running on http://{}", addr);
            let router = user_routes(service);
            axum::serve(listener, router).await?;
        }
        Some(unknown) => {
            eprintln!(
                "❌ Unknown mode: {}. Usage: {} [server|worker]",
                unknown, args[0]
            );
            std::process::exit(1);
        }
    }
    Ok(())
}
