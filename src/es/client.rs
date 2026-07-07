use super::{models, pool};
use crate::error::PluginError;
use elasticsearch::{
    cat::CatIndicesParts, http::StatusCode, indices::IndicesGetMappingParts, Elasticsearch,
};
use serde_json::{json, Value};

pub struct Client {
    es: Elasticsearch,
}

impl Client {
    pub async fn from_url(url: &str) -> Result<Self, PluginError> {
        let transport = pool::get_transport(url).await?;

        Ok(Self {
            es: Elasticsearch::new(transport.clone()),
        })
    }

    pub async fn get_indices(&self) -> Result<Vec<models::Index>, PluginError> {
        let response = self
            .es
            .cat()
            .indices(CatIndicesParts::None)
            .format("json")
            .s(&["index"])
            .send()
            .await?;

        if response.status_code() != StatusCode::OK {
            return Err(PluginError::internal(
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(response.json().await?)
    }

    pub async fn get_mapping(&self, index: &str) -> Result<Value, PluginError> {
        let response = self
            .es
            .indices()
            .get_mapping(IndicesGetMappingParts::Index(&[index]))
            .send()
            .await?;

        if response.status_code() != StatusCode::OK {
            return Err(PluginError::internal(
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(response.json().await?)
    }

    pub async fn execute_sql(&self, query: &str) -> Result<models::SqlResponse, PluginError> {
        let response = self
            .es
            .sql()
            .query()
            .body(json!({ "query": query }))
            .send()
            .await?;

        if response.status_code() != StatusCode::OK {
            return Err(PluginError::internal(
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(response.json().await?)
    }

    pub async fn translate_sql(&self, query: &str) -> Result<Value, PluginError> {
        let response = self
            .es
            .sql()
            .translate()
            .body(json!({ "query": query }))
            .send()
            .await?;

        if response.status_code() != StatusCode::OK {
            return Err(PluginError::internal(
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(response.json().await?)
    }
}
