use chrono::{DateTime, Utc};

/// Format a timestamp as ISO 8601 UTC string.
pub fn iso_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Parse an ISO 8601 timestamp string.
pub fn parse_iso_timestamp(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
}
