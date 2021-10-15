use serde::de::DeserializeOwned;
use std::marker::Unpin;
use std::thread::{self, JoinHandle};
use tokio::net::{TcpListener, TcpStream};

use futures::{join, TryFutureExt};
use futures_util::{future, stream, SinkExt, StreamExt, TryStreamExt};
use tokio::runtime::Builder;
use tokio::sync::broadcast::{channel as broadcast_channel, Receiver, Sender};

use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::tungstenite::{self, Message};

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct Test {
    count: usize,
}

pub enum Command {
    Reset,
}

pub fn create_server<T>() -> (Sender<T>, Receiver<T>, JoinHandle<()>)
where
    T: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
{
    let (sender, receiver) = broadcast_channel(16);
    let s1 = sender.clone();

    let handle = thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let addr = "127.0.0.1:8080".to_string();
            // Create the event loop and TCP listener we'll accept connections on.
            let try_socket = TcpListener::bind(&addr).await;
            let listener = try_socket.expect("Failed to bind");
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(accept_connection(
                    stream,
                    sender.clone(),
                    sender.subscribe(),
                ));
            }
        })
    });

    (s1.clone(), receiver, handle)
}

async fn accept_connection<T>(stream: TcpStream, to_client: Sender<T>, from_client: Receiver<T>)
where
    T: Serialize + DeserializeOwned + Send + Sync + std::clone::Clone + Unpin + 'static + Debug,
{
    let from_client = BroadcastStream::new(from_client).map(|v| {
        let json = serde_json::to_string(&v.unwrap()).expect("Unable to deserialize");
        Ok(Message::Text(json))
    });
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    println!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();

    let f1 = read
        .filter_map(|msg| async move {
            match msg {
                Ok(Message::Text(m)) => {
                    println!("Received message from client: {:?}", m);
                    let t: T = serde_json::from_str(&m).expect("Unable to deserialize!");
                    println!("Deserialised: {:?}", t);
                    Some(Ok(t))
                }
                _ => None,
            }
        })
        .try_for_each(|v| futures::future::ready(to_client.send(v).map(|_| ()).map_err(|_| ())));

    let f2 = from_client.forward(write);

    let (r1, r2) = join!(f1, f2);
    r1.expect("Problem!");
    r2.expect("Problem 2!");
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    #[test]
    fn it_works() {
        // This is simulating the inside of Hotham.
        let test = Test { count: 0 };
        let (sender, mut receiver, _handle) = create_server();
        sender.send(test).expect("Unable to send");
        let mut count = 0;

        loop {
            std::thread::sleep(Duration::from_secs(1));
            println!("About to receive..");
            count = match receiver.try_recv() {
                Ok(v) => v.count,
                Err(_) => count + 1,
            };
            let val = Test { count };
            println!("Count is now {}", count);
            sender.send(val).expect("Unable to update value");
            let _ = receiver.try_recv();
        }
    }
}
