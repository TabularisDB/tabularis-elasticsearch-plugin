use crate::error::PluginError;
use elasticsearch::http::transport::{SingleNodeConnectionPool, Transport, TransportBuilder};
use std::{
    collections::HashMap,
    sync::LazyLock,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use url::Url;

// Pool TTL: To evict stale connections after 30 minutes
const POOL_TTL: Duration = Duration::from_secs(30 * 60);

// A global connection pool for cached Elasticsearch transports.
static CONNECTION_POOLS: LazyLock<RwLock<HashMap<String, CachedTransport>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct CachedTransport {
    transport: Transport,
    last_used: std::time::Instant,
}

pub async fn get_transport(url: &str) -> Result<Transport, PluginError> {
    // Read and returns if already cached
    {
        let pools = CONNECTION_POOLS.read().await;
        if let Some(cached) = pools.get(url) {
            return Ok(cached.clone().transport);
        }
    }

    // Create new transport
    let u = Url::parse(url).map_err(|_| PluginError::invalid_params("invalid url"))?;

    let conn_pool = SingleNodeConnectionPool::new(u);

    // Cannot use Transport::single_node(url) because
    // we want to support URL with special characters
    // e.g. http://user:pass@123@localhost:9200
    let transport = TransportBuilder::new(conn_pool)
        .build()
        .map_err(|_| PluginError::internal("failed to create transport"))?;

    // Write and return
    let mut pools = CONNECTION_POOLS.write().await;
    let cached = pools
        .entry(url.to_string())
        .or_insert_with(move || CachedTransport {
            transport,
            last_used: Instant::now(),
        });

    Ok(cached.clone().transport)
}

pub async fn cleanup_pools() {
    let mut pools = CONNECTION_POOLS.write().await;

    pools.retain(|_, pool| pool.last_used.elapsed() < POOL_TTL);
}
