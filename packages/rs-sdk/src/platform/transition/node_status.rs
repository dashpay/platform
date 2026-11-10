//! What one node says it saw of a state transition, unproved: `getRecentStateTransitionStatus`.
//!
//! [`Sdk::recent_state_transition_status`] asks a node whether a transition is in its chain, in
//! its mempool, or unknown to it, and returns at once, where `waitForStateTransitionResult`
//! would block until a timeout. It re-broadcasts nothing and needs only the transition's hash.
//!
//! The answer is a hint. Mempools differ between nodes, anyone who saw the signed transition can
//! broadcast it again, and a node can answer anything, so "unknown" from one node does not mean
//! the transition will never land, and "committed" does not prove it did. A client uses it to
//! decide whether waiting longer makes sense, and to show the user something better than
//! "pending". Every decision that frees funds, retries a payment or reveals something private
//! waits for the proven [`TransitionOutcome`](super::outcome::TransitionOutcome) instead.
//!
//! # Privacy
//!
//! For shielded and private document transitions, ask only the node the transition was broadcast
//! to (an [`Sdk`] whose address list holds only that node). Asking several nodes about the same
//! transition spreads the link between the client's network address and the transition.

use crate::platform::ProtocolRequirement;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_recent_state_transition_status_request::{
    GetRecentStateTransitionStatusRequestV0, Version as RequestVersion,
};
use dapi_grpc::platform::v0::get_recent_state_transition_status_response::get_recent_state_transition_status_response_v0::Status;
use dapi_grpc::platform::v0::get_recent_state_transition_status_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetRecentStateTransitionStatusRequest, GetRecentStateTransitionStatusResponse,
    StateTransitionBroadcastError as StateTransitionBroadcastErrorProto,
};
use dpp::consensus::ConsensusError;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;
use rs_dapi_client::{DapiRequest, IntoInner, RequestSettings};

/// One node's unproved view of a state transition, from `getRecentStateTransitionStatus`.
///
/// # Never use this to decide anything about funds or privacy
///
/// Nothing here is proved. Do not free a reservation, unlock a held contact pair, retry a
/// payment or mark one failed on any of these answers: an [`NodeTransitionStatus::Unknown`]
/// transition may still land through another node, and a node can report
/// [`NodeTransitionStatus::Committed`] for a transition that is not. Only
/// [`TransitionOutcome`](super::outcome::TransitionOutcome), which
/// [`Sdk::state_transition_outcome`] proves, may drive those decisions. This type is for display
/// and for deciding whether waiting longer makes sense.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeTransitionStatus {
    /// In a block of the node's chain, as its transaction index reports it.
    Committed {
        /// The height of the block the node says holds it.
        height: u64,
        /// The error it failed with, when the node reports a result code other than 0.
        error: Option<NodeReportedError>,
    },
    /// In the node's mempool.
    Pending,
    /// Neither in the node's mempool nor in its chain. It may be in another node's mempool, may
    /// have been dropped, or may never have reached this node; a node that saw it and dropped it
    /// answers this too.
    Unknown,
}

/// The error a node reports for a committed transition whose result code is not 0, unproved.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeReportedError {
    /// The result code.
    pub code: u32,
    /// The node's message for it.
    pub message: String,
    /// The consensus error the node attached, when it decodes.
    pub cause: Option<Box<ConsensusError>>,
}

/// A node's answer about a transition, with the node's own latest height when it answered.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTransitionView {
    /// What the node says it saw. Unproved; see [`NodeTransitionStatus`].
    pub status: NodeTransitionStatus,
    /// The node's latest block height when it answered, as the node reports it (unproved), or
    /// `None` when it reported none.
    pub node_height: Option<u64>,
}

impl Sdk {
    /// Ask one node what it knows of the state transition with this hash, without waiting:
    /// committed, pending in its mempool, or unknown to it. Unproved; see
    /// [`NodeTransitionStatus`] for what it must not be used for.
    ///
    /// Nodes serve it from protocol version 15. Before the network is proven to run it, the call
    /// is refused with [`Error::ProtocolVersionNotActive`] without being sent.
    pub async fn recent_state_transition_status(
        &self,
        state_transition_hash: [u8; 32],
        settings: Option<RequestSettings>,
    ) -> Result<NodeTransitionView, Error> {
        let request = GetRecentStateTransitionStatusRequest {
            version: Some(RequestVersion::V0(
                GetRecentStateTransitionStatusRequestV0 {
                    state_transition_hash: state_transition_hash.to_vec(),
                },
            )),
        };
        self.ensure_network_serves(ProtocolRequirement::of_request(&request), settings)
            .await?;
        let settings = self
            .dapi_client_settings
            .override_by(settings.unwrap_or_default());
        let response = request
            .execute(self, settings)
            .await
            .into_inner()
            .map_err(Error::from)?;
        node_view_from_response(response)
    }

