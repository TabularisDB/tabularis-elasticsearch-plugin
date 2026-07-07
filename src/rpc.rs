//! JSON-RPC dispatch and response helpers.

use serde_json::{json, Value};

use crate::handlers;

/// Parse one JSON-RPC line and return the response value (serialised
/// downstream by `main.rs`). Never panics — parse errors and method
/// failures are surfaced as JSON-RPC error responses.
pub async fn handle_line(line: &str) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(err) => return error_response(Value::Null, -32700, &format!("parse error: {err}")),
    };

    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    match method.as_str() {
        "initialize" => handlers::init::initialize(id, &params).await,
        "ping" => handlers::query::ping(id, &params).await,
        "test_connection" => handlers::query::test_connection(id, &params).await,

        // // Metadata — return empty arrays so the driver loads cleanly.
        "get_tables" => handlers::metadata::get_tables(id, &params).await,
        "get_routines" => ok_response(id, json!([])),
        "get_views" => ok_response(id, json!([])),

        // // Query execution — critical but needs a real driver.
        "execute_query" => handlers::query::execute_query(id, &params).await,
        "explain_query" => handlers::query::explain_query(id, &params).await,

        // // CRUD.
        // "insert_record" => handlers::crud::insert_record(id, &params),
        // "update_record" => handlers::crud::update_record(id, &params),
        // "delete_record" => handlers::crud::delete_record(id, &params),

        // // DDL.
        // "get_create_table_sql" => handlers::ddl::get_create_table_sql(id, &params),
        // "get_add_column_sql" => handlers::ddl::get_add_column_sql(id, &params),
        // "get_alter_column_sql" => handlers::ddl::get_alter_column_sql(id, &params),
        // "get_create_index_sql" => handlers::ddl::get_create_index_sql(id, &params),
        // "get_create_foreign_key_sql" => handlers::ddl::get_create_foreign_key_sql(id, &params),
        // "drop_index" => handlers::ddl::drop_index(id, &params),
        // "drop_foreign_key" => handlers::ddl::drop_foreign_key(id, &params),
        other => not_implemented(id, other),
    }
}

pub fn ok_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "result": result,
        "id": id,
    })
}

pub fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "error": { "code": code, "message": message },
        "id": id,
    })
}

pub fn not_implemented(id: Value, method: &str) -> Value {
    error_response(
        id,
        -32601,
        &format!("method '{method}' is not implemented by this plugin yet"),
    )
}
