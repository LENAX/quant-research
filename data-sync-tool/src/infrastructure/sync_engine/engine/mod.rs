//! # Synchronization Engine
//! Responsible for the state management and orchestrating synchronization
//! The MVP version will use tokio channels rather than the abstracted version to simplify implementation
//! 
//! ## Major Behavior
//! ### Engine Lifecycle Management
//! 1. Initialization
//! 2. Start
//! 3. Pause
//! 4. Resume
//! 5. Stop
//! 6. Shutdown
//! 
//! ### Synchronization State Management
//! 1. start_all_plans
//! 2. pause_all_plans
//! 3. stop_all_plans
//! 4. resume_all_plans
//! 5. start_plans
//! 6. pause_plans
//! 7. stop_plans
//! 8. resume_lans
//! 
//! ### Progress and State Reporting
//! 1. get_progress
//! 2. listen_for_progress
//! 
pub mod engine;
pub mod commands;