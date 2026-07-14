use crate::error::PluginError;
use elasticsearch::http::transport::{SingleNodeConnectionPool, Transport, TransportBuilder};
use std::{
    collections::HashMap,
    sync::LazyLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use url::Url;

// Pool TTL: To evict stale connections after 30 minutes
const POOL_TTL: Duration = Duration::from_secs(30 * 60);

// A global connection pool for cached Elasticsearch transports.
// Always accessed via a plain mutex: every access (hit or miss) needs to
// write `last_used`, so a read/write split bought nothing but complexity.
static CONNECTION_POOLS: LazyLock<Mutex<HashMap<String, CachedTransport>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct CachedTransport {
    transport: Transport,
    last_used: Instant,
}

pub async fn get_transport(url: &str) -> Result<Transport, PluginError> {
    // Held for the whole check-then-build-then-insert sequence so two
    // concurrent requests for a brand-new URL can't both build a Transport:
    // the second one just finds the first one's entry already cached.
    // TransportBuilder::build() does no network I/O (it only constructs the
    // client), so holding the lock across it doesn't block other requests
    // for long.
    let mut pools = CONNECTION_POOLS.lock().await;

    if let Some(cached) = pools.get_mut(url) {
        cached.last_used = Instant::now();
        return Ok(cached.transport.clone());
    }

    let u = Url::parse(url).map_err(|_| PluginError::invalid_params("invalid url"))?;

    let conn_pool = SingleNodeConnectionPool::new(u);

    // Cannot use Transport::single_node(url) because
    // we want to support URL with special characters
    // e.g. http://user:pass@123@localhost:9200
    let transport = TransportBuilder::new(conn_pool)
        .build()
        .map_err(|_| PluginError::internal("failed to create transport"))?;

    let cached = pools
        .entry(url.to_string())
        .or_insert_with(|| CachedTransport {
            transport,
            last_used: Instant::now(),
        });

    Ok(cached.transport.clone())
}

pub async fn cleanup_pools() {
    let mut pools = CONNECTION_POOLS.lock().await;

    pools.retain(|_, pool| pool.last_used.elapsed() < POOL_TTL);
}
