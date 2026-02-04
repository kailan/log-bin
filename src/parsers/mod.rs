mod color_utils;

use crate::models::FieldData;
use color_utils::{color_for_string, contrast_ratio};
use serde_json::Value;
use std::collections::HashMap;

pub struct ParsedEvent {
    pub input_string: String,
    pub parser: Option<String>,
    pub fields: HashMap<String, FieldData>,
    pub time: i64,
}

impl ParsedEvent {
    pub fn new(input_string: String) -> Self {
        Self {
            input_string,
            parser: None,
            fields: HashMap::new(),
            time: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn parse(&mut self) {
        // Try JSON parser first
        if let Some(data) = parse_json(&self.input_string) {
            self.parser = Some("json".to_string());
            self.fields = create_fields(data);
            return;
        }

        // Try HTTP Structured Headers parser
        // Note: This is a simplified version. For full HTTP-SH support,
        // you'd need to implement or use a proper parser crate
        if let Some(data) = parse_structured_headers(&self.input_string) {
            self.parser = Some("structuredHeaders".to_string());
            self.fields = create_fields(data);
            return;
        }

        // No parser matched
        self.parser = None;
        self.fields = HashMap::new();
    }
}

fn parse_json(input: &str) -> Option<HashMap<String, String>> {
    serde_json::from_str::<Value>(input).ok().and_then(|v| {
        if let Value::Object(map) = v {
            let mut result = HashMap::new();
            for (key, value) in map {
                let value_str = match value {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    _ => serde_json::to_string(&value).unwrap_or_default(),
                };
                result.insert(key, value_str);
            }
            Some(result)
        } else {
            None
        }
    })
}

fn parse_structured_headers(input: &str) -> Option<HashMap<String, String>> {
    // Try parsing as a Dictionary (most common for structured logs)
    if let Ok(dict) = sfv::Parser::new(input).parse::<sfv::Dictionary>() {
        let mut result = HashMap::new();
        for (key, member) in dict.iter() {
            match member {
                sfv::ListEntry::Item(item) => {
                    result.insert(key.to_string(), bare_item_to_string(&item.bare_item));
                    // Also extract parameters as separate fields
                    for (param_key, param_val) in item.params.iter() {
                        result.insert(param_key.to_string(), bare_item_to_string(param_val));
                    }
                }
                sfv::ListEntry::InnerList(inner) => {
                    let value = inner
                        .items
                        .iter()
                        .map(|i| bare_item_to_string(&i.bare_item))
                        .collect::<Vec<_>>()
                        .join(", ");
                    result.insert(key.to_string(), value);
                    // Also extract inner list parameters
                    for (param_key, param_val) in inner.params.iter() {
                        result.insert(param_key.to_string(), bare_item_to_string(param_val));
                    }
                }
            }
        }
        if !result.is_empty() {
            return Some(result);
        }
    }

    // Try parsing as a List (only if it has multiple items or items with parameters)
    if let Ok(list) = sfv::Parser::new(input).parse::<sfv::List>() {
        let has_structure = list.len() > 1
            || list.iter().any(|entry| match entry {
                sfv::ListEntry::Item(item) => !item.params.is_empty(),
                sfv::ListEntry::InnerList(_) => true,
            });

        if has_structure {
            let mut result = HashMap::new();
            for (idx, entry) in list.iter().enumerate() {
                let value = match entry {
                    sfv::ListEntry::Item(item) => bare_item_to_string(&item.bare_item),
                    sfv::ListEntry::InnerList(inner) => inner
                        .items
                        .iter()
                        .map(|i| bare_item_to_string(&i.bare_item))
                        .collect::<Vec<_>>()
                        .join(", "),
                };
                result.insert(format!("item{}", idx), value);
            }
            if !result.is_empty() {
                return Some(result);
            }
        }
    }

    // Try parsing as a single Item (only if it has parameters, otherwise it's just plain text)
    if let Ok(item) = sfv::Parser::new(input).parse::<sfv::Item>() {
        if !item.params.is_empty() {
            let mut result = HashMap::new();
            result.insert("value".to_string(), bare_item_to_string(&item.bare_item));
            // Include parameters as additional fields
            for (key, val) in item.params.iter() {
                result.insert(key.to_string(), bare_item_to_string(val));
            }
            return Some(result);
        }
    }

    // Fallback: legacy semicolon-separated format (not RFC-compliant but previously supported)
    parse_legacy_structured_headers(input)
}

fn parse_legacy_structured_headers(input: &str) -> Option<HashMap<String, String>> {
    // Handle semicolon-separated key=value pairs for backward compatibility
    if !input.contains('=') {
        return None;
    }

    let mut result = HashMap::new();

    // Split by semicolon
    for pair in input.split(';') {
        let pair = pair.trim();
        if let Some(eq_pos) = pair.find('=') {
            let key = pair[..eq_pos].trim().to_string();
            let value = pair[eq_pos + 1..].trim();

            // Remove quotes if present
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value[1..value.len() - 1].to_string()
            } else {
                value.to_string()
            };

            result.insert(key, value);
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn bare_item_to_string(item: &sfv::BareItem) -> String {
    match item {
        sfv::BareItem::Integer(i) => i.to_string(),
        sfv::BareItem::Decimal(d) => d.to_string(),
        sfv::BareItem::String(s) => s.to_string(),
        sfv::BareItem::Token(t) => t.to_string(),
        sfv::BareItem::ByteSequence(b) => base64_encode(b),
        sfv::BareItem::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
        sfv::BareItem::Date(d) => d.to_string(),
        sfv::BareItem::DisplayString(s) => s.to_string(),
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn create_fields(data: HashMap<String, String>) -> HashMap<String, FieldData> {
    data.into_iter()
        .map(|(key, value)| {
            let color = color_for_string(&key);
            let contrast = contrast_ratio(&color, "#000000");
            (
                key,
                FieldData {
                    value,
                    color,
                    contrast,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parser() {
        let input = r#"{"level":"info","message":"test","timestamp":1234567890}"#;
        let mut event = ParsedEvent::new(input.to_string());
        event.parse();

        assert_eq!(event.parser, Some("json".to_string()));
        assert!(event.fields.contains_key("level"));
        assert!(event.fields.contains_key("message"));
    }

    #[test]
    fn test_structured_headers_parser() {
        // RFC 8941 Dictionary format: comma-separated key=value pairs
        let input = r#"level=info, message="test message", timestamp=1234567890"#;
        let mut event = ParsedEvent::new(input.to_string());
        event.parse();

        assert_eq!(event.parser, Some("structuredHeaders".to_string()));
        assert!(event.fields.contains_key("level"));
        assert_eq!(event.fields.get("message").unwrap().value, "test message");
    }

    #[test]
    fn test_structured_headers_legacy_semicolons() {
        // Legacy semicolon-separated format (not RFC-compliant but supported)
        let input = "level=info; message=\"test message\"; timestamp=1234567890";
        let mut event = ParsedEvent::new(input.to_string());
        event.parse();

        assert_eq!(event.parser, Some("structuredHeaders".to_string()));
        assert!(event.fields.contains_key("level"));
        assert_eq!(event.fields.get("message").unwrap().value, "test message");
    }

    #[test]
    fn test_plain_text_not_parsed_as_structured() {
        // Plain text should not be parsed as structured data
        let input = "Hello!";
        let mut event = ParsedEvent::new(input.to_string());
        event.parse();

        assert_eq!(event.parser, None);
        assert!(event.fields.is_empty());
    }
}
