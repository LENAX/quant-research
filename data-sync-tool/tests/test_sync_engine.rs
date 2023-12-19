use data_sync_tool::{infrastructure::commands::Plan, domain::synchronization::value_objects::task_spec::TaskSpecification};
use uuid::Uuid;


use std::collections::HashMap;

pub fn create_mock_plan() -> Plan {
    // Generate a random UUID for the plan
    let plan_id = Uuid::new_v4();

    // Create a list of task specifications
    let mut task_specs = Vec::new();

    // Define a GET request task
    let get_task_result = TaskSpecification::new(
        "https://httpbin.org/get",
        "GET",
        HashMap::new(), // No additional headers for GET
        None,
    );
    if let Ok(get_task) = get_task_result {
        for _ in 0..10 {
            task_specs.push(get_task.clone());
        }
    }

    // Define a POST request task with a simple JSON payload
    let mut post_headers = HashMap::new();
    post_headers.insert("Content-Type".to_string(), "application/json".to_string());

    let post_payload = serde_json::json!({
        "key": "value"
    });

    let post_task_result = TaskSpecification::new(
        "https://httpbin.org/post",
        "POST",
        post_headers,
        Some(post_payload),
    );
    if let Ok(post_task) = post_task_result {
        for _ in 0..10 {
            task_specs.push(post_task.clone());
        }
    }

    // Construct the Plan with the generated tasks
    Plan {
        plan_id,
        task_specs,
    }
}

#[cfg(test)]
mod integration_tests {
    use data_sync_tool::infrastructure::{sync_engine::init_engine, worker_commands::WorkerResult};
    use log::info;
    use tokio::{time::{timeout, Duration, sleep}, sync::broadcast};
    use fast_log::config::Config;

    use crate::create_mock_plan;
    use pretty_env_logger;

    async fn process_worker_results(mut results_rx: broadcast::Receiver<WorkerResult>) {
        while let Ok(result) = results_rx.recv().await {
            // Process each worker result here
            info!("Received worker result: {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_web_api_integration() {
        // fast_log::init(Config::new().console().chan_len(Some(100000))).unwrap();
        pretty_env_logger::init();
        
        // Initialize the engine
        let mut engine = init_engine(Some(4), Some(100), Some(Duration::from_secs(5))).await;
        let results_rx = engine.subscribe_to_worker_results();

        sleep(Duration::from_secs(1)).await;

        tokio::spawn(async move {
            process_worker_results(results_rx).await;
        });

        // Create a plan (Example, replace with actual plan creation)
        let plans = vec![
            create_mock_plan(),
            create_mock_plan(),
            create_mock_plan(),
            create_mock_plan(),
        ];

        // // Add a plan and start it immediately
        for plan in plans {
            let add_plan_result = timeout(Duration::from_secs(10), 
            engine.add_plan(plan, false)).await.expect("Timeout while adding plan");
        
            info!("add_plan_result: {:?}", add_plan_result);
            assert!(add_plan_result.is_ok(), "Failed to add plan");
        }


        // let plan_id = add_plan_result.unwrap();

        // Check if the plan started successfully
        // FIXME: Start sync does not work
        let start_sync_result = timeout(Duration::from_secs(10), 
            engine.start_sync()).await.expect("Timeout while starting sync");

        assert!(start_sync_result.is_ok(), "Failed to start synchronization");

        // // Optionally, check for worker results
        // // ...
        sleep(Duration::from_secs(30)).await;
        // // Finally, shutdown the engine
        let shutdown_result = timeout(Duration::from_secs(10), 
            engine.shutdown()).await.expect("Timeout while shutting down");
        assert!(shutdown_result.is_ok(), "Failed to shut down engine");
    }
}
