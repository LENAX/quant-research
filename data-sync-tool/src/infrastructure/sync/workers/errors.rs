use std::{error::Error, fmt};

use derivative::Derivative;

/**
 * All errors produced by sync worker
 */

#[derive(Derivative, Debug)]
pub struct RequestMethodNotSupported;
impl fmt::Display for RequestMethodNotSupported {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Request method is not supported for an http client!")
    }
}

impl Error for RequestMethodNotSupported {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncWorkerError {
    NoDataReceived,
    BuildRequestFailed(String),
    JsonParseFailed(String),
    WebRequestFailed(String),
    WebsocketConnectionFailed(String),
    ConnectionDroppedTimeout,
    OtherError(String),
    WorkerStopped,
    NoTaskAssigned,
    NoTaskReceived,
    SendTaskRequestFailed(String),
    CompleteTaskSendFailed,
    ResendTaskFailed,
    WSReadError(String),
    WSStreamDataSendFailed(String),
}
