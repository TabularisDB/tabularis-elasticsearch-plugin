use serde_json::Value;

/// Extracts the URL from the params object
pub fn extract_url(params: &Value) -> Option<String> {
    params
        .get("params")
        .and_then(|p| p.get("database")) // Because we use database field for storing the URL
        .and_then(|d| d.as_str())
        .map(|s| s.to_string())
}

/// Extracts the table from the params object
pub fn extract_table(params: &Value) -> Option<String> {
    params
        .get("table")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

/// Extracts the "query" from the params object
pub fn extract_query(params: &Value) -> Option<String> {
    params
        .get("query")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string())
}
