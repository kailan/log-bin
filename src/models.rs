use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct FieldData {
    pub value: String,
    pub color: String,
    pub contrast: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    pub time: i64,
    pub raw: String,
    pub fields: HashMap<String, FieldData>,
    pub parser: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsEvent {
    #[serde(rename = "clientCount")]
    pub client_count: usize,
    #[serde(rename = "connCount")]
    pub conn_count: usize,
    pub clients: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuspensionEvent {
    pub suspended: bool,
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
}
