# Tabularis — Elasticsearch Plugin

A Tabularis driver plugin that lets Tabularis users inspect and query Elasticsearch clusters.

Current features

- Index browsing: list indices and view basic index stats
- Mapping viewer: inspect index mappings (fields, types, nested structures)
- Document samples: preview documents from an index (sampling/scroll)
- Query execution: run Elasticsearch queries and show results in the Tabularis UI

## Usage

The plugin supports four query modes. The mode is determined by an optional shebang (`#!`) at the beginning of the query.

| Mode | Prefix | Description |
|------|--------|-------------|
| SQL (default) | _(none)_ or `#!sql` | Execute Elasticsearch SQL queries. |
| ES\|QL | `#!esql` | Execute ES\|QL queries. |
| REST | `#!rest` | Send raw Elasticsearch REST requests. |

For examples

```text
SELECT * FROM user_index_000000004;

-- Or explicitly:

#!sql
SELECT * FROM user_index_000000004;


#!esql
FROM user_index_000000004

#!rest
POST /post_index/_search
{"query":{"match_all":{}},"fields":[{"field":"id"},{"field":"content"}],"sort":[{"_doc":{"order":"asc"}}],"track_total_hits":-1,"_source":true}
```

The first line must contain the HTTP method and endpoint. The remaining content is sent as the request body.

## Installation

Build and install the plugin locally (developer workflow):

```bash
just dev-install    # builds and installs to $HOME/Library/Application Support/com.debba.tabularis/plugins/elasticsearch
```

Open Tabularis and choose the "Elasticsearch" driver in the connection picker. Configure a connection `http://<username>:<password>@<host>:<port>` to start exploring indices.

### Explain

Copy the binary and `manifest.json` into the Tabularis plugins folder under a
`elasticsearch/` subdirectory:

| OS | Path |
|----|------|
| Linux | `~/.local/share/tabularis/plugins/elasticsearch/` |
| macOS | `~/Library/Application Support/tabularis/plugins/elasticsearch/` |
| Windows | `%APPDATA%\debba\tabularis\data\plugins\elasticsearch\` |

Restart Tabularis (or install via Settings) and pick **Elasticsearch** in the
connection form.

## Build

```bash
cargo build --release
# binary: target/release/tabularis-elasticsearch-plugin
```

## Development

For setup test environment (run Elasticsearch and seed-data)
```bash
# Run Elasticsearch 
just run-es

# Seed data for Elasticsearch
just seed-es
```

Run the local REPL to test handlers without Tabularis:

```bash
just repl
# use REPL commands to exercise rpc handlers
```

Project layout (high level):

- src/main.rs       — stdio loop for plugin RPC transport
- src/rpc.rs        — method dispatch and helper responses
- src/es/*          — Elasticsearch client wrapper and connection config
- src/handlers/     — metadata, query, sample, and mapping handlers
- src/models.rs     — connection params and shared types
- bin/test_plugin.rs — REPL for exercising RPC handlers locally

## Maintainers

* @erwin-lovecraft 

## License

Apache-2.0.
