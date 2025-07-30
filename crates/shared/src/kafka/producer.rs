use crate::domain::KafkaEvent;
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
};
use std::time::Duration;

pub struct KafkaEventProducer {
    producer: FutureProducer,
    topic: String,
}

impl KafkaEventProducer {
    pub fn new(brokers: &str, topic: &str) -> Self {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Failed to create Kafka producer");

        Self {
            producer,
            topic: topic.to_owned(),
        }
    }

    pub async fn send(&self, event: &KafkaEvent) -> Result<(), String> {
        let payload = serde_json::to_vec(event).map_err(|e| e.to_string())?;
        let key = format!("{:?}", event);
        let record = FutureRecord::to(&self.topic).payload(&payload).key(&key);

        self.producer
            .send(record, Timeout::After(Duration::from_secs(2)))
            .await
            .map_err(|(e, _)| e.to_string())?;

        Ok(())
    }
}
