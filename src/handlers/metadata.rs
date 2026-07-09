//! Schema metadata: databases, schemas, tables, columns, indexes, FKs,
//! views, routines. Each handler below returns a valid-but-empty response
//! so the plugin loads without errors. Replace the bodies one by one.
//!
//! Full reference: https://github.com/TabularisDB/tabularis/blob/main/plugins/PLUGIN_GUIDE.md#5-required-methods

use crate::{
    error::ErrorCode,
    es::{self},
    handlers::models::ColumnResponse,
    rpc::{error_response, ok_response},
    utils::extractor,
};
use serde_json::{
    json,
    Value::{self},
};

/// Returns the list of index in the elasticsearch.
pub async fn get_tables(id: Value, params: &Value) -> Value {
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

    let result = match client.get_indices().await {
        Ok(result) => result,
        Err(err) => {
            return error_response(id, err.code, &err.message);
        }
    };

    ok_response(
        id,
        json!(result
            .into_iter()
            .map(|i| {
                json!({
                    "name": i.index,
                    "schema": null,
                    "comment": null
                })
            })
            .collect::<Vec<_>>()),
    )
}

pub async fn get_columns(id: Value, params: &Value) -> Value {
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

    let table_name = match extractor::extract_table(params) {
        Some(tb) if !tb.is_empty() => tb,
        _ => {
            return error_response(
                id,
                ErrorCode::InvalidParams,
                "table must be a non-empty string",
            )
        }
    };

    let client = match es::client::Client::from_url(&url).await {
        Ok(client) => client,
        Err(err) => {
            return error_response(id, err.code, &err.message);
        }
    };

    // Send request
    let resp = match client.get_mapping(&table_name).await {
        Ok(resp) => resp,
        Err(err) => {
            return error_response(id, ErrorCode::InternalError, &err.message);
        }
    };

    let result = resp.get(&table_name).and_then(|idx| {
        let colums: Vec<ColumnResponse> = idx
            .mappings
            .properties
            .iter()
            .map(|(name, props)| ColumnResponse::from_values(name.to_string(), props.to_owned()))
            .collect();

        Some(colums)
    });

    ok_response(id, json!(result))
}
