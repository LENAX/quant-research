/**
 * Sync Task Executor Factory
 * 
 * Provides easy-to-use APIs to create SyncTaskExecutor
 */



type TokioExecutorChannels = (
    TokioMpscMessageBusReceiver<StreamingData>,
    TokioBroadcastingMessageBusSender<SyncWorkerMessage>,
    TokioMpscMessageBusReceiver<SyncWorkerError>,
    TokioMpscMessageBusReceiver<SyncTask>,
    TokioMpscMessageBusReceiver<TaskManagerError>,
    TokioSpmcMessageBusSender<FailedTask>,
);

pub fn create_tokio_task_executor<TRS, TTR, CTS, FTS>(
    n_workers: usize,
    channel_size: usize,
    create_tm_request: &CreateTaskManagerRequest,
) -> (
    SyncTaskExecutor<
        WebsocketSyncWorker<
            TokioMpscMessageBusSender<StreamingData>,
            TokioBroadcastingMessageBusReceiver<SyncWorkerMessage>,
            TokioMpscMessageBusSender<SyncWorkerError>,
        >,
        WebAPISyncWorker<TRS, TTR, CTS, FTS>,
        TaskManager<
            WebRequestRateLimiter,
            TokioBroadcastingMessageBusSender<SyncTask>,
            TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
            TokioMpscMessageBusSender<TaskManagerError>,
            TokioSpmcMessageBusReceiver<FailedTask>,
        >,
    >,
    TokioBroadcastingMessageBusSender<SyncWorkerMessage>,
) {
    // create message bus channels
    let (task_sender, task_receiver) = 
        create_tokio_broadcasting_channel::<SyncTask>(channel_size);
    let (task_request_sender, task_request_receiver) =
        create_tokio_broadcasting_channel::<GetTaskRequest>(channel_size);
    let (error_sender, error_receiver) =
        create_tokio_mpsc_channel::<TaskManagerError>(channel_size);
    let (failed_task_sender, failed_task_receiver) =
        create_tokio_spmc_channel::<(Uuid, SyncTask)>(channel_size);
    let (sync_worker_message_sender, sync_worker_message_receiver) =
        create_tokio_broadcasting_channel::<SyncWorkerMessage>(channel_size);
    let (sync_worker_data_sender, sync_worker_data_receiver) =
        create_tokio_mpsc_channel::<StreamingData>(channel_size);
    let (sync_worker_error_sender, sync_worker_error_receiver) =
        create_tokio_mpsc_channel::<SyncWorkerError>(channel_size);

    // create workers
    let long_running_workers = create_websocket_sync_workers(
        n_workers,
        sync_worker_data_sender,
        sync_worker_message_receiver.clone(),
        sync_worker_error_sender,
    );
    let short_running_workers = create_web_api_sync_workers(n_workers);

    // create task manager
    // FIXME: Derive receiver based on create task manager request
    let n_queues = create_tm_request.create_task_queue_requests().len();
    let task_request_receivers: Vec<_> = (0..n_queues).map(|_| { task_request_receiver.clone() }).collect();
    let task_manager = create_sync_task_manager(
        create_tm_request,
        task_sender,
        task_request_receivers,
        error_sender,
        failed_task_receiver,
    );

    // create task executor
    let task_executor = SyncTaskExecutor {
        long_running_workers,
        short_task_handling_workers: short_running_workers,
        task_manager,
        worker_channels: WorkerChannels {
            worker_data_receiver: Arc::new(RwLock::new(Box::new(sync_worker_data_receiver))),
            worker_message_sender: Arc::new(RwLock::new(sync_worker_message_sender.clone_boxed())),
            worker_error_receiver: Arc::new(RwLock::new(Box::new(sync_worker_error_receiver))),
        },
        task_manager_channels: TaskManagerChannels {
            sync_task_receiver: Arc::new(RwLock::new(Box::new(task_receiver))),
            task_request_sender: Arc::new(RwLock::new(Box::new(task_request_sender))),
            failed_task_sender: Arc::new(RwLock::new(Box::new(failed_task_sender))),
            manager_error_receiver: Arc::new(RwLock::new(Box::new(error_receiver))),
        },
    };

    (task_executor, sync_worker_message_sender.clone())
}