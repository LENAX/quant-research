# SyncEngine (MVP Version)

`SyncEngine` is the central coordination module that manages the execution of synchronization tasks. It is responsbile for synchronization state management, worker allocation, and progress reporting.

This document describes the minimally viable product version of the sync engine.

The MVP sync engine will support:

1. Basic plan synchronization management
   1. Add plan
   2. Remove plan
2. Synchronization state management
   1. Start all plans
   2. Cancel all plans
3. Task management
   1. Load tasks from plans and store them in queues
4. Basic Worker Management
   1. Implement a supervisor to handle worker management
5. Worker based synchronization
   1. HTTP API only
   2. Hard coded rate limiting

### Submodules:

- `Worker`: Responsible for sending requests and receiving data
  - It has two subtypes: HttpApiWorker and WebsocketStreamingWorker
    - HttpApiWorker handles normal get and post requests
    - WebsocketStreamingWorker: not supported in mvp version
- `TaskManager`: Responsible for task loading, and sending tasks upon request.
- `TaskQueue`: Queue for storing tasks
- `Supervisor`: Responsible for managing workers
  - Assign tasks to workers
  - Find idle workers

### Entities

- SyncPlan: Synchronization Plan object consisting tasks and specifications
- `Task`: The atomic unit of work that a worker executes.


### Development Breakdown and Considerations

#### 1. Basic Plan Synchronization Management

- **Add Plan**: Implement a method in `SyncEngine` to receive and store `SyncPlan` objects. This will likely involve interacting with `TaskManager` to load tasks from the plans into `TaskQueue`.
- **Remove Plan**: Allow for the removal of plans, which includes clearing associated tasks and updating any worker assignments.

#### 2. Synchronization State Management

- **Start All Plans**: A method in `SyncEngine` to signal the `Supervisor` to start executing tasks on available workers.
- **Cancel All Plans**: Implement cancellation logic that instructs workers to stop current tasks and prevents queued tasks from being executed.

#### 3. Task Management

- **Load Tasks**: `TaskManager` should efficiently load tasks from `SyncPlan`s and manage them in `TaskQueue`s.

#### 4. Basic Worker Management

- **Supervisor for Worker Management**: Develop the `Supervisor` submodule to manage worker lifecycle, including task assignment and monitoring worker status (idle, busy, etc.).

#### 5. Worker-based Synchronization

- **HTTP API Worker**: Focus on implementing the `HttpApiWorker` subtype, which handles HTTP GET and POST requests.
- **Rate Limiting**: Implement basic hardcoded rate limiting in `HttpApiWorker` to manage the frequency of requests.

### Submodules Implementation

#### Worker

- **HttpApiWorker**: Focus on creating a functional HTTP client capable of sending requests and handling responses.
- **Worker Structure**: Each worker should be able to report its status back to the `Supervisor` and receive tasks to execute.

#### TaskManager

- **Task Loading**: Implement logic to load tasks from `SyncPlan`s and enqueue them.
- **Task Distribution**: Handle requests for tasks from the `Supervisor` and distribute tasks accordingly.

#### TaskQueue

- **Queue Implementation**: Ensure efficient queuing and retrieval of tasks. Consider using Rust's standard library collections like `VecDeque`.

#### Supervisor

- **Worker Allocation**: Implement logic to manage the worker pool, including task assignment based on worker availability.
- **Idle Worker Finding**: Efficiently identify idle workers and assign them new tasks.

### Entities

- **SyncPlan**: Define the structure to encapsulate synchronization details, such as endpoints, credentials, and the collection of tasks.
- **Task**: Design the `Task` entity to represent individual work units, including necessary parameters for HTTP requests.

### Additional Tips

- **Testing**: Develop unit tests for each component to ensure functionality and catch errors early.
- **Documentation**: Document each part of your codebase for clarity and maintenance.
- **Modularity**: Keep the code modular to facilitate easy updates and potential feature expansions.
- **Error Handling**: Implement robust error handling, especially for network operations and inter-module communication.

### Project Management

- **Version Control**: Use Git for version control to manage your codebase effectively.
- **Iterative Development**: Adopt an iterative development approach, starting with the most critical features and then gradually building upon them.
- **Feedback Loops**: Regularly test the integration of components to ensure they work well together.
