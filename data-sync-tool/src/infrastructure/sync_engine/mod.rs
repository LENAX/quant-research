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
pub mod task_manager;
pub mod worker;
pub mod message;