#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EcsError {
    InvalidOperation(String),
    ObjectIsDestroyed,
    OutOfBounds,
    OutOfMemory,
    TypeDoesNotMatchId,
}

impl std::fmt::Display for EcsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EcsError::InvalidOperation(reason) => write!(f, "invalid operation: {}", reason),
            EcsError::ObjectIsDestroyed => write!(f, "object is destroyed"),
            EcsError::OutOfBounds => write!(f, "operation was out of bounds"),
            EcsError::OutOfMemory => write!(f, "out of memory"),
            EcsError::TypeDoesNotMatchId => write!(f, "type does not match the id"),
        }
    }
}

pub type EcsResult<T> = Result<T, EcsError>;

impl std::error::Error for EcsError {}