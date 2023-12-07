//! TaskManager Implementation
//! 

use std::collections::HashMap;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::infrastructure::sync_engine::message::ControlMessage;

use super::{commands::TaskManagerCommand, task_queue::TaskQueue};


pub struct TaskManager {
    receiver: mpsc::Receiver<TaskManagerCommand>,
    task_queues: HashMap<Uuid, TaskQueue>,
    is_running: bool
}

impl TaskManager {

    async fn run(mut self) {
        while let Some(command) = self.receiver.recv().await {
            match command {
                TaskManagerCommand::LifecycleControl(message) => {
                    match message {
                        ControlMessage::Start => { self.is_running = true },
                        ControlMessage::Stop =>  {
                            self.is_running = false;
                            self.receiver.close();
                            break;
                        },
                    }
                },
                TaskManagerCommand::SyncControl(message) => {

                },
                // ... other commands ...
            }
        }
    }
}