//! TaskManager Implementation
//!

use std::collections::{HashMap, VecDeque};

use getset::Getters;
use log::info;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::{
    domain::synchronization::value_objects::task_spec::TaskSpecification,
    infrastructure::sync_engine::ComponentState,
};

use super::commands::{TaskManagerCommand, TaskManagerResponse, TaskRequestResponse};

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct Task {
    plan_id: Uuid,
    task_id: Uuid,
    spec: TaskSpecification,
}

#[derive(Debug)]
pub struct TaskManager {
    cmd_rx: mpsc::Receiver<TaskManagerCommand>,
    resp_tx: broadcast::Sender<TaskManagerResponse>,
    task_queues: HashMap<Uuid, VecDeque<Task>>,
    task_senders: HashMap<Uuid, broadcast::Sender<TaskRequestResponse>>,
    state: ComponentState,
}

impl TaskManager {
    pub fn new() -> (
        Self,
        mpsc::Sender<TaskManagerCommand>,
        broadcast::Receiver<TaskManagerResponse>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<TaskManagerCommand>(300);
        let (resp_tx, resp_rx) = broadcast::channel::<TaskManagerResponse>(300);
        let tm = Self {
            cmd_rx,
            resp_tx,
            task_queues: HashMap::new(),
            task_senders: HashMap::new(),
            state: ComponentState::Created,
        };

        (tm, cmd_tx, resp_rx)
    }

    pub async fn run(mut self) {
        while let Some(command) = self.cmd_rx.recv().await {
            match command {
                TaskManagerCommand::Shutdown => {
                    info!("Shutting down Task Manager...");
                    // Perform any cleanup needed for shutdown
                    self.state = ComponentState::Stopped;
                    let _ = self.resp_tx.send(TaskManagerResponse::ShutdownComplete);
                    break;
                }
                TaskManagerCommand::AddPlan(plan) => {
                    info!("Add new plan to Task Manager...");
                    let plan_id = plan.plan_id; // Assuming plan_id() method exists
                    let tasks: VecDeque<Task> = plan
                        .task_specs()
                        .iter()
                        .map(|spec| {
                            return Task {
                                plan_id: plan_id,
                                task_id: Uuid::new_v4(),
                                spec: spec.clone(),
                            };
                        })
                        .collect();
                    self.task_queues.insert(plan_id, tasks);

                    let task_tx = broadcast::channel::<TaskRequestResponse>(100).0;
                    self.task_senders.insert(plan_id, task_tx);
                    
                    let _ = self
                        .resp_tx
                        .send(TaskManagerResponse::PlanAdded { plan_id });
                }
                // TODO: Save checkpoint for the removed plan
                TaskManagerCommand::RemovePlan(plan_id) => {
                    info!("Remove plan to Task Manager...");
                    self.task_queues.remove(&plan_id);
                    let _ = self
                        .resp_tx
                        .send(TaskManagerResponse::PlanRemoved { plan_id });
                }
                TaskManagerCommand::RequestTask(plan_id) => {
                    let response = self.task_queues.get_mut(&plan_id)
                        .and_then(|queue| queue.pop_front()) // Try to get a task from the queue
                        .map_or_else(
                            || TaskRequestResponse::NoTaskLeft, // If no task, return NoTaskLeft
                            |task| TaskRequestResponse::NewTask(task) // If there's a task, return NewTask
                        );

                    if let Some(task_sender) = self.task_senders.get(&plan_id) {
                        let _ = task_sender.send(response); // Send the response
                    }
                },
                TaskManagerCommand::RequestTaskReceiver { plan_id } => {
                    let response = if let Some(task_sender) = self.task_senders.get(&plan_id) {
                        TaskManagerResponse::TaskChannel { plan_id, task_sender: task_sender.clone() }
                    } else {
                        TaskManagerResponse::Error { message: format!("No task sender found for plan {}", plan_id) }
                    };

                    let _ = self.resp_tx.send(response);
                }
            }
        }
    }
}
