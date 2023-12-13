use std::collections::HashMap;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::infrastructure::{sync_engine::task_manager::commands::TaskManagerCommand, sync::workers::worker_traits::WorkerCommand};

use super::{worker::Worker, commands::SupervisorCommand};


pub struct SupervisorConfig {
    worker_pool_size: usize
    // ... other fields ...
}

pub struct Supervisor {
    task_manager_tx: mpsc::Sender<TaskManagerCommand>,
    worker_channels: HashMap<Uuid, mpsc::Sender<WorkerCommand>>,
    command_receiver: mpsc::Receiver<SupervisorCommand>,
    
    // ... other state ...
}

impl Supervisor {
    async fn new(task_manager_tx: mpsc::Sender<TaskManagerCommand>, config: &SupervisorConfig) -> Self {
        let mut supervisor = Supervisor {
            task_manager_tx,
            worker_channels: HashMap::new(),
        };
        supervisor.initialize_workers(config.worker_pool_size).await;
        supervisor
    }

    async fn initialize_workers(&mut self, count: usize) {
        for _ in 0..count {
            let (tx, rx) = mpsc::channel(32);
            let worker_id = Uuid::new_v4();
            self.worker_channels.insert(worker_id, tx);
            
            let worker = Worker::new(self.task_manager_tx.clone());
            tokio::spawn(async move {
                worker.run().await;
            });
        }
    }


    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(command) = self.command_receiver.recv() => {
                    match command {
                        SupervisorCommand::AddTask(task) => {
                            self.task_queue.push(task);
                        }
                        SupervisorCommand::Shutdown => break,
                        // Handle other commands
                    }
                }
                Some(request) = self.worker_request_receiver.recv() => {
                    match request {
                        WorkerRequest::RequestTask => {
                            if let Some(task) = self.task_queue.pop() {
                                // Find an available worker and send the task
                                // Assuming we know the worker's ID and have a mapping to their sender
                                // ...
                            }
                        }
                        // Handle other requests
                    }
                }
            }
        }
    }

    // ... other methods ...
}