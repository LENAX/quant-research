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



### Updated Development Plan

#### Week 1-2: Core System Setup and Basic Plan Management

- **System Architecture Setup**: Initialize the Rust project and set up the basic architecture. This includes creating modules for `Worker`, `TaskManager`, `TaskQueue`, `Supervisor`, and `SyncEngine`.
- **SyncPlan and Task Implementation**: Define the `SyncPlan` and `Task` structures. `SyncPlan` includes details like endpoints, credentials, and a collection of `Task`s.
- **Plan Management in SyncEngine**: Implement methods in `SyncEngine` for adding and removing `SyncPlan`s. These methods will interact with the `TaskManager` to load and manage tasks.

#### Week 3-4: Task Manager and Task Queue

- **Task Loading**: Implement logic in the `TaskManager` for loading tasks from `SyncPlan`s into the `TaskQueue`.
- **Task Queue Management**: Develop the `TaskQueue` to efficiently enqueue and dequeue tasks.

#### Week 5-6: Worker and Supervisor Development

- **HTTP API Worker**: Focus on `HttpApiWorker` implementation, capable of handling HTTP GET and POST requests.
- **Supervisor Implementation**: Develop the `Supervisor` to manage workers. This includes assigning tasks to workers and monitoring their status.

#### Week 7-8: Synchronization State Management and Worker Supervision

- **Start and Cancel Synchronization**: Implement methods in `SyncEngine` for starting and canceling all plans. This involves coordinating with the `Supervisor` to control the task assignments to workers.
- **Worker State Monitoring**: Enhance the `Supervisor` to actively monitor and update the state of each worker (idle, busy, etc.).
- **Rate Limiting**: Implement basic hardcoded rate limiting in `HttpApiWorker`.

#### Week 9-10: RESTful API Development

- **API Setup**: Set up a web server using a framework like `warp` or `actix-web`.
- **API Endpoints**: Define and implement RESTful API endpoints for interacting with the `SyncEngine` (e.g., adding plans, starting synchronization).

#### Week 11-12: Integration and Testing

- **System Integration**: Integrate all components and ensure they work together as expected.
- **Testing**: Conduct extensive testing to identify and fix bugs. Focus on both unit testing for individual components and integration testing for the entire system.

#### Week 13-14: Refinement, Documentation, and Deployment

- **Code Refinement**: Refine the code for better performance, readability, and error handling.
- **Documentation**: Document the codebase, API usage, and deployment instructions.
- **Deployment**: Prepare the system for deployment, which may include containerization or setting up a cloud environment.

### Additional Considerations

- **Scalability and Performance**: Design with scalability in mind, but don't over-optimize prematurely.
- **Iterative and Agile Approach**: Adapt and refine the plan as you progress, based on testing feedback and any new insights.
- **Version Control**: Regularly commit to Git and possibly use feature branches for new components.
- **Regular Code Reviews**: Conduct code reviews, if working in a team, to maintain code quality.
