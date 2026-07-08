use crate::es::models::{EsqlResponse, SearchResponse, SqlResponse};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum QueryMode {
    None,
    Sql,
    Rest,
    Esql,
}

#[derive(Debug, Clone)]
pub struct Query {
    pub mode: QueryMode,
    pub body: String,
}

impl From<String> for Query {
    fn from(query: String) -> Self {
        let query = query.trim_start();

        let (header, body) = query.split_once('\n').unwrap_or((query, ""));

        let (mode, query) = match header.trim() {
            "#!sql" => (QueryMode::Sql, body.trim_start()),
            "#!rest" => (QueryMode::Rest, body.trim_start()),
            "#!esql" => (QueryMode::Esql, body.trim_start()),
            _ => (QueryMode::None, query),
        };

        Self {
            mode,
            body: query.to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExecuteQueryResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub affected_rows: usize,
    pub execution_time_ms: usize,
}

impl From<SqlResponse> for ExecuteQueryResponse {
    fn from(t: SqlResponse) -> Self {
        let affected_rows = t.rows.len();

        Self {
            columns: t.columns.iter().map(|c| c.name.clone()).collect(),
            rows: t.rows,
            affected_rows,
            execution_time_ms: 0,
        }
    }
}

impl From<EsqlResponse> for ExecuteQueryResponse {
    fn from(t: EsqlResponse) -> Self {
        Self {
            columns: t.columns.iter().map(|c| c.name.clone()).collect(),
            rows: t.values,
            affected_rows: t.documents_found,
            execution_time_ms: t.took,
        }
    }
}

impl From<SearchResponse> for ExecuteQueryResponse {
    fn from(resp: SearchResponse) -> Self {
        let hits = resp.hits.hits;

        let columns: Vec<String> = hits
            .first()
            .map(|hit| {
                hit.source
                    .as_ref()
                    .or(hit.fields.as_ref())
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let rows = hits
            .iter()
            .map(|hit| {
                let data = hit.source.as_ref().or(hit.fields.as_ref());

                columns
                    .iter()
                    .map(|col| {
                        data.and_then(|m| m.get(col))
                            .map(|v| v.to_owned())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();

        Self {
            affected_rows: hits.len(),
            execution_time_ms: resp.took,
            columns,
            rows,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestCase<'a> {
        name: &'a str,
        input: &'a str,
        exp_mode: QueryMode,
        exp_query: &'a str,
    }

    #[test]
    fn parse_query() {
        let cases = [
            TestCase {
                name: "sql",
                input: "#!sql\nSELECT * FROM users",
                exp_mode: QueryMode::Sql,
                exp_query: "SELECT * FROM users",
            },
            TestCase {
                name: "rest",
                input: "#!rest\nGET /users/_search",
                exp_mode: QueryMode::Rest,
                exp_query: "GET /users/_search",
            },
            TestCase {
                name: "esql",
                input: "#!esql\nFROM users",
                exp_mode: QueryMode::Esql,
                exp_query: "FROM users",
            },
            TestCase {
                name: "default",
                input: "SELECT * FROM users",
                exp_mode: QueryMode::None,
                exp_query: "SELECT * FROM users",
            },
        ];

        for case in cases {
            let parsed = Query::from(case.input.to_owned());

            assert_eq!(parsed.mode, case.exp_mode, "{}", case.name);
            assert_eq!(parsed.body, case.exp_query, "{}", case.name);
        }
    }
}
