use crate::transport::{TransportClient, TransportRequest};
use crate::{Address, CanRetry, DapiClientError, RequestSettings};
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::VersionedGrpcResponse;
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

/// Unwrap wrapped types
pub trait IntoInner<T> {
    /// Unwrap the inner type.
    ///
    /// This function returns inner type, dropping additional context information.
    /// It is lossy operation, so it should be used with caution.
    fn into_inner(self) -> T;
}

/// Convert inner type without loosing additional context information of the wrapper.
pub trait InnerInto<T> {
    /// Convert inner type without loosing additional context information of the wrapper.
    fn inner_into(self) -> T;
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

impl<F, T> InnerInto<ExecutionError<T>> for ExecutionError<F>
where
    F: Into<T>,
{
    /// Convert inner error type without loosing retries and address
    fn inner_into(self) -> ExecutionError<T> {
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
pub struct ExecutionResponse<R> {
    /// The response from the request
    pub inner: R,
    /// How many times the request was retried
    pub retries: usize,
    /// The address of the node that was used for the request
    pub address: Address,
}

#[cfg(feature = "mocks")]
impl<R: Default> Default for ExecutionResponse<R> {
    fn default() -> Self {
        Self {
            retries: Default::default(),
            address: "http://127.0.0.1".parse().expect("create mock address"),
            inner: Default::default(),
        }
    }
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

impl<F, T> InnerInto<ExecutionResponse<T>> for ExecutionResponse<F>
where
    F: Into<T>,
{
    /// Convert inner response type without loosing retries and address
    fn inner_into(self) -> ExecutionResponse<T> {
        ExecutionResponse {
            inner: self.inner.into(),
            retries: self.retries,
            address: self.address,
        }
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

impl<F, FE, T, TE> InnerInto<ExecutionResult<T, TE>> for ExecutionResult<F, FE>
where
    F: Into<T>,
    FE: Into<TE>,
{
    fn inner_into(self) -> ExecutionResult<T, TE> {
        match self {
            Ok(response) => Ok(response.inner_into()),
            Err(error) => Err(error.inner_into()),
        }
    }
}
