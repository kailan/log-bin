use crate::models::{LogEvent, SseEvent, StatsEvent};
use futures_util::stream::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

const HISTORY_SIZE: usize = 10;
const GC_WAIT_MS: u64 = 10000;

/// Guard that removes a client from the clients map when dropped
struct ClientGuard {
    client_id: String,
    clients: Arc<RwLock<HashMap<String, ()>>>,
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        let client_id = self.client_id.clone();
        let clients = self.clients.clone();
        tokio::spawn(async move {
            clients.write().await.remove(&client_id);
            info!("Client {} disconnected and removed", client_id);
        });
    }
}

pub struct Channel {
    sender: broadcast::Sender<SseEvent>,
    history: Arc<RwLock<Vec<SseEvent>>>,
    clients: Arc<RwLock<HashMap<String, ()>>>,
}

impl Channel {
    pub fn new(_name: String) -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            sender,
            history: Arc::new(RwLock::new(Vec::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    pub async fn subscribe(&self) -> Pin<Box<dyn Stream<Item = SseEvent> + Send>> {
        let client_id = Uuid::new_v4().to_string();
        self.clients.write().await.insert(client_id.clone(), ());

        let mut receiver = self.sender.subscribe();
        let history = self.history.read().await.clone();

        // Create a guard that will remove the client when the stream is dropped
        let _guard = ClientGuard {
            client_id,
            clients: self.clients.clone(),
        };

        Box::pin(async_stream::stream! {
            // Move guard into the stream so it's dropped when the stream is dropped
            let _guard = _guard;

            // Send history first
            for event in history {
                yield event;
            }

            // Then stream new events
            while let Ok(event) = receiver.recv().await {
                yield event;
            }
        })
    }

    pub async fn publish_log(&self, event: LogEvent) {
        let data = serde_json::to_string(&event).unwrap();
        let sse_event = SseEvent {
            event_type: "log".to_string(),
            data,
        };

        // Add to history
        let mut history = self.history.write().await;
        history.push(sse_event.clone());
        if history.len() > HISTORY_SIZE {
            history.remove(0);
        }
        drop(history);

        // Broadcast to all subscribers
        let _ = self.sender.send(sse_event);
    }

    pub async fn publish_stats(&self, stats: StatsEvent) {
        let data = serde_json::to_string(&stats).unwrap();
        let sse_event = SseEvent {
            event_type: "stats".to_string(),
            data,
        };
        let _ = self.sender.send(sse_event);
    }

    pub fn get_stats(&self) -> StatsEvent {
        let clients = futures::executor::block_on(self.clients.read());
        let client_ids: Vec<String> = clients.keys().cloned().collect();
        StatsEvent {
            client_count: client_ids.len(),
            conn_count: self.subscriber_count(),
            clients: client_ids,
        }
    }
}

pub struct ChannelManager {
    channels: HashMap<String, Arc<Channel>>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn get_or_create_channel(&mut self, name: &str) -> Arc<Channel> {
        self.channels
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Channel::new(name.to_string())))
            .clone()
    }

    pub async fn garbage_collect(&mut self) {
        let mut to_remove = Vec::new();

        for (name, channel) in &self.channels {
            if channel.subscriber_count() == 0 {
                let name_clone = name.clone();
                let channel_clone = channel.clone();

                // Wait a bit before actually removing
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(GC_WAIT_MS)).await;
                    if channel_clone.subscriber_count() == 0 {
                        info!("Channel {} eligible for cleanup", name_clone);
                    }
                });

                to_remove.push(name.clone());
            }
        }

        for name in to_remove {
            if let Some(channel) = self.channels.get(&name) {
                if channel.subscriber_count() == 0 {
                    info!("Removing channel: {}", name);
                    self.channels.remove(&name);
                }
            }
        }
    }
}
