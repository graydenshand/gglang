/// Structured error types for the gglang engine.
#[derive(Debug, thiserror::Error)]
pub enum GglangError {
    #[error("Parse error: {message}")]
    Parse { message: String },

    #[error("Compile error: {message}")]
    Compile { message: String },

    #[error("Data error: {message}")]
    Data { message: String },

    #[error("Render error: {message}")]
    Render { message: String },

    #[error("Export error: {message}")]
    Export { message: String },
}
