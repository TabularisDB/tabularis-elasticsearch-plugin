use crate::error::PluginError;
use elasticsearch::http::transport::Transport;
use std::{
    collections::HashMap,
    sync::LazyLock,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

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
    let transport = Transport::single_node(url)?;

    // Write and return
    let mut pools = CONNECTION_POOLS.write().await;
    let cached = pools
        .entry(url.to_string())
        .or_insert_with(move || CachedTransport {
            transport: transport,
            last_used: Instant::now(),
        });

    Ok(cached.clone().transport)
}

pub async fn cleanup_pools() {
    let mut pools = CONNECTION_POOLS.write().await;

    pools.retain(|_, pool| pool.last_used.elapsed() < POOL_TTL);
}
