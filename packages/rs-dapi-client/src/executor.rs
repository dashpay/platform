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

/// Convert inner type without losing additional context information of the wrapper.
pub trait InnerInto<T> {
    /// Convert inner type without losing additional context information of the wrapper.
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
    /// Convert inner error type without losing retries and address
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

    fn is_no_available_addresses(&self) -> bool {
        self.inner.is_no_available_addresses()
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
    /// Convert inner response type without losing retries and address
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_address() -> Address {
        "http://127.0.0.1:3000".parse().expect("valid address")
    }

    fn mock_response() -> ExecutionResponse<i32> {
        ExecutionResponse {
            inner: 42,
            retries: 3,
            address: mock_address(),
        }
    }

    fn mock_error() -> ExecutionError<String> {
        ExecutionError {
            inner: "test error".to_string(),
            retries: 2,
            address: Some(mock_address()),
        }
    }

    #[test]
    fn test_execution_error_partial_eq() {
        let err1 = mock_error();
        let err2 = mock_error();
        assert_eq!(err1, err2);

        let err3 = ExecutionError {
            inner: "different".to_string(),
            retries: 2,
            address: Some(mock_address()),
        };
        assert_ne!(err1, err3);

        let err4 = ExecutionError {
            inner: "test error".to_string(),
            retries: 5,
            address: Some(mock_address()),
        };
        assert_ne!(err1, err4);
    }

    #[test]
    fn test_execution_result_from_response() {
        let response = mock_response();
        let result: ExecutionResult<i32, String> = response.into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().inner, 42);
    }

    #[test]
    fn test_execution_result_from_error() {
        let error = mock_error();
        let result: ExecutionResult<i32, String> = error.into();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().inner, "test error");
    }

    #[test]
    fn test_execution_result_inner_into_ok() {
        let response = mock_response();
        let result: ExecutionResult<i32, String> = Ok(response);
        let converted: ExecutionResult<i64, String> = result.inner_into();
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap().inner, 42i64);
    }

    #[test]
    fn test_execution_result_inner_into_err() {
        let error = mock_error();
        let result: ExecutionResult<i32, String> = Err(error);
        let converted: ExecutionResult<i64, String> = result.inner_into();
        assert!(converted.is_err());
        assert_eq!(converted.unwrap_err().inner, "test error");
    }

    #[test]
    fn test_wrap_to_execution_result_ok() {
        let context = mock_response();
        let inner_result: Result<i64, String> = Ok(100);
        let wrapped: ExecutionResult<i64, String> = inner_result.wrap_to_execution_result(&context);

        let response = wrapped.unwrap();
        assert_eq!(response.inner, 100);
        assert_eq!(response.retries, 3);
        assert_eq!(response.address, mock_address());
    }

    #[test]
    fn test_wrap_to_execution_result_err() {
        let context = mock_response();
        let inner_result: Result<i64, String> = Err("wrapped error".to_string());
        let wrapped: ExecutionResult<i64, String> = inner_result.wrap_to_execution_result(&context);

        let error = wrapped.unwrap_err();
        assert_eq!(error.inner, "wrapped error");
        assert_eq!(error.retries, 3);
        assert_eq!(error.address, Some(mock_address()));
    }

    #[test]
    fn test_execution_response_into_inner() {
        let response = mock_response();
        let inner: i32 = response.into_inner();
        assert_eq!(inner, 42);
    }

    #[test]
    fn test_execution_response_inner_into() {
        let response = mock_response();
        let converted: ExecutionResponse<i64> = response.inner_into();
        assert_eq!(converted.inner, 42i64);
        assert_eq!(converted.retries, 3);
    }

    #[test]
    fn test_execution_error_into_inner() {
        let error = mock_error();
        let inner: String = error.into_inner();
        assert_eq!(inner, "test error");
    }

    #[test]
    fn test_execution_error_inner_into() {
        let error = mock_error();
        let converted: ExecutionError<String> = error.inner_into();
        assert_eq!(converted.inner, "test error");
        assert_eq!(converted.retries, 2);
    }

    #[test]
    fn test_execution_result_into_inner_ok() {
        let result: ExecutionResult<i32, String> = Ok(mock_response());
        let inner: Result<i32, String> = result.into_inner();
        assert_eq!(inner.unwrap(), 42);
    }

    #[test]
    fn test_execution_result_into_inner_err() {
        let result: ExecutionResult<i32, String> = Err(mock_error());
        let inner: Result<i32, String> = result.into_inner();
        assert_eq!(inner.unwrap_err(), "test error");
    }

    #[test]
    fn test_execution_error_can_retry_delegates() {
        // Test that ExecutionError's CanRetry impl delegates correctly
        // We need a type that implements CanRetry
        use crate::DapiClientError;

        let inner = DapiClientError::NoAvailableAddresses;
        let error = ExecutionError {
            inner,
            retries: 0,
            address: None,
        };

        assert!(!error.can_retry());
        assert!(error.is_no_available_addresses());
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_execution_response_default() {
        let response: ExecutionResponse<i32> = ExecutionResponse::default();
        assert_eq!(response.inner, 0);
        assert_eq!(response.retries, 0);
    }
}
