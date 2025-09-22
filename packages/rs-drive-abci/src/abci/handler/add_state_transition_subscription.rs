use crate::abci::handler::error::HandlerError;
use crate::error::Error;
use crate::metrics::LABEL_ABCI_RESPONSE_CODE;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use drive::drive::subscriptions::document_filter::DriveDocumentQueryFilter;
use metrics::Label;

/// Request data for adding a state transition subscription.
///
/// This is a temporary stand-in until the Tenderdash protobuf definitions are finalized.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AddStateTransitionSubscriptionRequest {
    /// Identifier of the client issuing the subscription request (e.g. Tenderdash peer ID).
    pub client_id: String,
    /// Caller-provided label that will be echoed back in the response.
    pub subscription_label: String,
    /// Optional document-filter configuration describing which transitions to deliver.
    pub filter: Option<DriveDocumentQueryFilter>,
}

impl AddStateTransitionSubscriptionRequest {
    /// Create a new placeholder request.
    #[allow(dead_code)]
    pub fn new(
        client_id: impl Into<String>,
        subscription_label: impl Into<String>,
        filter: Option<DriveDocumentQueryFilter>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            subscription_label: subscription_label.into(),
            filter,
        }
    }
}

/// Response returned from the placeholder subscription handler.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AddStateTransitionSubscriptionResponse {
    /// ABCI-style response code (0 indicates success).
    pub code: u32,
    /// Base64-encoded error information (empty on success).
    pub info: String,
    /// Identifier of the client that initiated the request.
    pub client_id: String,
    /// Echoed subscription label.
    pub subscription_label: String,
    /// Whether the subscription was accepted.
    pub accepted: bool,
}

impl AddStateTransitionSubscriptionResponse {
    #[allow(dead_code)]
    fn from_error(
        error: HandlerError,
        request: &AddStateTransitionSubscriptionRequest,
    ) -> Result<Self, Error> {
        Ok(Self {
            code: error.code(),
            info: error.response_info()?,
            client_id: request.client_id.clone(),
            subscription_label: request.subscription_label.clone(),
            accepted: false,
        })
    }
}

/// Handle a client request to subscribe for state transition notifications.
///
/// The underlying protocol support is not yet implemented; this handler currently validates
/// the provided filter (if any) and returns an `Unimplemented` response. The structure mirrors
/// other ABCI handlers so the real implementation can be plugged in once the protobuf schema
/// lands in Tenderdash.
#[allow(dead_code)]
pub fn add_state_transition_subscription<C>(
    platform: &Platform<C>,
    request: AddStateTransitionSubscriptionRequest,
) -> Result<AddStateTransitionSubscriptionResponse, Error>
where
    C: CoreRPCLike,
{
    let mut timer = crate::metrics::abci_request_duration("add_state_transition_subscription");

    // Touch the platform so we do not trigger unused warnings before the implementation lands.
    let _existing_subscriptions = platform.state_transition_subscriptions.len();

    let filter = match request.filter.as_ref() {
        Some(filter) => filter,
        None => {
            let handler_error = HandlerError::InvalidArgument(
                "missing state transition subscription filter".to_string(),
            );
            timer.add_label(Label::new(
                LABEL_ABCI_RESPONSE_CODE,
                handler_error.code().to_string(),
            ));
            return AddStateTransitionSubscriptionResponse::from_error(handler_error, &request);
        }
    };

    let validation = filter.validate();
    if !validation.is_valid() {
        let aggregated_errors = validation
            .errors
            .into_iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("; ");

        let handler_error = HandlerError::InvalidArgument(format!(
            "invalid state transition subscription filter: {aggregated_errors}",
        ));
        timer.add_label(Label::new(
            LABEL_ABCI_RESPONSE_CODE,
            handler_error.code().to_string(),
        ));
        return AddStateTransitionSubscriptionResponse::from_error(handler_error, &request);
    }

    // Until protobuf definitions and subscription plumbing exist, signal to the caller that
    // the operation is not yet available.
    let handler_error = HandlerError::Unimplemented(
        "state transition subscriptions are not yet supported".to_string(),
    );
    timer.add_label(Label::new(
        LABEL_ABCI_RESPONSE_CODE,
        handler_error.code().to_string(),
    ));

    AddStateTransitionSubscriptionResponse::from_error(handler_error, &request)
}
