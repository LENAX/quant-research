use async_trait::async_trait;

use crate::{domain::synchronization::sync_task::SyncTask, infrastructure::mq::{message_bus::{MessageBusReceiver, StaticClonableAsyncComponent, BroadcastingMessageBusReceiver, MessageBusSender, BroadcastingMessageBusSender, StaticClonableMpscMQ, StaticAsyncComponent, MpscMessageBus}, tokio_channel_mq::{TokioMpscMessageBusSender, TokioBroadcastingMessageBusReceiver, TokioMpscMessageBusReceiver, TokioBroadcastingMessageBusSender}}};

use super::{errors::SyncWorkerError, worker::{SyncWorkerMessage, SyncWorkerData}};

/**
 * Synchronization worker traits
 */


 #[async_trait]
 pub trait SyncWorker: Send + Sync {
     // handles sync task, then updates its states and result
     // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<&mut SyncTask, Box<dyn Error>>;
     async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), SyncWorkerError>;
 }
 
 /// A marker trait that marks a long running worker
 pub trait LongTaskHandlingWorker {}
 
 /// A market trait for workers handling short tasks
 pub trait ShortTaskHandlingWorker {}
 pub trait ShortRunningWorker: SyncWorker + ShortTaskHandlingWorker {}
 pub trait LongRunningWorker: SyncWorker + LongTaskHandlingWorker {}
 
 trait SyncWorkerMessageBroadcastingReceiver:
     MessageBusReceiver<SyncWorkerMessage>
     + StaticClonableAsyncComponent
     + BroadcastingMessageBusReceiver
     + Clone
 {
 }
 trait SyncWorkerMessageBroadcastingSender:
     MessageBusSender<SyncWorkerMessage>
     + StaticClonableAsyncComponent
     + BroadcastingMessageBusSender<SyncWorkerMessage>
     + Clone
 {
 }
 
 trait SyncWorkerDataMpscReceiver:
     MessageBusReceiver<SyncWorkerData> + StaticClonableMpscMQ + Clone
 {
 }
 trait SyncWorkerDataMpscSender:
     MessageBusSender<SyncWorkerData> + StaticClonableMpscMQ + Clone
 {
 }
 
 trait SyncWorkerErrorMessageMpscReceiver:
     MessageBusReceiver<SyncWorkerError> + StaticClonableMpscMQ + Clone
 {
 }
 trait SyncWorkerErrorMessageMpscSender:
     MessageBusSender<SyncWorkerError> + StaticClonableMpscMQ + Clone
 {
 }
 
 impl SyncWorkerDataMpscSender for TokioMpscMessageBusSender<SyncWorkerData> {}
 impl SyncWorkerErrorMessageMpscSender for TokioMpscMessageBusSender<SyncWorkerError> {}
 
 // Try to work around the object safety restriction
 // Traits extend from Clone is not object-safe
 // Use this trait in external modules and use clone_boxed method to clone the object
 pub trait SyncWorkerMessageMPMCReceiver:
     MessageBusReceiver<SyncWorkerMessage> + BroadcastingMessageBusReceiver + StaticAsyncComponent
 {
     fn clone_boxed(&self) -> Box<dyn SyncWorkerMessageMPMCReceiver>;
 }
 pub trait SyncWorkerMessageMPMCSender:
     MessageBusSender<SyncWorkerMessage>
     + BroadcastingMessageBusSender<SyncWorkerMessage>
     + StaticAsyncComponent
 {
     fn clone_boxed(&self) -> Box<dyn SyncWorkerMessageMPMCSender>;
 }
 
 pub trait SyncWorkerDataMPSCReceiver:
     MessageBusReceiver<SyncWorkerData> + MpscMessageBus + StaticAsyncComponent
 {
 }
 pub trait SyncWorkerDataMPSCSender:
     MessageBusSender<SyncWorkerData> + MpscMessageBus + StaticAsyncComponent
 {
 }
 
 pub trait SyncWorkerErrorMessageMPSCReceiver:
     MessageBusReceiver<SyncWorkerError> + MpscMessageBus + StaticAsyncComponent
 {
 }
 
 pub trait SyncWorkerErrorMessageMPSCSender:
     MessageBusSender<SyncWorkerError> + MpscMessageBus + StaticAsyncComponent
 {
     fn clone_boxed(&self) -> Box<dyn SyncWorkerErrorMessageMPSCSender>;
 }
 
 impl SyncWorkerMessageBroadcastingReceiver
     for TokioBroadcastingMessageBusReceiver<SyncWorkerMessage>
 {
     // Implement the required methods
 }
 
 impl SyncWorkerDataMPSCSender for TokioMpscMessageBusSender<SyncWorkerData> {}
 impl SyncWorkerMessageMPMCReceiver for TokioBroadcastingMessageBusReceiver<SyncWorkerMessage> {
     fn clone_boxed(&self) -> Box<dyn SyncWorkerMessageMPMCReceiver> {
         Box::new(self.clone())
     }
 }
 
 impl SyncWorkerErrorMessageMPSCSender for TokioMpscMessageBusSender<SyncWorkerError> {
     fn clone_boxed(&self) -> Box<dyn SyncWorkerErrorMessageMPSCSender> {
         Box::new(self.clone())
     }
     // implement the methods required by SyncWorkerErrorMessageMPSCSender here
 }
 
 impl SyncWorkerDataMPSCReceiver for TokioMpscMessageBusReceiver<SyncWorkerData> {}
 impl StaticAsyncComponent for TokioBroadcastingMessageBusSender<SyncWorkerMessage> {}
 impl SyncWorkerMessageMPMCSender for TokioBroadcastingMessageBusSender<SyncWorkerMessage> {
     fn clone_boxed(&self) -> Box<dyn SyncWorkerMessageMPMCSender> {
         Box::new(self.clone())
     }
 }
 impl SyncWorkerErrorMessageMPSCReceiver for TokioMpscMessageBusReceiver<SyncWorkerError> {}
 