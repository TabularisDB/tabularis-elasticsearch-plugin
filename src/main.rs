//! Entry point: read JSON-RPC lines from stdin, dispatch, write responses.
//
use std::{
    io::{self, BufRead, Write},
    time::Duration,
};

use tokio::time::interval;

mod error;
mod es;
mod handlers;
mod models;
mod rpc;
mod utils;

#[tokio::main]
async fn main() {
    // Init
    tokio::spawn(async {
        let mut timer = interval(Duration::from_secs(600)); // 10 minutes
        loop {
            timer.tick().await;
            es::pool::cleanup_pools().await;
        }
    });

    // Read JSON-RPC lines from stdin, dispatch, write responses
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                eprintln!("stdin read error, exiting: {err}");
                break;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = rpc::handle_line(trimmed).await;
        let mut body = match serde_json::to_string(&response) {
            Ok(s) => s,
            Err(err) => format!(
                "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32603,\"message\":\"serialization failed: {err}\"}},\"id\":null}}",
            ),
        };
        body.push('\n');
        if out.write_all(body.as_bytes()).is_err() {
            break;
        }
        let _ = out.flush();
    }
}
