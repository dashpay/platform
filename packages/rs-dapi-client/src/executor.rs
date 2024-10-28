use crate::transport::{TransportClient, TransportRequest};
use crate::{Address, CanRetry, DapiClientError, RequestSettings};
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::VersionedGrpcResponse;
use dapi_grpc::tonic::async_trait;
use http_serde::http::Uri;
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

/// Unwrap wrapped types
pub trait IntoInner<T> {
    /// Unwrap the inner type
    fn into_inner(self) -> T;
}

/// Error happened during request execution.
#[derive(Debug, Clone, thiserror::Error, Eq, PartialEq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
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
    /// Convert inner error type without loosing retries and address
    pub fn into<F>(self) -> ExecutionError<F>
    where
        F: From<E>,
    {
        ExecutionError {
            inner: self.inner.into(),
            retries: self.retries,
            address: self.address,
        }
    }
}

impl<E, I> IntoInner<I> for ExecutionError<E>
where
    E: Into<I>,
{
    /// Unwrap the error cause
    fn into_inner(self) -> I {
        self.inner.into()
    }
}

impl<E: CanRetry> CanRetry for ExecutionError<E> {
    fn can_retry(&self) -> bool {
        self.inner.can_retry()
    }
}

/// Request execution response.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionResponse<R> {
    /// The response from the request
    pub inner: R,
    /// How many times the request was retried
    pub retries: usize,
    /// The address of the node that was used for the request
    pub address: Address,
}

impl<R, I> IntoInner<I> for ExecutionResponse<R>
where
    R: Into<I>,
{
    /// Unwrap the response
    fn into_inner(self) -> I {
        self.inner.into()
    }
}

impl<T: VersionedGrpcResponse> VersionedGrpcResponse for ExecutionResponse<T> {
    type Error = T::Error;

    fn metadata(&self) -> Result<&dapi_grpc::platform::v0::ResponseMetadata, Self::Error> {
        self.inner.metadata()
    }
    fn proof(&self) -> Result<&dapi_grpc::platform::v0::Proof, Self::Error> {
        self.inner.proof()
    }
    fn proof_owned(self) -> Result<dapi_grpc::platform::v0::Proof, Self::Error> {
        self.inner.proof_owned()
    }
}

impl<R> From<R> for ExecutionResponse<R> {
    fn from(inner: R) -> Self {
        Self {
            inner,
            retries: 0,
            address: Uri::default().into(),
        }
    }
}

/// Result of request execution
pub type ExecutionResult<R, E> = Result<ExecutionResponse<R>, ExecutionError<E>>;

impl<T, E> IntoInner<Result<T, E>> for ExecutionResult<T, E> {
    fn into_inner(self) -> Result<T, E> {
        match self {
            Ok(response) => Ok(response.into_inner()),
            Err(error) => Err(error.into_inner()),
        }
    }
}
