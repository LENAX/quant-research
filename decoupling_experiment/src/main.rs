/**
 * Decoupled component experiment
 * 模块解耦设计模式试验
 * 
 * In this example, I tried to explore how to write decoupled components with generics, advanced traits, and builder pattern.
 * If successful, I can proceed to apply this pattern to my data-sync-tool program.
 * 
 * 这段代码是为了试出正确书写解耦模块的方式，以便将所得的套路用在data-sync-tool项目上。
 * 所用到的特性：泛型，关联类型和构建模式
 */

 #[derive(Debug, Clone, Copy)]
 pub struct Task;
 
 pub trait RateLimiter {
     fn can_proceed(&self) -> bool;
 }
 
 struct RequestRateLimiterA;
 struct RequestRateLimiterB;
 
 impl RateLimiter for RequestRateLimiterA {
     fn can_proceed(&self) -> bool {
         println!("RequestRateLimiterA says ok");
         true
     }
 }
 
 impl RateLimiter for RequestRateLimiterB {
     fn can_proceed(&self) -> bool {
         println!("RequestRateLimiterB says no");
         false
     }
 }
 
 pub trait TaskReceiver {
     fn send(&self, message: &str);
     fn receive(&mut self);
 }
 
 struct TaskMpscChannelReceiver;
 impl TaskReceiver for TaskMpscChannelReceiver {
     fn send(&self, message: &str) {
         println!("Sent a message {} in TaskMpscChannelReceiver", message);
     }
     
     fn receive(&mut self) {
         println!("Received a message in TaskMpscChannelReceiver");
     }
 }
 
 struct TaskBroadcastChannelReceiver;
 impl TaskReceiver for TaskBroadcastChannelReceiver {
     fn send(&self, message: &str) {
         println!("Sent a message {} in TaskBroadcastChannelReceiver", message);
     }
     
     fn receive(&mut self) {
         println!("Received a message in TaskBroadcastChannelReceiver");
     }
 }
 
 
 pub trait Builder {
     type Product;
 
     fn new() -> Self;
     fn build(self) -> Self::Product;
 }
 
 pub trait TaskQueueFieldSetters<RL: RateLimiter, TR: TaskReceiver> {
     fn with_queue(self, queue: Vec<Task>) -> Self;
     fn with_rate_limiter(self, rate_limiter: RL) -> Self;
     fn with_task_receiver(self, task_receiver: TR) -> Self;
 }
 
 #[derive(Debug)]
 struct TaskQueueABuilder<RL: RateLimiter, TR: TaskReceiver> {
     queue: Option<Vec<Task>>,
     rate_limiter: Option<RL>,
     task_receiver: Option<TR>
 }
 
 impl<RL: RateLimiter, TR: TaskReceiver> TaskQueueFieldSetters<RL, TR> for TaskQueueABuilder<RL, TR> {
     fn with_queue(mut self, queue: Vec<Task>) -> Self {
         self.queue = Some(queue);
         self
     }
     fn with_rate_limiter(mut self, rate_limiter: RL) -> Self {
         self.rate_limiter = Some(rate_limiter);
         self
     }
     fn with_task_receiver(mut self, task_receiver: TR) -> Self {
         self.task_receiver = Some(task_receiver);
         self
     } 
 }
 
 impl<RL: RateLimiter, TR: TaskReceiver> Builder for TaskQueueABuilder<RL, TR> {
     type Product = TaskQueueA<RL, TR>;
     
     fn new() -> Self {
         Self { queue: Some(vec![]), rate_limiter: None, task_receiver: None }
     }
     fn build(self) -> Self::Product {
         Self::Product {
             queue: self.queue.unwrap_or(vec![]),
             rate_limiter: self.rate_limiter.expect("Rate limiter is a required component of TaskQueueA!"),
             task_receiver: self.task_receiver.expect("Task receiver is a required component of TaskQueueA!")
         }
     }
 }
 
 struct TaskQueueBBuilder<RL: RateLimiter, TR: TaskReceiver> {
     queue: Option<Vec<Task>>,
     rate_limiter: Option<RL>,
     task_receiver: Option<TR>
 }
 
 
 impl<RL: RateLimiter, TR: TaskReceiver> TaskQueueFieldSetters<RL, TR> for TaskQueueBBuilder<RL, TR> {
     fn with_queue(mut self, queue: Vec<Task>) -> Self {
         self.queue = Some(queue);
         self
     }
     fn with_rate_limiter(mut self, rate_limiter: RL) -> Self {
         self.rate_limiter = Some(rate_limiter);
         self
     }
     fn with_task_receiver(mut self, task_receiver: TR) -> Self {
         self.task_receiver = Some(task_receiver);
         self
     } 
 }
 
 impl<RL: RateLimiter, TR: TaskReceiver> Builder for TaskQueueBBuilder<RL, TR> {
     type Product = TaskQueueB<RL, TR>;
     
     fn new() -> Self {
         Self { queue: Some(vec![]), rate_limiter: None, task_receiver: None }
     }
     fn build(self) -> Self::Product {
         Self::Product {
             queue: self.queue.unwrap_or(vec![]),
             rate_limiter: self.rate_limiter.expect("Rate limiter is a required component of TaskQueueA!"),
             task_receiver: self.task_receiver.expect("Task receiver is a required component of TaskQueueA!")
         }
     }
 }
 
 trait Queue {
     type BuilderType;
 
     fn describe(&self);
     fn push_back(&mut self, task: Task);
 }
 
 
 struct TaskQueueA<RL: RateLimiter, TR: TaskReceiver> {
     queue: Vec<Task>,
     rate_limiter: RL,
     task_receiver: TR
 }
 
 impl<RL: RateLimiter, TR: TaskReceiver> Queue for TaskQueueA<RL, TR> {
     type BuilderType = TaskQueueABuilder<RL, TR>;
 
     fn describe(&self) {
         println!("I am Product A");
     }
     
     fn push_back(&mut self, task: Task) {
         println!("Push 1 task in queue a!");
         self.queue.push(task);
     }
 }
 
 struct TaskQueueB<RL: RateLimiter, TR: TaskReceiver> {
     queue: Vec<Task>,
     rate_limiter: RL,
     task_receiver: TR
 }
 
 impl<RL: RateLimiter, TR: TaskReceiver> Queue for TaskQueueB<RL, TR> {
     type BuilderType = TaskQueueBBuilder<RL, TR>;
 
     fn describe(&self) {
         println!("I am Product B");
     }
     
     fn push_back(&mut self, task: Task) {
         println!("Push 1 task in queue b!");
         self.queue.push(task);
     }
 }
 
 struct QueueFactory;
 
 impl QueueFactory {
     // fn use_impl
 
     fn create_queue<B: Builder + TaskQueueFieldSetters<RL, TR>, RL: RateLimiter, TR: TaskReceiver>(rate_limiter: RL, task_receiver: TR) -> B::Product
     where
         B::Product: Queue
     {
         let builder = B::new();
         builder
             .with_rate_limiter(rate_limiter)
             .with_task_receiver(task_receiver)
             .build()
     }
 }
 
 
 struct TaskManager<TQ: Queue> {
     queue: Vec<TQ>
 }
 
 impl<TQ: Queue> TaskManager<TQ> {
     pub fn new() -> Self {
         Self { queue: vec![] }
     }
 
     pub fn add_tasks_to_queue<RL: RateLimiter, TR: TaskReceiver>(&mut self, tasks: Vec<Task>, rate_limiter: RL, task_receiver: TR)
     where
         <TQ as Queue>::BuilderType: Builder<Product = TQ>,
         <TQ as Queue>::BuilderType: TaskQueueFieldSetters<RL, TR> 
         // <<TQ as Queue>::BuilderType as Builder>::Product: Queue
     {
         let mut new_queue = QueueFactory::create_queue::<TQ::BuilderType, RL, TR>(rate_limiter, task_receiver);
         for task in tasks {
             new_queue.push_back(task);
         }
         self.queue.push(new_queue);
         println!("TaskManager has {} queue", self.queue.len());
     }
 }
 
 
 fn main() {
     let mut task_manager_a = TaskManager::<TaskQueueA<RequestRateLimiterA, TaskBroadcastChannelReceiver>>::new();
     let mut task_manager_b = TaskManager::<TaskQueueB<RequestRateLimiterB, TaskMpscChannelReceiver>>::new();
     
     let tasks_for_a = (0..5).map(|_| {
         (0..10).map(|_| { Task {} }).collect::<Vec<_>>()
     }).collect::<Vec<_>>();
     let tasks_for_b = (0..5).map(|_| {
         (0..10).map(|_| { Task {} }).collect::<Vec<_>>()
     }).collect::<Vec<_>>();
     
     for task_list in tasks_for_a {
         task_manager_a.add_tasks_to_queue::<RequestRateLimiterA, TaskBroadcastChannelReceiver>(
             task_list, RequestRateLimiterA {}, TaskBroadcastChannelReceiver {});
     }
     
     for task_list in tasks_for_b {
         task_manager_b.add_tasks_to_queue::<RequestRateLimiterB, TaskMpscChannelReceiver>(
             task_list, RequestRateLimiterB{}, TaskMpscChannelReceiver{});
     }
 }
 