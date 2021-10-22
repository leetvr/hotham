use serde::de::DeserializeOwned;
use std::marker::Unpin;
use std::thread::{self, JoinHandle};
use tokio::net::{TcpListener, TcpStream};

use futures::join;
use futures_util::{StreamExt, TryStreamExt};
use tokio::runtime::Builder;
use tokio::sync::broadcast::{channel as broadcast_channel, Receiver, Sender};

use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::tungstenite;

use serde::{Deserialize, Serialize};
use serde_repr::*;
use std::fmt::Debug;

use schemars::JsonSchema;

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Test {
    count: usize,
}
#[derive(Serialize_repr, Deserialize_repr, Clone, Debug)]
#[repr(u8)]
pub enum Command {
    Reset,
    GetSchema,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum Message {
    Data(Test),
    Command(Command),
    Schema(String),
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

async fn accept_connection<T>(stream: TcpStream, to_hotham: Sender<T>, from_hotham: Receiver<T>)
where
    T: Serialize + DeserializeOwned + Send + Sync + std::clone::Clone + Unpin + 'static + Debug,
{
    let from_client = BroadcastStream::new(from_hotham).map(|message| {
        let message = message.unwrap();
        let json =
            serde_json::to_string(&message).expect(&format!("Unable to deserialize {:?}", message));
        Ok(tungstenite::Message::Text(json))
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
                Ok(tungstenite::Message::Text(m)) => {
                    println!("Received message from client: {:?}", m);
                    let t: T = serde_json::from_str(&m).expect("Unable to deserialize!");
                    println!("Deserialised: {:?}", t);
                    Some(Ok(t))
                }
                _ => None,
            }
        })
        .try_for_each(|v| futures::future::ready(to_hotham.send(v).map(|_| ()).map_err(|_| ())));

    let f2 = from_client.forward(write);

    let (r1, r2) = join!(f1, f2);
    r1.expect("Problem!");
    r2.expect("Problem 2!");
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use schemars::schema_for;

    use super::*;
    #[test]
    fn it_works() {
        // This is simulating the inside of Hotham.
        let test = Test { count: 0 };
        let test_message = Message::Data(test);
        let (sender, mut receiver, _handle) = create_server();
        sender.send(test_message).expect("Unable to send");
        let mut count = 0;

        loop {
            std::thread::sleep(Duration::from_secs(1));
            println!("About to receive..");
            let response = match receiver.try_recv() {
                Ok(Message::Data(d)) => {
                    count = d.count;
                    None
                }
                Ok(Message::Command(Command::Reset)) => {
                    count = 0;
                    None
                }
                Ok(Message::Command(Command::GetSchema)) => {
                    let schema = schema_for!(Test);
                    let schema = serde_json::to_string_pretty(&schema).unwrap();
                    Some(Message::Schema(schema))
                }
                Ok(Message::Schema(_)) => None,
                Err(_) => {
                    count = count + 1;
                    Some(Message::Data(Test { count }))
                }
            };
            println!("Count is now {}", count);

            if let Some(response) = response {
                sender.send(response).expect("Unable to update value");
                let _ = receiver.try_recv();
            }
        }
    }
}
