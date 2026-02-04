use crate::MAX_LOG_LINES_PER_MINUTE;
use crate::models::{LogEvent, SseEvent, StatsEvent, SuspensionEvent};
use futures_util::stream::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tracing::{info, warn};
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
    // Rate limiting fields
    suspended: AtomicBool,
    log_count_current_minute: AtomicU64,
    current_minute_timestamp: AtomicU64,
}

impl Channel {
    pub fn new(_name: String) -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            sender,
            history: Arc::new(RwLock::new(Vec::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            suspended: AtomicBool::new(false),
            log_count_current_minute: AtomicU64::new(0),
            current_minute_timestamp: AtomicU64::new(0),
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

    /// Check if bucket is suspended
    pub fn is_suspended(&self) -> bool {
        self.suspended.load(Ordering::Relaxed)
    }

    /// Record log entries and check rate limit. Returns true if logs were accepted, false if suspended.
    pub fn record_logs(&self, count: u64) -> bool {
        if self.suspended.load(Ordering::Relaxed) {
            return false;
        }

        let now_minutes = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 60;

        let stored_minute = self.current_minute_timestamp.load(Ordering::Relaxed);

        if now_minutes != stored_minute {
            // New minute, reset counter
            self.current_minute_timestamp
                .store(now_minutes, Ordering::Relaxed);
            self.log_count_current_minute
                .store(count, Ordering::Relaxed);
        } else {
            // Same minute, increment counter
            let new_count = self
                .log_count_current_minute
                .fetch_add(count, Ordering::Relaxed)
                + count;
            if new_count > MAX_LOG_LINES_PER_MINUTE {
                self.suspended.store(true, Ordering::Relaxed);
                warn!(
                    "Channel suspended due to rate limit exceeded: {} logs in current minute",
                    new_count
                );
                return false;
            }
        }

        true
    }

    pub async fn publish_suspension(&self, suspended: bool) {
        let event = SuspensionEvent { suspended };
        let data = serde_json::to_string(&event).unwrap();
        let sse_event = SseEvent {
            event_type: "suspension".to_string(),
            data,
        };
        let _ = self.sender.send(sse_event);
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

    pub fn get_channel(&self, name: &str) -> Option<Arc<Channel>> {
        self.channels.get(name).cloned()
    }

    pub async fn garbage_collect(&mut self) {
        let mut to_remove = Vec::new();

        for (name, channel) in &self.channels {
            // Only consider for removal if there are no subscribers and it's not suspended
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
