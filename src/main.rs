//! Entry point: read JSON-RPC lines from stdin, dispatch, write responses.
//!
//! Requests are processed concurrently by a fixed pool of worker tasks so a
//! slow query doesn't block unrelated requests. Response ordering doesn't
//! matter since Tabularis matches responses back to requests via `id`.
use std::{sync::Arc, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::{mpsc, watch, Mutex},
    time::interval,
};

mod error;
mod es;
mod handlers;
mod models;
mod rpc;
mod utils;

// Plain constant for now. Once the plugin gets an `init` RPC call from the
// host app, this should come from runtime-updatable config instead.
const WORKER_POOL_SIZE: usize = 4;

// Bounded so a burst of requests applies backpressure to the stdin reader
// instead of buffering unboundedly in memory.
const REQUEST_QUEUE_CAPACITY: usize = 64;

const POOL_CLEANUP_INTERVAL: Duration = Duration::from_secs(600); // 10 minutes

#[tokio::main]
async fn main() {
    // Single shutdown signal shared by every long-running task. The pool
    // cleanup task has no other way to know when to stop, so it reacts to
    // this directly. The reader/worker/writer pipeline instead shuts down
    // by closing channels in sequence (see below) so that anything already
    // read from stdin is guaranteed to be processed and written back before
    // the process exits.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let cleanup_handle = tokio::spawn(run_pool_cleanup(shutdown_rx));

    let (req_tx, req_rx) = mpsc::channel::<String>(REQUEST_QUEUE_CAPACITY);
    let req_rx = Arc::new(Mutex::new(req_rx));

    let (resp_tx, resp_rx) = mpsc::unbounded_channel::<String>();
    let writer_handle = tokio::spawn(run_writer(resp_rx));

    let worker_handles: Vec<_> = (0..WORKER_POOL_SIZE)
        .map(|_| tokio::spawn(run_worker(req_rx.clone(), resp_tx.clone())))
        .collect();
    // Drop main's copy so the writer's channel closes once every worker has
    // finished draining its queue and dropped its own copy.
    drop(resp_tx);

    run_reader(req_tx).await;

    // Stdin closed (or errored): stop the cleanup task and let the workers
    // drain whatever is still queued before they see the request channel
    // close and exit.
    let _ = shutdown_tx.send(true);

    for handle in worker_handles {
        let _ = handle.await;
    }
    let _ = writer_handle.await;
    let _ = cleanup_handle.await;
}

async fn run_pool_cleanup(mut shutdown_rx: watch::Receiver<bool>) {
    let mut timer = interval(POOL_CLEANUP_INTERVAL);
    loop {
        tokio::select! {
            _ = timer.tick() => es::pool::cleanup_pools().await,
            _ = shutdown_rx.changed() => break,
        }
    }
}

async fn run_reader(req_tx: mpsc::Sender<String>) {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(err) => {
                eprintln!("stdin read error, exiting: {err}");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Blocks when the queue is full, applying backpressure to reading.
        if req_tx.send(trimmed.to_string()).await.is_err() {
            break;
        }
    }
}

async fn run_worker(
    req_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    resp_tx: mpsc::UnboundedSender<String>,
) {
    loop {
        let line = {
            let mut rx = req_rx.lock().await;
            rx.recv().await
        };
        let Some(line) = line else { break };

        let response = rpc::handle_line(&line).await;
        let body = match serde_json::to_string(&response) {
            Ok(s) => s,
            Err(err) => format!(
                "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32603,\"message\":\"serialization failed: {err}\"}},\"id\":null}}",
            ),
        };

        if resp_tx.send(body).is_err() {
            break;
        }
    }
}

async fn run_writer(mut resp_rx: mpsc::UnboundedReceiver<String>) {
    let mut stdout = tokio::io::stdout();
    while let Some(mut body) = resp_rx.recv().await {
        body.push('\n');
        if stdout.write_all(body.as_bytes()).await.is_err() {
            break;
        }
        let _ = stdout.flush().await;
    }
}
