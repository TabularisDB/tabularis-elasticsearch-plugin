//! Connection and query execution.
//!
//! `test_connection` and `ping` return success unconditionally — this is
//! what lets the driver show up in the Tabularis connection picker right
//! after `just dev-install`. Replace with real checks before shipping.

use crate::error::PluginError;
use crate::es::client::Client;
use crate::handlers::models::{ExecuteQueryResponse, Query, QueryMode};
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
        Err(err) => error_response(id, err.code, &err.message),
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

    let query = match extractor::extract_query(params) {
        Some(payload) if !payload.is_empty() => Query::from(payload),
        _ => {
            return error_response(
                id,
                ErrorCode::InvalidParams,
                "query must be a non-empty string",
            )
        }
    };

    let client = match Client::from_url(&url).await {
        Ok(client) => client,
        Err(err) => {
            return error_response(id, err.code, &err.message);
        }
    };

    match QueryExecutor::new(query).execute(client).await {
        Ok(result) => ok_response(id, json!(result)),
        Err(err) => error_response(id, err.code, &err.message),
    }
}

struct QueryExecutor {
    query: Query,
}

impl QueryExecutor {
    fn new(query: Query) -> Self {
        Self{ query }
    }

    async fn execute(&self, client: Client) -> Result<ExecuteQueryResponse, PluginError> {
        match self.query.mode {
            QueryMode::Rest => {
                let resp = client.search(self.query.clone()).await;
                resp.map(|rs| ExecuteQueryResponse::from(rs))
            },
            QueryMode::Esql => {
                let resp = client.execute_esql(self.query.clone()).await;
                resp.map(|rs| ExecuteQueryResponse::from(rs))
            },
            QueryMode::None | QueryMode::Sql => {
                let resp = client.execute_sql(self.query.clone()).await;
                resp.map(|rs| ExecuteQueryResponse::from(rs))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn ping_success() {
        let params = json!({
            "params": {
                "database": "http://elastic:secret@123@localhost:9200"
            },
            "query": ""
        });

        let result = ping(json!(1), &params).await;

        assert_eq!(result["id"], 1);
    }
}
