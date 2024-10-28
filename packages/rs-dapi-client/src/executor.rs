use crate::transport::{TransportClient, TransportRequest};
use crate::{Address, CanRetry, DapiClientError, RequestSettings};
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
use std::fmt::Debug;

#[async_trait]
/// DAPI client executor trait.
pub trait DapiRequestExecutor {
    /// Execute request using this DAPI client.
    async fn execute<R>(
        &self,
        request: R,
        settings: RequestSettings,
    ) -> ExecutionResult<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable,
        <R::Client as TransportClient>::Error: Mockable;
}

/// Error happened during request execution.
#[derive(Debug, Clone, thiserror::Error, Eq, PartialEq)]
#[error("{inner}")]
pub struct ExecutionError<E> {
    /// The cause of error
    pub inner: E,
    /// How many times the request was retried
    pub retries: usize,
    /// The address of the node that was used for the request
    pub address: Option<Address>,
}

impl<E> ExecutionError<E> {
    /// Unwrap the error cause
    pub fn into_inner(self) -> E {
        self.inner
    }
}

impl<E: CanRetry> CanRetry for ExecutionError<E> {
    fn can_retry(&self) -> bool {
        self.inner.can_retry()
    }
}

/// Request execution response.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecutionResponse<R> {
    /// The response from the request
    pub inner: R,
    /// How many times the request was retried
    pub retries: usize,
    /// The address of the node that was used for the request
    pub address: Address,
}

impl<R> ExecutionResponse<R> {
    /// Unwrap the response
    pub fn into_inner(self) -> R {
        self.inner
    }
}

/// Result of request execution
pub type ExecutionResult<R, E> = Result<ExecutionResponse<R>, ExecutionError<E>>;
