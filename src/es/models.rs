use serde::Deserialize;
use serde_json::Value;

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
    pub rows: Vec<Value>,
}
