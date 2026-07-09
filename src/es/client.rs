use super::pool;
use crate::error::PluginError;
use crate::es::models::{EsqlResponse, Index, IndexMapping, SearchResponse, SqlResponse};
use crate::handlers::models::Query;
use elasticsearch::http::headers::HeaderMap;
use elasticsearch::http::request::JsonBody;
use elasticsearch::http::Method;
use elasticsearch::{
    cat::CatIndicesParts, http::StatusCode, indices::IndicesGetMappingParts, Elasticsearch,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use url::Url;

#[derive(Debug)]
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

    pub async fn get_indices(&self) -> Result<Vec<Index>, PluginError> {
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

    pub async fn get_mapping(
        &self,
        index: &str,
    ) -> Result<HashMap<String, IndexMapping>, PluginError> {
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

        let body = (!body.is_empty())
            .then_some(body)
            .map(|b| serde_json::from_str::<Value>(b))
            .transpose()
            .map_err(|err| PluginError::internal(err.to_string()))? // Option<Result<T, E>> -> Result<Option<T>, E>
            .map(JsonBody::from);

        let response = self
            .es
            .send(method, path, HeaderMap::new(), query_params, body, None)
            .await?;

        if response.status_code() != StatusCode::OK {
            return Err(PluginError::internal(format!(
                "Unexpected status code {} {}",
                response.status_code(),
                response.text().await.unwrap_or_default()
            )));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::models::QueryMode;

    #[derive(Debug)]
    struct TestCase {
        q: Query,
    }

    #[tokio::test]
    async fn get_mapping_success() {
        let client = Client::from_url("http://elastic:secret@123@localhost:9200").await;
        assert!(client.is_ok(), "{:?}", client);

        let index_name = "posts";
        let resp = client.unwrap().get_mapping(index_name).await;
        assert!(resp.is_ok(), "{:?}", resp);
    }

    #[tokio::test]
    async fn execute_sql_success() {
        let tcs = [TestCase {
            q: Query {
                mode: QueryMode::Sql,
                body: r"SELECT * FROM posts".to_string(),
            },
        }];

        for tc in tcs {
            let client = Client::from_url("http://elastic:secret@123@localhost:9200").await;
            assert!(client.is_ok(), "{:?}", client);

            let resp = client.unwrap().execute_sql(tc.q).await;
            assert!(resp.is_ok(), "{:?}", resp);
        }
    }

    #[tokio::test]
    async fn execute_esql_success() {
        let tcs = [
            TestCase {
                q: Query {
                    mode: QueryMode::Esql,
                    body: "FROM posts | WHERE author LIKE \"*Erwin*\" | KEEP author, content"
                        .to_string(),
                },
            },
            TestCase {
                q: Query {
                    mode: QueryMode::Esql,
                    body: "FROM posts \n| WHERE author LIKE \"*Erwin*\" \n| KEEP author, content"
                        .to_string(),
                },
            },
        ];

        for tc in tcs {
            let client = Client::from_url("http://elastic:secret@123@localhost:9200").await;
            assert!(client.is_ok(), "{:?}", client);

            let resp = client.unwrap().execute_esql(tc.q).await;
            assert!(resp.is_ok(), "{:?}", resp);
        }
    }

    #[tokio::test]
    async fn execute_search_success() {
        let tcs = [
            TestCase {
                q: Query {
                    mode: QueryMode::Rest,
                    body: "POST /posts/_search\n{\"_source\":[\"author\",\"content\"],\"query\":{\"wildcard\":{\"author\":{\"value\":\"*Erwin*\"}}}}"
                        .to_string(),
                },
            },
        ];

        for tc in tcs {
            let client = Client::from_url("http://elastic:secret@123@localhost:9200").await;
            assert!(client.is_ok(), "{:?}", client);

            let resp = client.unwrap().search(tc.q).await;
            assert!(resp.is_ok(), "{:?}", resp);
        }
    }
}
