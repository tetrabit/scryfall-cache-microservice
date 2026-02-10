//! Structured error handling for API responses

pub mod codes;
pub mod response;

pub use codes::ErrorCode;
pub use response::{ErrorDetail, ErrorResponse};
