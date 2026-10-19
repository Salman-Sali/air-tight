use crate::error::ApiError;

/// Runs blocking work (diesel is synchronous) off the async executor.
pub async fn blocking<T, F>(work: F) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ApiError> + Send + 'static,
{
    tokio::task::spawn_blocking(work).await.map_err(|err| {
        eprintln!("blocking task failed: {err}");
        ApiError::Internal
    })?
}
