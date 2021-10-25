use schemars::schema::RootSchema;
use serde::de::DeserializeOwned;
use std::io::{Error as IOError, ErrorKind};
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

use schemars::schema_for_value;

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

pub struct DebugServer<E, N> {
    pub sender: Sender<Message<E, N>>,
    pub receiver: Receiver<Message<E, N>>,
    _handle: JoinHandle<()>,
}

async fn accept_connection<E, N>(
    stream: TcpStream,
    to_hotham: Sender<Message<E, N>>,
    from_hotham: Receiver<Message<E, N>>,
) where
    E: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
    N: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
{
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    println!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("New WebSocket connection: {}", addr);

    let (to_client, from_client) = ws_stream.split();

    let client_to_hotham = from_client
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

    let from_hotham = BroadcastStream::new(from_hotham).map(|message| match message {
        Ok(message) => {
            let json = serde_json::to_string(&message)
                .expect(&format!("Unable to deserialize {:?}", message));
            Ok(tungstenite::Message::Text(json))
        }
        Err(e) => Err(tokio_tungstenite::tungstenite::Error::Io(IOError::new(
            ErrorKind::Other,
            e.to_string(),
        ))),
    });
    let hotham_to_client = from_hotham.forward(to_client);

    let (r1, r2) = join!(client_to_hotham, hotham_to_client);
    r1.expect("Problem!");
    r2.expect("Problem 2!");
}

impl<E, N> DebugServer<E, N>
where
    E: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug + Default,
    N: Serialize + DeserializeOwned + Send + Sync + Clone + Unpin + 'static + Debug,
{
    pub fn new() -> DebugServer<E, N>
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
                    tokio::spawn(accept_connection(stream, s1.clone(), s1.subscribe()));
                }
            })
        });

        DebugServer {
            sender,
            receiver,
            _handle: handle,
        }
    }

    pub fn sync(&mut self, non_editable_data: &N) -> Option<E> {
        let mut editable_data = None;
        let response: Option<Message<E, N>> = match self.receiver.try_recv() {
            Ok(Message::Data(Data {
                editable: Some(editible),
                ..
            })) => {
                editable_data = Some(editible);
                Some(Message::Data(Data {
                    editable: None,
                    non_editable: Some(non_editable_data.clone()),
                }))
            }
            Ok(Message::Command(Command::Reset)) => {
                editable_data = Some(E::default());
                Some(Message::Data(Data {
                    editable: None,
                    non_editable: Some(non_editable_data.clone()),
                }))
            }
            Ok(Message::Command(Command::Init)) => {
                let editable = E::default();
                let non_editable = non_editable_data.clone();

                let editable_schema = schema_for_value!(editable);
                let non_editable_schema = schema_for_value!(non_editable_data);
                Some(Message::Init(InitData {
                    schema: Schema {
                        editable: editable_schema,
                        non_editable: non_editable_schema,
                    },
                    data: Data {
                        editable: Some(editable),
                        non_editable: Some(non_editable),
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
            self.sender.send(response).expect("Unable to update value");
            let _ = self.receiver.try_recv();
        }

        editable_data
    }
}

#[cfg(test)]
#[allow(unused_assignments)]
mod tests {
    use std::time::Duration;
    #[derive(Deserialize, Serialize, Clone, Debug, Default)]
    struct Test {
        name: String,
    }

    #[derive(Deserialize, Serialize, Clone, Debug)]
    struct Info {
        count: usize,
    }

    use super::*;
    #[test]
    fn it_works() {
        // This is simulating the inside of Hotham.
        let mut server: DebugServer<Test, Info> = DebugServer::new();

        let mut data = Test {
            name: "Железного́рск".to_string(),
        };
        let mut info = Info { count: 0 };

        loop {
            std::thread::sleep(Duration::from_secs(1));
            let changed = server.sync(&info);
            if let Some(changed) = changed {
                data = changed;
                println!("Changed! Data is now {:?}", data);
            }
            info.count += 1;
        }
    }
}
