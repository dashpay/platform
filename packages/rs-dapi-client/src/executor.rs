use crate::transport::TransportRequest;
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
    ) -> ExecutionResult<R::Response, DapiClientError>
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable;
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
#[derive(Debug, Clone, thiserror::Error, Eq)]
#[error("{inner}")]
pub struct ExecutionError<E> {
    /// The cause of error
    pub inner: E,
    /// How many times the request was retried
    pub retries: usize,
    /// The address of the node that was used for the request
    pub address: Option<Address>,
}

impl<E: PartialEq> PartialEq for ExecutionError<E> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self.retries == other.retries && self.address == other.address
    }
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

/// Result of request execution
pub type ExecutionResult<R, E> = Result<ExecutionResponse<R>, ExecutionError<E>>;

impl<R, E> From<ExecutionResponse<R>> for ExecutionResult<R, E> {
    fn from(response: ExecutionResponse<R>) -> Self {
        ExecutionResult::<R, E>::Ok(response)
    }
}

impl<R, E> From<ExecutionError<E>> for ExecutionResult<R, E> {
    fn from(e: ExecutionError<E>) -> Self {
        ExecutionResult::<R, E>::Err(e)
    }
}

impl<R, E> IntoInner<Result<R, E>> for ExecutionResult<R, E> {
    fn into_inner(self) -> Result<R, E> {
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

/// Convert Result<T,TE> to ExecutionResult<R,E>, taking context from ExecutionResponse.
pub trait WrapToExecutionResult<R, RE, W>: Sized {
    /// Convert self (eg. some [Result]) to [ExecutionResult], taking context information from `W` (eg. ExecutionResponse).
    ///
    /// This function simplifies processing of results by wrapping them into ExecutionResult.
    /// It is useful when you have execution result retrieved in previous step and you want to
    /// add it to the result of the current step.
    ///
    /// Useful when chaining multiple commands and you want to keep track of retries and address.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use rs_dapi_client::{ExecutionResponse, ExecutionResult, WrapToExecutionResult};
    ///
    /// fn some_request() -> ExecutionResult<i8, String> {
    ///     Ok(ExecutionResponse {
    ///         inner: 42,
    ///         retries: 123,
    ///         address: "http://127.0.0.1".parse().expect("create mock address"),
    ///     })
    /// }
    ///
    /// fn next_step() -> Result<i32, String> {
    ///     Err("next error".to_string())
    /// }
    ///
    /// let response = some_request().expect("request should succeed");
    /// let result: ExecutionResult<i32, String> = next_step().wrap_to_execution_result(&response);
    ///
    /// if let ExecutionResult::Err(error) = result {
    ///    assert_eq!(error.inner, "next error");
    ///    assert_eq!(error.retries, 123);
    /// } else {
    ///    panic!("Expected error");
    /// }
    /// ```
    fn wrap_to_execution_result(self, result: &W) -> ExecutionResult<R, RE>;
}

impl<R, RE, TR, IR, IRE> WrapToExecutionResult<R, RE, ExecutionResponse<TR>> for Result<IR, IRE>
where
    R: From<IR>,
    RE: From<IRE>,
{
    fn wrap_to_execution_result(self, result: &ExecutionResponse<TR>) -> ExecutionResult<R, RE> {
        match self {
            Ok(r) => ExecutionResult::Ok(ExecutionResponse {
                inner: r.into(),
                retries: result.retries,
                address: result.address.clone(),
            }),
            Err(e) => ExecutionResult::Err(ExecutionError {
                inner: e.into(),
                retries: result.retries,
                address: Some(result.address.clone()),
            }),
        }
    }
}
