//! Data Sync Engine (Actor Model Version)
//!
//! This is a rewrite of the original sync engine that uses a lot of locks for shared state.
//! This version utilizes the actor model to become more message-driven and asynchronous.
//!
//! Primary Components:
//! 1. SyncEngine: Responsible for the state management and orchestrating synchronization
//! 2. WorkerPool: Responsible for managing workers during synchronization
//! 3. Worker: Do the actual data requesting and fetching
//! 4. TaskManager: Responsible for storing and dispatching tasks to workers
//! 5. TaskQueue: Containers holding Tasks
//! 6. Task: One unit of work, carrying specification for workers

pub mod engine;
pub mod engine_proxy;
pub mod task_manager;
pub mod worker;

use std::time::Duration;

use log::info;
use tokio::{sync::mpsc, time::sleep};

use crate::infrastructure::sync_engine::engine_proxy::EngineProxy;

use self::{
    engine::engine::SyncEngine,
    task_manager::task_manager::TaskManager,
    worker::{commands::WorkerResult, supervisor::Supervisor},
};

#[derive(Debug, PartialEq, Eq)]
pub enum ComponentState {
    Created,
    Running,
    Stopped,
}

/// Create and run sync engine
pub async fn init_engine(
    n_workers: Option<usize>,
    channel_size: Option<usize>,
    engine_timeout: Option<Duration>,
) -> EngineProxy {
    let channel_size = channel_size.unwrap_or(100);

    let (task_manager, task_manager_cmd_tx, task_manager_resp_tx, task_manager_resp_rx) =
        TaskManager::new();

    let (worker_result_tx, worker_result_rx) = mpsc::channel::<WorkerResult>(5 * channel_size);
    let (supervisor, supervisor_cmd_tx, supervisor_resp_rx) = Supervisor::new(
        n_workers,
        task_manager_cmd_tx.clone(),
        task_manager_resp_rx.resubscribe(),
        task_manager_resp_tx,
        worker_result_tx,
        engine_timeout,
    );

    let (sync_engine, engine_cmd_sender, engine_resp_receiver) = SyncEngine::new(
        task_manager_cmd_tx,
        task_manager_resp_rx,
        supervisor_cmd_tx,
        supervisor_resp_rx,
        engine_timeout,
    );

    info!("Start running task manager...");
    let _ = tokio::spawn(async move {
        task_manager.run().await;
    });
    // sleep(Duration::from_secs(1)).await;

    info!("Start running supervisor...");
    let _ = tokio::spawn(async move {
        supervisor.run().await;
    });
    // sleep(Duration::from_secs(1)).await;

    info!("Start running engine...");
    let _ = tokio::spawn(async move {
        sync_engine.run().await;
    });
    // sleep(Duration::from_secs(1)).await;
    info!("Engine initialized!");

    return EngineProxy::new(engine_cmd_sender, engine_resp_receiver, worker_result_rx);
}