    /// [`Sdk::recent_state_transition_status`] for `transition`, by its hash.
    pub async fn recent_state_transition_status_of(
        &self,
        transition: &StateTransition,
        settings: Option<RequestSettings>,
    ) -> Result<NodeTransitionView, Error> {
        let hash = transition.transaction_id()?;
        self.recent_state_transition_status(hash, settings).await
    }
}

/// Read a node's answer.
fn node_view_from_response(
    response: GetRecentStateTransitionStatusResponse,
) -> Result<NodeTransitionView, Error> {
    let Some(ResponseVersion::V0(v0)) = response.version else {
        return Err(Error::Generic(
            "get_recent_state_transition_status answer has no version".to_string(),
        ));
    };
    let status = match v0.status {
        Some(Status::Committed(committed)) => NodeTransitionStatus::Committed {
            height: committed.height,
            error: committed.error.map(node_reported_error),
        },
        Some(Status::Pending(_)) => NodeTransitionStatus::Pending,
        Some(Status::Unknown(_)) => NodeTransitionStatus::Unknown,
        None => {
            return Err(Error::Generic(
                "get_recent_state_transition_status answer has no status".to_string(),
            ));
        }
    };
    Ok(NodeTransitionView {
        status,
        node_height: v0.metadata.map(|metadata| metadata.height),
    })
}

fn node_reported_error(error: StateTransitionBroadcastErrorProto) -> NodeReportedError {
    let cause = if error.data.is_empty() {
        None
    } else {
        ConsensusError::deserialize_from_bytes_untrusted(&error.data)
            .ok()
            .map(Box::new)
    };
    NodeReportedError {
        code: error.code,
        message: error.message,
        cause,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_recent_state_transition_status_response::get_recent_state_transition_status_response_v0::{
        Committed, Pending, Unknown,
    };
    use dapi_grpc::platform::v0::get_recent_state_transition_status_response::GetRecentStateTransitionStatusResponseV0;
    use dapi_grpc::platform::v0::ResponseMetadata;

    fn answer(
        status: Option<Status>,
        height: Option<u64>,
    ) -> GetRecentStateTransitionStatusResponse {
        GetRecentStateTransitionStatusResponse {
            version: Some(ResponseVersion::V0(
                GetRecentStateTransitionStatusResponseV0 {
                    status,
                    metadata: height.map(|height| ResponseMetadata {
                        height,
                        ..ResponseMetadata::default()
                    }),
                },
            )),
        }
    }

    #[test]
    fn should_read_a_committed_answer_with_its_height() {
        let view = node_view_from_response(answer(
            Some(Status::Committed(Committed {
                height: 120,
                error: None,
            })),
            Some(125),
        ))
        .expect("answer reads");
        assert_eq!(
            view,
            NodeTransitionView {
                status: NodeTransitionStatus::Committed {
                    height: 120,
                    error: None
                },
                node_height: Some(125),
            }
        );
    }

    #[test]
    fn should_keep_the_code_of_a_committed_transition_that_failed() {
        let view = node_view_from_response(answer(
            Some(Status::Committed(Committed {
                height: 9,
                error: Some(StateTransitionBroadcastErrorProto {
                    code: 40100,
                    message: "nonce already present".to_string(),
                    data: vec![0xff],
                }),
            })),
            None,
        ))
        .expect("answer reads");
        let NodeTransitionStatus::Committed {
            error: Some(error), ..
        } = view.status
        else {
            panic!("expected a committed failure, got {:?}", view.status);
        };
        assert_eq!(error.code, 40100);
        // Bytes that do not decode leave the cause out rather than failing the answer.
        assert_eq!(error.cause, None);
    }

    #[test]
    fn should_read_pending_and_unknown_answers() {
        let pending = node_view_from_response(answer(Some(Status::Pending(Pending {})), None))
            .expect("answer reads");
        assert_eq!(pending.status, NodeTransitionStatus::Pending);
        let unknown = node_view_from_response(answer(Some(Status::Unknown(Unknown {})), None))
            .expect("answer reads");
        assert_eq!(unknown.status, NodeTransitionStatus::Unknown);
    }

    #[test]
    fn should_refuse_an_answer_without_a_status() {
        assert!(node_view_from_response(answer(None, Some(1))).is_err());
        assert!(
            node_view_from_response(GetRecentStateTransitionStatusResponse::default()).is_err()
        );
    }
}
