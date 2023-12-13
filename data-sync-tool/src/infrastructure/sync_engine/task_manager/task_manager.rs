//! TaskManager Implementation
//!

use std::collections::{HashMap, VecDeque};

use getset::Getters;
use log::info;
use tokio::sync::{mpsc, broadcast};
use uuid::Uuid;

use crate::{infrastructure::sync_engine::{message::ControlMessage, ComponentState}, domain::synchronization::value_objects::task_spec::TaskSpecification};

use super::commands::{TaskManagerCommand, TaskManagerResponse};

#[derive(Debug, Clone, Getters)]
pub struct Task {
    plan_id: Uuid,
    task_id: Uuid,
    spec: TaskSpecification
}

#[derive(Debug)]
pub struct TaskManager {
    cmd_rx: mpsc::Receiver<TaskManagerCommand>,
    resp_tx: broadcast::Sender<TaskManagerResponse>,
    task_queues: HashMap<Uuid, VecDeque<Task>>,
    state: ComponentState
}

impl TaskManager {
    pub fn new() -> (
        Self,
        mpsc::Sender<TaskManagerCommand>,
        broadcast::Receiver<TaskManagerResponse>,
    ) {
        let (cmd_tx, 
             cmd_rx) = mpsc::channel::<TaskManagerCommand>(300);
        let (resp_tx, 
             resp_rx) = broadcast::channel::<TaskManagerResponse>(300);
        let tm = Self {
            cmd_rx, resp_tx,
            task_queues: HashMap::new(),
            state: ComponentState::Created
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
                },
                TaskManagerCommand::AddPlan(plan) => {
                    info!("Add new plan to Task Manager...");
                    let plan_id = plan.plan_id; // Assuming plan_id() method exists
                    self.task_queues.insert(plan_id, VecDeque::from(plan.task_specs)); // Assuming TaskQueue::new() method exists
                    let _ = self.resp_tx.send(TaskManagerResponse::PlanAdded { plan_id });
                },
                // TODO: Save checkpoint for the removed plan
                TaskManagerCommand::RemovePlan(plan_id) => {
                    info!("Remove plan to Task Manager...");
                    self.task_queues.remove(&plan_id);
                    let _ = self.resp_tx.send(TaskManagerResponse::PlanRemoved { plan_id });
                },
                TaskManagerCommand::RequestTask(plan_id) => {
                    if let Some(queue) = self.task_queues.get_mut(&plan_id) {
                        if let Some(task_spec) = queue.pop_front() { // Assuming next_task() method exists
                            let _ = self.resp_tx.send(TaskManagerResponse::NewTask { plan_id, task: task_spec });
                        }
                    }
                },
            }
        }
    }
}
