//! Connection and query execution.
//!
//! `test_connection` and `ping` return success unconditionally — this is
//! what lets the driver show up in the Tabularis connection picker right
//! after `just dev-install`. Replace with real checks before shipping.

use std::time::Instant;

use crate::{
    error::ErrorCode,
    es,
    rpc::{error_response, ok_response},
    utils::extractor,
};
use serde_json::{json, Value};

pub async fn test_connection(id: Value, params: &Value) -> Value {
    let url = match extractor::extract_url(params) {
        Some(tb) if !tb.is_empty() => tb,
        _ => {
            return error_response(
                id,
                ErrorCode::InvalidParams,
                "url must be a non-empty string",
            )
        }
    };

    let client = match es::client::Client::from_url(&url).await {
        Ok(client) => client,
        Err(err) => {
            return error_response(id, err.code, &err.message);
        }
    };

    match client.ping().await {
        Ok(_) => ok_response(id, json!({"success": true})),
        Err(err) => error_response(id, ErrorCode::InternalError, &err.to_string()),
    }
}

pub async fn ping(id: Value, params: &Value) -> Value {
    test_connection(id, params).await
}

pub async fn execute_query(id: Value, params: &Value) -> Value {
    let url = match extractor::extract_url(params) {
        Some(tb) if !tb.is_empty() => tb,
        _ => {
            return error_response(
                id,
                ErrorCode::InvalidParams,
                "url must be a non-empty string",
            )
        }
    };

    let query = match extractor::extract_url(params) {
        Some(tb) if !tb.is_empty() => tb,
        _ => {
            return error_response(
                id,
                ErrorCode::InvalidParams,
                "query must be a non-empty string",
            )
        }
    };

    let client = match es::client::Client::from_url(&url).await {
        Ok(client) => client,
        Err(err) => {
            return error_response(id, err.code, &err.message);
        }
    };

    let start = Instant::now();

    let result = match client.execute_sql(&query).await {
        Ok(result) => result,
        Err(err) => {
            return error_response(id, err.code, &err.message);
        }
    };

    let elapsed = start.elapsed();

    ok_response(
        id,
        json!({
            "columns": result.columns.into_iter().map(|c| c.name).collect::<Vec<_>>(),
            "rows": result.rows,
            "affected_rows": result.rows.len(),
            "execution_time_ms": elapsed.as_millis(),
        }),
    )
}
