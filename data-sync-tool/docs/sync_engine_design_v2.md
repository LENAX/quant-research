# SyncEngine

`SyncEngine` is the central coordination module that manages the execution of synchronization tasks. It is responsbile for synchronization state management, worker allocation, and progress reporting.

### Submodules:

- `Worker`: Responsible for sending requests and receiving data
  - It has two subtypes: HttpApiWorker and WebsocketStreamingWorker
    - HttpApiWorker handles normal get and post requests
    - WebsocketStreamingWorker handles continuous websocket data streaming
- `TaskManager`: Responsible for task loading, sending, and rate control.

### Entities

- SyncPlan: Synchronization Plan object consisting tasks and specifications
- `Task`: The atomic unit of work that a worker executes.

### Functional Requirement

---

#### SyncPlan management

1. a sync plan could be added for execution
2. a sync plan could be cancelled during execution

#### SyncPlan execution

1. a sync plan can be executed by loading it to the task manager, and handled by workers
2. the execution result and the data received from remote can be received by the client modules

#### Execution Process Management

1. client modules can start, pause, resume, and cancel the execution of a sync plan
2. client modules can view the status and progress of each sync plan

#### Rate Throttling

1. Sync engine should not exceed the request limit of remote data providers

#### Error Handling

1. errors should be correctly handled and reported

### Non-Functional Requirement

---

#### Concurrency

* **Multithreading** : Leverage Rust's `tokio` runtime to execute multiple tasks concurrently.
* **Asynchronous I/O** : Use non-blocking I/O operations to maximize efficiency.

#### Performance

* **Efficiency** : Optimize task execution to minimize latency and resource usage.
* **Scalability** : Design `SyncEngine` to handle an increasing number of tasks without significant degradation in performance.

#### Reliability

* **Fault Tolerance** : Implement strategies to recover from worker failures or network issues.
* **Data Consistency** : Ensure data consistency across all synchronization operations


## Standard Operating Procedures (SOPs)

### SyncPlan Execution SOP

1. **Initialization** : A `SyncPlan` is created and added to the `SyncEngine`.
2. **Task Distribution** : The `TaskManager` loads the `SyncPlan` and assigns tasks to available workers.
3. **Execution** : Workers execute their assigned tasks concurrently.
4. **Monitoring** : The `TaskManager` monitors the progress and handles inter-task dependencies.
5. **Throttling** : Rate limiting is enforced to maintain compliance with external service limits.
6. **Completion** : Upon completion of all tasks, the `SyncEngine` reports the results to the client.
7. **Error Handling** : Any errors encountered are logged and reported to the client for further action.

### Worker Operation SOP

1. **Worker Initialization** : Workers are instantiated based on the `SyncPlan` requirements.
2. **Task Execution** : Workers perform synchronization tasks, handling HTTP or WebSocket communication.
3. **State Management** : Workers maintain state information for ongoing synchronization tasks.
4. **Data Handling** : Workers process incoming data and apply any necessary transformations.
5. **Result Reporting** : Workers report the outcome of tasks back to the `TaskManager`.

### Error Handling SOP

1. **Detection** : Errors are detected by workers or the `TaskManager` during task execution.
2. **Logging** : Errors are logged with sufficient detail for debugging purposes.
3. **Notification** : The `SyncEngine` notifies client modules of any errors.
4. **Recovery** : If possible, the `SyncEngine` attempts to recover from errors and resume operations.
5. **Escalation** : Unrecoverable errors are escalated for manual intervention.
