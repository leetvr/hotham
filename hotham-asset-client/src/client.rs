use crate::message::Message;
use crate::AssetUpdatedMessage;
use anyhow::{anyhow, bail, Context, Result};
use futures_util::{StreamExt, TryStreamExt};
use quinn::{ClientConfig, Endpoint};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

const BUFFER_SIZE: usize = 104_857_600; // 100MB

pub async fn watch(asset_names: Vec<String>, sender: Sender<AssetUpdatedMessage>) -> Result<()> {
    run_client(asset_names, sender).await
}

async fn run_client(
    mut asset_names: Vec<String>,
    sender: Sender<AssetUpdatedMessage>,
) -> Result<()> {
    let server_addr: Option<&'static str> = option_env!("HOTHAM_ASSET_SERVER_ADDRESS");
    let server_addr = server_addr.ok_or_else(|| anyhow!("Can't connect to server - the HOTHAM_ASSET_SERVER_ADDRESS environment variable was not set at compile time"))?.parse()?;
    let client_cfg = configure_client();
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;
    endpoint.set_default_client_config(client_cfg);

    // connect to server
    let quinn::NewConnection {
        connection,
        bi_streams,
        ..
    } = endpoint
        .connect(server_addr, "hotham_asset_server")?
        .await?;
    println!("[CLIENT] connected: addr={}", connection.remote_address());

    asset_names.push("../../../hotham/hotham/src/shaders/pbr.frag.spv".into());
    asset_names.push("../../../hotham/hotham/src/shaders/pbr.vert.spv".into());

    let _ = asset_names
        .drain(..)
        .map(|file| ask_for_watch(connection.clone(), file))
        .collect::<futures_util::stream::FuturesUnordered<_>>()
        .try_collect::<Vec<_>>()
        .await?;

    wait_for_updates(bi_streams, connection.clone(), sender)
        .await
        .context("Watching file")?;

    println!("[CLIENT] Done, closing connection..");
    connection.close(0u32.into(), b"done");
    endpoint.wait_idle().await;
    println!("[CLIENT] Closed. Goodbye!");

    Ok(())
}

async fn wait_for_updates(
    mut bi_streams: quinn::IncomingBiStreams,
    connection: quinn::Connection,
    sender: Sender<AssetUpdatedMessage>,
) -> Result<()> {
    while let Some(stream) = bi_streams.next().await {
        let stream = match stream {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                println!("[CLIENT] Connection closed");
                return Ok(());
            }
            Err(e) => {
                bail!(e);
            }
            Ok(s) => s,
        };
        tokio::spawn(handle_incoming(stream, connection.clone(), sender.clone()));
    }

    Ok(())
}

async fn ask_for_watch(connection: quinn::Connection, asset_name: String) -> Result<()> {
    let (mut send, mut recv) = connection.open_bi().await?;

    println!("[CLIENT] Sending watch request for {asset_name}..",);
    Message::WatchAsset(&asset_name)
        .write_all(&mut send)
        .await?;
    println!("[CLIENT] Done! Waiting for OK..");
    let mut buffer = vec![0; BUFFER_SIZE];

    match Message::read(&mut recv, &mut buffer).await? {
        Message::OK => {
            println!("[CLIENT] OK received! {asset_name} is now being watched");
        }
        Message::Error(e) => {
            bail!("[CLIENT] Received error watching {asset_name} - {e}");
        }
        invalid => bail!("[CLIENT] Invalid message received! {invalid:?}"),
    };

    Ok(())
}

async fn handle_incoming(
    (mut send, mut recv): (quinn::SendStream, quinn::RecvStream),
    connection: quinn::Connection,
    sender: Sender<AssetUpdatedMessage>,
) -> Result<()> {
    println!("[CLIENT] Incoming connection! Reading..");
    let mut buffer = vec![0; BUFFER_SIZE];
    let asset_name = match Message::read(&mut recv, &mut buffer).await? {
        Message::AssetUpdated(asset_name) => {
            println!("[CLIENT] Asset updated! {asset_name}");
            Message::OK.write_all(&mut send).await?;
            asset_name
        }
        invalid => anyhow::bail!("[CLIENT] Received invalid response: {invalid:?}"),
    };
    let asset_name = asset_name.to_string();

    println!("[CLIENT] Opening new stream");
    let (mut send, mut recv) = connection.open_bi().await?;
    println!("[CLIENT] Done! Fetching {asset_name}..");

    Message::GetAsset(&asset_name).write_all(&mut send).await?;
    let asset_data = match Message::read(&mut recv, &mut buffer).await? {
        Message::Asset(buf) => {
            println!("[CLIENT] Received asset: {}", buf.len());
            Message::OK.write_all(&mut send).await?;
            buf
        }
        invalid => anyhow::bail!("Received invalid response: {invalid:?}"),
    };

    sender
        .send(AssetUpdatedMessage {
            asset_id: asset_name,
            asset_data: Arc::new(asset_data),
        })
        .await?;

    Ok(())
}

fn configure_client() -> ClientConfig {
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    ClientConfig::new(Arc::new(crypto))
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

/// Dummy certificate verifier that treats any certificate as valid.
/// NOTE, such verification is vulnerable to MITM attacks, but convenient for testing.
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<(), Box<dyn std::error::Error>> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
        let files: Vec<String> = vec![
            "Many",
            "Different",
            "Files",
            "That",
            "Could",
            "Be",
            "Updated",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let expected_updates = files.len() * 2;

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(watch(files, sender))
                .unwrap();
        });

        // Wait for each asset to be updated twice.
        let mut updates = 0;
        while let Some(message) = receiver.blocking_recv() {
            assert_eq!(*message.asset_data, b"DAHALLHASAWALL22");
            updates += 1;

            if updates == expected_updates {
                return Ok(());
            }
        }

        panic!("Failed!");
    }
}
