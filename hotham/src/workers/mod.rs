use hotham_asset_client::{watch, AssetUpdatedMessage};

use std::sync::mpsc;

#[derive(Debug, Clone)]
pub(crate) enum WorkerMessage {
    AssetUpdated(AssetUpdatedMessage),
    Error(WorkerError),
}

#[derive(Debug, Clone)]
pub(crate) enum WorkerError {
    #[allow(unused)]
    TaskFailed(String),
}

pub(crate) struct Workers {
    pub(crate) receiver: mpsc::Receiver<WorkerMessage>,
}

impl Workers {
    pub fn new(asset_list: Vec<String>) -> Self {
        let (to_engine, from_worker) = mpsc::channel();
        if asset_list.is_empty() {
            return Self {
                receiver: from_worker,
            };
        }

        std::thread::spawn(|| {
            let local_set = tokio::task::LocalSet::new();
            let (to_workers, mut from_asset_watcher) = tokio::sync::mpsc::channel(100);
            let to_engine_1 = to_engine.clone();
            local_set.spawn_local(async move {
                watch(asset_list, to_workers).await.map_err(|e| {
                    to_engine_1.send(WorkerMessage::Error(WorkerError::TaskFailed(format!(
                        "{e:?}"
                    ))))
                })
            });
            local_set.spawn_local(async move {
                loop {
                    if let Some(message) = from_asset_watcher.recv().await {
                        to_engine
                            .send(WorkerMessage::AssetUpdated(message))
                            .unwrap();
                    }
                }
            });

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            println!("[HOTHAM_WORKER] Runtime starting..");
            runtime.block_on(local_set);
            panic!("[HOTHAM_WORKER] RUNTIME FINISHED - THIS SHOULD NOT HAPPEN");
        });

        Self {
            receiver: from_worker,
        }
    }
}
