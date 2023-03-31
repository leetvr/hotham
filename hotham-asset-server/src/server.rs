use anyhow::{bail, Result};
use futures_util::TryFutureExt;
use hotham_asset_client::message::Message;
/// A simple server that serves assets to localhost or remote targets. It's great and has no flaws.
// TODO:
// 1. Accept connections
// 2. Send a file on "GET"
// 3. Watch for file updates
// 4. Send a "file updated" message back to the client on update
// 5. GOTO 2
use quinn::{Endpoint, ServerConfig};
use std::{
    error::Error,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::WatchList;

pub async fn handle_connection(conn: quinn::Connection, watch_list: WatchList) -> Result<()> {
    println!("[SERVER] Connection established!");
    loop {
        let (send, recv) = match conn.accept_bi().await {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                println!("[SERVER] Connection closed");
                return Ok(());
            }
            Err(e) => {
                bail!("Error: {}", e)
            }
            Ok(s) => s,
        };
        tokio::spawn(
            handle_incoming(recv, send, watch_list.clone())
                .map_err(|e| eprintln!("[SERVER] Error in incoming: {e:?}")),
        );
    }
}

pub async fn watch_files(connection: quinn::Connection, watch_list: WatchList) {
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    loop {
        interval.tick().await;
        let mut asset_names = watch_list.lock().await;
        if asset_names.is_empty() {
            continue;
        }

        // This could definitely be more optimised, but it's not really important.
        for (name, last_updated) in asset_names.iter_mut() {
            let current_updated = get_last_updated(name).await;

            // Figure out if we should send an updated version or not
            let should_send_update = match (&last_updated, &current_updated) {
                (Ok(_), Err(e)) => {
                    println!("[SERVER] Failed to get timestamp for {name}: {e}");
                    false
                }
                (Ok(last), Ok(current)) => current != last,
                (Err(_), Ok(_)) => true,
                (Err(_), Err(_)) => false,
            };

            // Update the last_updated in the hashmap
            *last_updated = current_updated;

            if should_send_update {
                // If we got here, the file has been updated. Open a connection to the client and tell them the file has been updated.
                let connection = connection.clone();
                let name = name.clone();
                tokio::spawn(async move {
                    let (mut send, mut recv) = connection.open_bi().await?;
                    let mut buffer = vec![0; 1024 * 64];
                    println!("[SERVER] {} updated! Sending message..", name.clone());
                    Message::AssetUpdated(&name).write_all(&mut send).await?;

                    match Message::read(&mut recv, &mut buffer).await? {
                        Message::OK => Ok(()),
                        Message::Error(e) => {
                            anyhow::bail!("[SERVER] Got an error: {e}");
                        }
                        m => anyhow::bail!("[SERVER] Invalid message: {m:?}"),
                    }
                });
            }
        }
    }
}

async fn handle_incoming(
    mut recv: quinn::RecvStream,
    mut send: quinn::SendStream,
    watch_list: WatchList,
) -> Result<()> {
    let mut buffer = vec![0; 1024 * 64];
    loop {
        if let Some(response) = handle_request(&mut recv, &mut buffer, watch_list.clone()).await? {
            println!("[SERVER] Sending response: {:?}", response.get_type());
            response.write_all(&mut send).await?;
        }
    }
}

async fn handle_request<'a>(
    recv: &'a mut quinn::RecvStream,
    buffer: &'a mut [u8],
    watch_list: WatchList,
) -> Result<Option<Message<'a>>> {
    let message = Message::read(recv, buffer).await?;

    println!("[SERVER] Received a message: {message:?}");
    let response = match message {
        Message::GetAsset(path) => {
            let message = match get_asset(path).await {
                Ok(asset_bytes) => Message::Asset(asset_bytes),
                Err(e) => Message::Error(e.to_string()),
            };
            Some(message)
        }
        Message::WatchAsset(path) => {
            println!("[SERVER] Watching an asset: {path}");
            watch_list
                .lock()
                .await
                .insert(path.into(), Ok(SystemTime::UNIX_EPOCH));
            Some(Message::OK)
        }
        Message::OK => {
            println!("[SERVER] OK :-)");
            None
        }
        Message::Error(e) => {
            anyhow::bail!("[SERVER] Got an error: {e}");
        }
        _ => None,
    };

    Ok(response)
}

async fn get_asset(path: &str) -> Result<Vec<u8>> {
    println!("[SERVER] Requested an asset: {path}. Reading from disk..");
    let bytes = tokio::fs::read(path).await?;
    println!("[SERVER] Done. Read {} bytes", bytes.len());
    Ok(bytes)
}

async fn get_last_updated(path: &str) -> anyhow::Result<SystemTime> {
    Ok(tokio::fs::metadata(path).await?.modified()?)
}

pub fn make_server_endpoint(bind_addr: SocketAddr) -> Result<(Endpoint, Vec<u8>), Box<dyn Error>> {
    let (server_config, server_cert) = configure_server()?;
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok((endpoint, server_cert))
}

/// Returns default server configuration along with its certificate.
#[allow(clippy::field_reassign_with_default)] // https://github.com/rust-lang/rust-clippy/issues/6527
fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["hotham_asset_server".into()]).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    let priv_key = rustls::PrivateKey(priv_key);
    let cert_chain = vec![rustls::Certificate(cert_der.clone())];

    let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .keep_alive_interval(Some(Duration::from_secs(5)));

    Ok((server_config, cert_der))
}
