mod server;

use std::{collections::HashMap, sync::Arc, time::SystemTime};
use tokio::sync::Mutex;

use anyhow::Result;
/// A simple server that serves assets to localhost or remote targets. It's great and has no flaws.
// TODO:
// 1. Accept connections
// 2. Send a file on "GET"
// 3. Watch for file updates
// 4. Send a "file updated" message back to the client on update
// 5. GOTO 2
use server::{handle_connection, make_server_endpoint, watch_files};

pub type WatchList = Arc<Mutex<HashMap<String, anyhow::Result<SystemTime>>>>;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:5000".parse().unwrap();
    let (endpoint, _server_cert) = make_server_endpoint(addr).unwrap();

    loop {
        let incoming_conn = endpoint.accept().await.unwrap();
        let new_conn = incoming_conn.await.unwrap();
        let watch_list = WatchList::default();

        let watcher_watch_list = watch_list.clone();
        tokio::spawn(watch_files(new_conn.clone(), watcher_watch_list));
        tokio::spawn(handle_connection(new_conn, watch_list.clone()));
    }
}
