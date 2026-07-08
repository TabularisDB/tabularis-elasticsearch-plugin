use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Deserialize)]
pub struct Index {
    pub index: String,
    pub uuid: String,
    pub health: String,
    pub status: String,
    pub pri: String,
    pub rep: String,
    #[serde(rename = "docs.count")]
    pub docs_count: String,
    #[serde(rename = "docs.deleted")]
    pub docs_deleted: String,
    #[serde(rename = "store.size")]
    pub store_size: String,
    #[serde(rename = "pri.store.size")]
    pub pri_store_size: String,
    #[serde(rename = "dataset.size")]
    pub dataset_size: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Column {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Field {
    pub field: String,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SqlResponse {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EsqlResponse {
    pub took: usize,
    pub is_partial: bool,
    pub documents_found: usize,
    pub values_loaded: usize,
    pub columns: Vec<Column>,
    pub values: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Shards {
    pub total: i64,
    pub successful: i64,
    pub skipped: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hit {
    #[serde(rename = "_index")]
    pub index: String,
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_score")]
    pub score: Option<f64>,
    #[serde(rename = "_source")]
    pub source: Option<Map<String, Value>>,
    pub fields: Option<Map<String, Value>>,
    pub sort: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hits {
    pub max_score: Option<f64>,
    pub hits: Vec<Hit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub took: usize,
    pub timed_out: bool,

    #[serde(rename = "_shards")]
    pub shards: Shards,
    pub hits: Hits,
}
