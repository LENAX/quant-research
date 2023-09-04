struct Task;

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

// allows types implementing this trait to make a builder of that type
pub trait AbstractBuilder {
    // type Product;
    type ConcreteBuilder;
    
    fn builder() -> Self::ConcreteBuilder;
}

pub trait Builder {
    type Product;

    fn new() -> Self;
    fn build(&mut self) -> Self::Product;
}

pub trait TaskQueueFieldSetters<RL: RateLimiter, TR: TaskReceiver> {
    fn with_queue(&mut self, queue: Vec<Task>) -> &mut Self;
    fn with_rate_limiter(&mut self, rate_limiter: RL) -> &mut Self;
    fn with_task_receiver(&mut self, task_receiver: TR) -> &mut Self;
}

struct TaskQueueABuilder<RL: RateLimiter, TR: TaskReceiver> {
    queue: Option<Vec<Task>>,
    rate_limiter: Option<RL>,
    task_receiver: Option<TR>
}

impl<RL: RateLimiter, TR: TaskReceiver> TaskQueueFieldSetters<RL, TR> for TaskQueueABuilder<RL, TR> {
    fn with_queue(&mut self, queue: Vec<Task>) -> &mut Self {
        self.queue = Some(queue);
        self
    }
    fn with_rate_limiter(&mut self, rate_limiter: RL) -> &mut Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }
    fn with_task_receiver(&mut self, task_receiver: TR) -> &mut Self {
        self.task_receiver = Some(task_receiver);
        self
    } 
}

impl<RL: RateLimiter, TR: TaskReceiver> Builder for TaskQueueABuilder<RL, TR> {
    type Product = TaskQueueA<RL, TR>;
    
    fn new() -> Self {
        Self { queue: Some(vec![]), rate_limiter: None, task_receiver: None }
    }
    fn build(&mut self) -> Self::Product {
        Self::Product {
            queue: self.queue.expect("queue is required!"),
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
    fn with_queue(&mut self, queue: Vec<Task>) -> &mut Self {
        self.queue = Some(queue);
        self
    }
    fn with_rate_limiter(&mut self, rate_limiter: RL) -> &mut Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }
    fn with_task_receiver(&mut self, task_receiver: TR) -> &mut Self {
        self.task_receiver = Some(task_receiver);
        self
    } 
}

impl<RL: RateLimiter, TR: TaskReceiver> Builder for TaskQueueBBuilder<RL, TR> {
    type Product = TaskQueueB<RL, TR>;
    
    fn new() -> Self {
        Self { queue: Some(vec![]), rate_limiter: None, task_receiver: None }
    }
    fn build(&mut self) -> Self::Product {
        Self::Product {
            queue: self.queue.expect("queue is required!"),
            rate_limiter: self.rate_limiter.expect("Rate limiter is a required component of TaskQueueA!"),
            task_receiver: self.task_receiver.expect("Task receiver is a required component of TaskQueueA!")
        }
    }
}

trait Queue {
    fn describe(&self);
    fn push_back(&mut self, task: Task);
}


struct TaskQueueA<RL: RateLimiter, TR: TaskReceiver> {
    queue: Vec<Task>,
    rate_limiter: RL,
    task_receiver: TR
}

impl<RL: RateLimiter, TR: TaskReceiver> AbstractBuilder for TaskQueueA<RL, TR> {
    type ConcreteBuilder = TaskQueueABuilder<RL, TR>;
    // type Product = TaskQueueA<RL, TR>;

    fn builder() -> Self::ConcreteBuilder {
        TaskQueueABuilder::<RL, TR>::new()
    }
}

impl<RL: RateLimiter, TR: TaskReceiver> Queue for TaskQueueA<RL, TR> {
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
    fn describe(&self) {
        println!("I am Product B");
    }
    
    fn push_back(&mut self, task: Task) {
        println!("Push 1 task in queue b!");
        self.queue.push(task);
    }
}

impl<RL: RateLimiter, TR: TaskReceiver> AbstractBuilder for TaskQueueB<RL, TR> {
    type ConcreteBuilder = TaskQueueBBuilder<RL, TR>;
    // type Product = TaskQueueA<RL, TR>;
    
    fn builder() -> Self::ConcreteBuilder {
        TaskQueueBBuilder::<RL, TR>::new()
    }
}


struct QueueFactory;

impl QueueFactory {
    fn create_queue<T: Queue + AbstractBuilder, RL: RateLimiter, TR: TaskReceiver>(rate_limiter: RL, task_receiver: TR) ->  <<T as AbstractBuilder>::ConcreteBuilder as Builder>::Product
    where
        T::ConcreteBuilder: TaskQueueFieldSetters<RL, TR> + Builder
    {
        let builder = T::builder();
        builder
            .with_rate_limiter(rate_limiter)
            .with_task_receiver(task_receiver)
            .build()
    }
}


struct TaskManager<TQ: Queue> {
    queue: Vec<TQ>
}

impl<TQ: Queue + AbstractBuilder> TaskManager<TQ> {
    pub fn new() -> Self {
        Self { queue: vec![] }
    }

    pub fn add_tasks_to_queue<RL: RateLimiter, TR: TaskReceiver>(&mut self, tasks: Vec<Task>, rate_limiter: RL, task_receiver: TR)
    where 
        <TQ as AbstractBuilder>::ConcreteBuilder: TaskQueueFieldSetters<RL, TR>,
        <TQ as AbstractBuilder>::ConcreteBuilder: Builder
    {
        let mut new_queue = QueueFactory::create_queue::<TQ, RL, TR>(rate_limiter, task_receiver);
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
