use hotham_asset_client::{watch, AssetUpdatedMessage};

use std::sync::mpsc;

#[derive(Debug, Clone)]
pub(crate) enum WorkerMessage {
    AssetUpdated(AssetUpdatedMessage),
    Error(WorkerError),
}

#[derive(Debug, Clone)]
pub(crate) enum WorkerError {
    TaskFailed(String),
}

pub(crate) struct Workers {
    pub(crate) receiver: mpsc::Receiver<WorkerMessage>,
}

#[cfg(not(target_os = "android"))]
static ASSETS_PATH: &'static str = "assets.glb";

#[cfg(target_os = "android")]
static ASSETS_PATH: &'static str = "assets_squished.glb";

impl Default for Workers {
    fn default() -> Self {
        let (to_engine, from_engine) = mpsc::channel();

        std::thread::spawn(|| {
            let local_set = tokio::task::LocalSet::new();
            let (to_workers, mut from_asset_watcher) = tokio::sync::mpsc::channel(100);
            let to_engine_1 = to_engine.clone();
            local_set.spawn_local(async move {
                watch(vec![ASSETS_PATH.into()], to_workers)
                    .await
                    .map_err(|e| {
                        to_engine_1.send(WorkerMessage::Error(WorkerError::TaskFailed(format!(
                            "{:?}",
                            e
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
            runtime.block_on(async { local_set.await });
            panic!("[HOTHAM_WORKER] RUNTIME FINISHED - THIS SHOULD NOT HAPPEN");
        });

        Self {
            receiver: from_engine,
        }
    }
}
