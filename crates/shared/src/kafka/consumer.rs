use crate::{abstract_trait::UserServiceTrait, domain::KafkaEvent};
use futures::StreamExt;
use rdkafka::{
    Message,
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
};
use std::sync::Arc;
use tokio::task;

pub struct KafkaEventConsumer {
    consumer: StreamConsumer,
    user_service: Arc<dyn UserServiceTrait>,
}

impl KafkaEventConsumer {
    pub async fn new(
        brokers: &str,
        group_id: &str,
        topic: &str,
        user_service: Arc<dyn UserServiceTrait>,
    ) -> Self {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", brokers)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "smallest")
            .set("session.timeout.ms", "6000")
            .create()
            .expect("Failed to create Kafka consumer");

        consumer
            .subscribe(&[topic])
            .expect("Can't subscribe to topic");

        Self {
            consumer,
            user_service,
        }
    }

    pub async fn start_listening(self) {
        let mut stream = self.consumer.stream();

        println!("👂 Kafka consumer listening for events...");

        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        match serde_json::from_slice::<KafkaEvent>(payload) {
                            Ok(event) => {
                                let service = self.user_service.clone();
                                task::spawn(async move {
                                    Self::handle_event(event, service).await;
                                });
                            }
                            Err(e) => eprintln!("❌ Failed to parse Kafka event: {}", e),
                        }
                    }
                }
                Err(e) => eprintln!("Kafka error: {}", e),
            }
        }
    }

    async fn handle_event(event: KafkaEvent, service: Arc<dyn UserServiceTrait>) {
        match event {
            KafkaEvent::ImportCsv { path } => {
                println!("📥 Handling import from CSV: {}", path);
                if let Err(e) = service.import_from_csv(&path).await {
                    eprintln!("❌ Failed to import from {}: {}", path, e);
                } else {
                    println!("✅ Successfully imported from {}", path);
                }
            }
            KafkaEvent::ExportCsv { path } => {
                println!("📤 Handling export to CSV: {}", path);
                if let Err(e) = service.export_to_csv(&path).await {
                    eprintln!("❌ Export failed: {}", e);
                } else {
                    println!("✅ Exported to {}", path);
                }
            }
        }
    }
}
