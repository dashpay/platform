//! futures-related utilities to handle async code from sync code.

use std::future::Future;

use drive_proof_verifier::error::ContextProviderError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AsyncError {
    #[error("asynchronous call from synchronous context failed: {0}")]
    #[allow(unused)]
    Generic(String),
}

impl From<AsyncError> for ContextProviderError {
    fn from(error: AsyncError) -> Self {
        ContextProviderError::AsyncError(error.to_string())
    }
}

impl From<AsyncError> for crate::Error {
    fn from(error: AsyncError) -> Self {
        Self::ContextProviderError(error.into())
    }
}

/// Block on the provided future and return the result.
pub(crate) fn block_on<F: Future + Send + 'static>(fut: F) -> Result<F::Output, AsyncError>
where
    F::Output: Send,
{
    Ok(futures::executor::block_on(fut))
}
