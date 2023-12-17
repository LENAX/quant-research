// In src/lib.rs
pub mod domain;
pub mod infrastructure {
    pub mod sync_engine;
    // Re-export
    pub use sync_engine::engine;
    pub use sync_engine::engine::commands;
    pub use sync_engine::task_manager;
    pub use sync_engine::task_manager::commands as tm_commands;
    pub use sync_engine::worker;
    pub use sync_engine::worker::commands as worker_commands;
    // pub use sync_engine::engine::engine::SyncEngine;
}
