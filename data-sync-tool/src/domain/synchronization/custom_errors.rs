use derivative::Derivative;
use std::error::{self, Error};
use std::fmt::{self, Display, Formatter};
use url::ParseError;

#[derive(Debug)]
pub enum TaskCreationError {
    InsufficientArgError,
    InvalidRequestMethod,
    // We will defer to the parse error implementation for their error.
    // Supplying extra info requires adding more data to the type.
    UrlParseError(ParseError),
}

impl fmt::Display for TaskCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TaskCreationError::InsufficientArgError =>
                write!(f, "Please provide enough argument (data_endpoint, request_method, payload) to create a task! Perhaps the argument arrays passed don't have equal length?"),
            // The wrapped error contains additional information and is available
            // via the source() method.
            // TODO: Provide better error information
            TaskCreationError::InvalidRequestMethod =>
                write!(f, "The provided string could not be parsed as a valid request method."),
            TaskCreationError::UrlParseError(..) =>
                write!(f, "The provided string could not be parsed as an Url"),
        }
    }
}

impl error::Error for TaskCreationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            TaskCreationError::InsufficientArgError => None,
            TaskCreationError::InvalidRequestMethod => None,
            // The cause is the underlying implementation error type. Is implicitly
            // cast to the trait object `&error::Error`. This works because the
            // underlying type already implements the `Error` trait.
            TaskCreationError::UrlParseError(ref e) => Some(e),
        }
    }
}

// Implement the conversion from `ParseIntError` to `TaskCreationError`.
// This will be automatically called by `?` if a `ParseIntError`
// needs to be converted into a `TaskCreationError`.
impl From<ParseError> for TaskCreationError {
    fn from(err: ParseError) -> TaskCreationError {
        TaskCreationError::UrlParseError(err)
    }
}

/// Repsitory Errors
#[derive(Debug)]
pub enum RepositoryError {
    ItemNotFound,
    DuplicateItem,
    DatabaseConnectionFailed,
    DataSerializationFailed,
    PermissionDenied,
    // Other errors...
}

impl error::Error for RepositoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            RepositoryError::ItemNotFound => None,
            RepositoryError::DuplicateItem => None,
            RepositoryError::DatabaseConnectionFailed => None,
            RepositoryError::DataSerializationFailed => None,
            RepositoryError::PermissionDenied => None,
        }
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RepositoryError::ItemNotFound => f.write_str("Item not found"),
            RepositoryError::DuplicateItem => f.write_str("Duplicate item found"),
            RepositoryError::DatabaseConnectionFailed => {
                f.write_str("Failed to connect to the database")
            }
            RepositoryError::DataSerializationFailed => f.write_str("Failed to serialize data"),
            RepositoryError::PermissionDenied => f.write_str("Permission denied"),
        }
    }
}

#[derive(Debug)]
pub struct RequestMethodParsingError;

impl fmt::Display for RequestMethodParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "could not parse the provided string into a valid request method! Accepts: 'get', 'Get', 'GET', 'post', 'POST', 'Post'")
    }
}

impl error::Error for RequestMethodParsingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum TimerError {
    TimeCountFailedError,
    TimerAlreadyRunningError,
}

impl fmt::Display for TimerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TimerError::TimeCountFailedError => write!(f, "Failed to update time in the timer!"),
            // The wrapped error contains additional information and is available
            // via the source() method.
            // TODO: Provide better error information
            TimerError::TimerAlreadyRunningError => write!(f, "The timer is already running!"),
        }
    }
}

impl error::Error for TimerError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            TimerError::TimeCountFailedError => None,
            TimerError::TimerAlreadyRunningError => None,
        }
    }
}
