use schemars::schema::RootSchema;
use serde::de::DeserializeOwned;
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

use schemars::{schema_for, JsonSchema};

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
pub struct Test {
    name: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Info {
    count: usize,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Data<E, N> {
    editable: Option<E>,
    non_editable: Option<N>,
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Debug)]
#[repr(u8)]
pub enum Command {
    Reset,
    Init,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Schema {
    editable: RootSchema,
    non_editable: RootSchema,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct InitData<E, N> {
    data: Data<E, N>,
    schema: Schema,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum Message<E, N> {
    Data(Data<E, N>),
    Command(Command),
    Init(InitData<E, N>),
    Error(String),
}

pub fn create_server<E, N>() -> (
    Sender<Message<E, N>>,
    Receiver<Message<E, N>>,
    JoinHandle<()>,
)
where
    E: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
    N: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
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

async fn accept_connection<E, N>(
    stream: TcpStream,
    to_hotham: Sender<Message<E, N>>,
    from_hotham: Receiver<Message<E, N>>,
) where
    E: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
    N: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
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
                    match serde_json::from_str::<Message<E, N>>(&m) {
                        Ok(message) => {
                            println!("Deserialised: {:?}", message);
                            Some(Ok(message))
                        }
                        Err(e) => {
                            let error_message = format!("Error deserialising: {:?}", e);
                            eprintln!("{:?}", error_message);
                            Some(Ok(Message::Error(error_message)))
                        }
                    }
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

pub fn run_server<E, N>(
    editable_data: &mut E,
    non_editable_data: &N,
    to_server: &mut Sender<Message<E, N>>,
    from_server: &mut Receiver<Message<E, N>>,
) -> bool
where
    E: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug + Default,
    N: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
{
    let mut changed = false;
    let response = match from_server.try_recv() {
        Ok(Message::Data(Data {
            editable: Some(editible),
            ..
        })) => {
            *editable_data = editible;
            changed = true;
            Some(Message::Data(Data {
                editable: None,
                non_editable: Some(non_editable_data.clone()),
            }))
        }
        Ok(Message::Command(Command::Reset)) => {
            *editable_data = E::default();
            changed = true;
            Some(Message::Data(Data {
                editable: None,
                non_editable: Some(non_editable_data.clone()),
            }))
        }
        Ok(Message::Command(Command::Init)) => {
            let editable = schema_for!(Test);
            let non_editable = schema_for!(Info);
            Some(Message::Init(InitData {
                schema: Schema {
                    editable,
                    non_editable,
                },
                data: Data {
                    editable: Some(editable_data.clone()),
                    non_editable: Some(non_editable_data.clone()),
                },
            }))
        }
        Ok(error_message @ Message::Error(_)) => Some(error_message),
        Ok(_) => None,
        Err(_) => Some(Message::Data(Data {
            editable: None,
            non_editable: Some(non_editable_data.clone()),
        })),
    };

    if let Some(response) = response {
        to_server.send(response).expect("Unable to update value");
        let _ = from_server.try_recv();
    }

    changed
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    #[test]
    fn it_works() {
        // This is simulating the inside of Hotham.
        let (mut sender, mut receiver, _handle) = create_server();
        let mut data = Test {
            name: "Железного́рск".to_string(),
        };
        let mut info = Info { count: 0 };

        loop {
            std::thread::sleep(Duration::from_secs(1));
            let changed = run_server(&mut data, &info, &mut sender, &mut receiver);
            if changed {
                println!("Changed! Data is now {:?}", data);
            }
            info.count += 1;
        }
    }
}
