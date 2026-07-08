use super::{models, pool};
use crate::error::PluginError;
use elasticsearch::http::headers::HeaderMap;
use elasticsearch::http::Method;
use elasticsearch::{
    cat::CatIndicesParts, http::StatusCode, indices::IndicesGetMappingParts, Elasticsearch,
};
use serde_json::{json, Value};
use url::Url;
use crate::es::models::{EsqlResponse, SearchResponse, SqlResponse};
use crate::handlers::models::Query;

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

    pub async fn ping(&self) -> Result<(), PluginError> {
        let response = self.es.ping().send().await?;

        if response.status_code() != StatusCode::OK {
            return Err(PluginError::internal(
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(())
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

    pub async fn execute_sql(self, query: Query) -> Result<SqlResponse, PluginError> {
        let resp = self
            .es
            .sql()
            .query()
            .body(json!({"query": query.body}))
            .send()
            .await?;
        if resp.status_code() != StatusCode::OK {
            return Err(PluginError::internal(resp.text().await.unwrap_or_default()));
        }

        Ok(resp.json().await?)
    }

    pub async fn execute_esql(&self, query: Query) -> Result<EsqlResponse, PluginError> {
        let resp = self
            .es
            .esql()
            .query()
            .body(json!({"query": query.body}))
            .send()
            .await?;
        if resp.status_code() != StatusCode::OK {
            return Err(PluginError::internal(resp.text().await.unwrap_or_default()));
        }

        Ok(resp.json().await?)
    }

    /// Execute query in format REST
    ///
    /// Query body example:
    ///
    /// GET /user_index/_search
    /// {
    ///   "size": 1000,
    ///   "query": {
    ///     "multi_match": {
    ///       "query": "%GU%",
    ///       "fields": [
    ///         "name^1.0"
    ///       ]
    ///     }
    ///   },
    ///   "track_total_hits": -1
    /// }
    pub async fn search(self, query: Query) -> Result<SearchResponse, PluginError> {
        let (header, body) = query.body.split_once('\n').unwrap_or((&query.body, ""));

        let (method, path) = header
            .split_once(char::is_whitespace)
            .map(|(m, p)| (m.trim(), p.trim()))
            .ok_or_else(|| PluginError::invalid_params("Invalid request line"))?;

        let method = match method.to_ascii_uppercase().as_str() {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            other => {
                return Err(PluginError::invalid_params(format!(
                    "Unknown HTTP method: {other}"
                )))
            }
        };

        let url = Url::parse(&format!("http://localhost{path}")) // Assume path already have /
            .map_err(|e| PluginError::invalid_params(format!("Invalid path: {e}")))?;

        let path = url.path();

        if !(path == "/_search" || path.ends_with("/_search")) {
            return Err(PluginError::invalid_params(format!(
                "Unsupported path: {path}. Only '/_search' and '/<index>/_search' are supported."
            )));
        }

        let query_params: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        let query_params = (!query_params.is_empty()).then_some(&query_params);

        let body = body.trim();
        let body = (!body.is_empty()).then(|| String::from(body));

        let response = self
            .es
            .send(method, path, HeaderMap::new(), query_params, body, None)
            .await?;

        if response.status_code() != StatusCode::OK {
            return Err(PluginError::internal(format!("Unexpected status code {} {}", response.status_code(), response.text().await.unwrap_or_default())));
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
