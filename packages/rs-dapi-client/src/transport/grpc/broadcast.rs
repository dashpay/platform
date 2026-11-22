//! Classification belongs to the broadcast transport, never to generic reads.
use crate::transport::{RequestErrorClassification, TransportError};
use crate::CanRetry;
use dapi_grpc::tonic::{Code, ConnectError};
use std::error::Error;

const PROCESSING_REJECTION: &str = "State Transition processing error. Please report faulty state transition and try to create a new state transition with different hash as a workaround.";

pub(super) fn classify_error(error: &TransportError) -> RequestErrorClassification {
    let TransportError::Grpc(status) = error;
    let definite = |retry| RequestErrorClassification {
        possibly_accepted: false,
        retry,
    };
    let uncertain = |retry| RequestErrorClassification {
        possibly_accepted: true,
        retry,
    };
    let mut source = status.source();
    while let Some(cause) = source {
        if cause.is::<ConnectError>() {
            return definite(error.can_retry());
        }
        source = cause.source();
    }
    // An application refusal ends sending, even under an unusual gRPC code.
    // It says nothing about earlier attempts, whose history the executor retains.
    if status
        .metadata()
        .contains_key("dash-serialized-consensus-error-bin")
    {
        return definite(false);
    }
    let message = status
        .message()
        .strip_prefix("Internal error: ")
        .unwrap_or(status.message());
    if status.code() == Code::Internal && message == PROCESSING_REJECTION {
        // A cache hit can precede mempool insertion; successful standalone CheckTx
        // does not rule out the original submission. Stop sending, but check its result.
        return uncertain(false);
    }
    if status.code() == Code::AlreadyExists {
        return uncertain(!matches!(
            message,
            "state transition already in mempool" | "state transition already in chain"
        ));
    }
    if status.code() == Code::ResourceExhausted
        && message == "broadcasting state transition is timed out"
    {
        return uncertain(error.can_retry());
    }
    if matches!(
        status.code(),
        Code::DeadlineExceeded
            | Code::Cancelled
            | Code::Unknown
            | Code::Internal
            | Code::Aborted
            | Code::DataLoss
            | Code::Unavailable
    ) {
        return uncertain(error.can_retry());
    }
    definite(error.can_retry())
}
