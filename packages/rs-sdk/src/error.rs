//! Definitions of errors
use crate::platform::encrypted_for::EncryptedForError;
use dapi_grpc::platform::v0::StateTransitionBroadcastError as StateTransitionBroadcastErrorProto;
use dapi_grpc::tonic::Code;
pub use dash_context_provider::ContextProviderError;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::consensus::basic::state_transition::{
    OutputBelowMinimumError, TransitionNoInputsError, TransitionNoOutputsError,
};
use dpp::consensus::state::address_funds::{AddressDoesNotExistError, AddressNotEnoughFundsError};
use dpp::consensus::ConsensusError;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersionError;
use dpp::{dashcore_rpc, ProtocolError};
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::{CanRetry, DapiClientError, ExecutionError};
use std::fmt::Debug;
use std::time::Duration;

/// Error type for the SDK
// TODO: Propagate server address and retry information so that the user can retrieve it
#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// SDK is not configured properly
    #[error("SDK misconfigured: {0}")]
    Config(String),
    /// Drive error
    #[error("Drive error: {0}")]
    Drive(#[from] drive::error::Error),
    /// Drive proof error with associated proof bytes and block info
    #[error("Drive error with associated proof: {0}")]
    DriveProofError(drive::error::proof::ProofError, Vec<u8>, BlockInfo),
    /// DPP error
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    /// Proof verification error
    #[error("Proof verification error: {0}")]
    Proof(#[source] drive_proof_verifier::Error),
    /// Invalid Proved Response error
    #[error("Invalid Proved Response error: {0}")]
    InvalidProvedResponse(String),
    /// The proof authenticated only the state the transition affects (a
    /// height-pinned snapshot), while the caller required evidence that this
    /// specific transition executed. Use the `*_affected_state` wait APIs to
    /// accept snapshot outcomes explicitly.
    #[error("proof authenticates the transition's affected state only, not its execution: {0}")]
    ExecutionNotProved(String),
    /// DAPI client error, for example, connection error
    #[error("Dapi client error: {0}")]
    DapiClientError(rs_dapi_client::DapiClientError),
    #[cfg(feature = "mocks")]
    /// DAPI mocks error
    #[error("Dapi mocks error: {0}")]
    DapiMocksError(#[from] rs_dapi_client::mock::MockError),
    /// Dash core error
    #[error("Dash core error: {0}")]
    CoreError(#[from] dpp::dashcore::Error),
    /// MerkleBlockError
    #[error("Dash core error: {0}")]
    MerkleBlockError(#[from] dpp::dashcore::merkle_tree::MerkleBlockError),
    /// Core client error, for example, connection error
    #[error("Core client error: {0}")]
    CoreClientError(#[from] dashcore_rpc::Error),
    /// Dependency not found, for example data contract for a document not found
    #[error("Required {0} not found: {1}")]
    MissingDependency(String, String),
    /// Total credits in Platform are not found; we must always have credits in Platform
    #[error("Total credits in Platform are not found; it should never happen")]
    TotalCreditsNotFound,
    /// Epoch not found; we must have at least one epoch
    #[error("No epoch found on Platform; it should never happen")]
    EpochNotFound,
    /// SDK operation timeout reached error
    #[error("SDK operation timeout {} secs reached: {}", .0.as_secs(), .1)]
    TimeoutReached(Duration, String),

    /// Returned when an attempt is made to create an object that already exists in the system
    #[error("Object already exists: {0}")]
    AlreadyExists(String),
    /// Invalid credit transfer configuration
    #[error("Invalid credit transfer: {0}")]
    InvalidCreditTransfer(String),
    /// Identity nonce overflow: the nonce has reached its maximum value and
    /// cannot be incremented further without wrapping to zero.
    #[error("Identity nonce overflow: nonce has reached the maximum value ({0})")]
    NonceOverflow(u64),
    /// Identity nonce not found on Platform.
    ///
    /// Platform returned no nonce for the requested identity (or identity–
    /// contract pair).  This usually means the queried DAPI node has not yet
    /// indexed the identity — for example right after identity creation or
    /// when the node is lagging behind the chain tip.
    ///
    /// **Recovery**: retry the state transition; the SDK will re-fetch the
    /// nonce from a (potentially different) DAPI node on the next attempt.
    #[error("Identity nonce not found on platform: {0}")]
    IdentityNonceNotFound(String),

    /// Drive returned an internal error that is not a consensus error.
    ///
    /// Contains the decoded human-readable message extracted from the
    /// `drive-error-data-bin` gRPC metadata (CBOR map, `message` field).
    /// Typically a storage-level failure (e.g., GroveDB constraint violation).
    #[error("Drive internal error: {0}")]
    DriveInternalError(String),

    /// Generic error
    // TODO: Use domain specific errors instead of generic ones
    #[error("SDK error: {0}")]
    Generic(String),

    /// Context provider error
    #[error("Context provider error: {0}")]
    ContextProviderError(#[from] ContextProviderError),

    /// Operation cancelled - cancel token was triggered, timeout, etc.
    #[error("Operation cancelled: {0}")]
    Cancelled(String),

    /// Remote node is stale; try another server
    #[error(transparent)]
    StaleNode(#[from] StaleNodeError),

    /// Error returned when trying to broadcast a state transition
    #[error(transparent)]
    StateTransitionBroadcastError(#[from] StateTransitionBroadcastError),

    /// All available addresses have been exhausted (banned due to errors).
    /// Contains the last meaningful error that caused addresses to be banned.
    #[error("no available addresses to retry, last error: {0}")]
    NoAvailableAddressesToRetry(Box<Error>),

    /// A property declared `encryptedFor` could not be encrypted or decrypted
    #[error(transparent)]
    EncryptedFor(#[from] EncryptedForError),

    /// The request uses a feature that not every node of the network serves yet, so the
    /// SDK refused it without sending it: nodes of an older release would answer
    /// UNIMPLEMENTED or a different question, and get banned for it.
    ///
    /// Retrying another node cannot help. The same request succeeds once a verified answer
    /// shows the network runs `required`, and this SDK's own version is at least
    /// `required`. See [`TransportRequest::needs_protocol_version`](rs_dapi_client::transport::TransportRequest::needs_protocol_version).
    #[error(
        "{method} needs Platform protocol version {required}; {}",
        describe_proven_protocol_version(*.network, *.pinned, .cause.is_some())
    )]
    ProtocolVersionNotActive {
        /// gRPC method of the refused request, e.g. `get_contract_fee_pots`.
        method: &'static str,
        /// Protocol version from which every node serves the request.
        required: u32,
        /// Protocol version the SDK could rely on: the lower of its own version and the
        /// newest version a verified answer was signed with; 0 before any verified answer.
        network: u32,
        /// Whether the SDK is pinned to a version below `required`
        /// ([`SdkBuilder::with_version`](crate::SdkBuilder::with_version)): then no answer
        /// can lift the refusal, whatever the network runs.
        pinned: bool,
        /// Why the network's version could not be confirmed, when a query for it failed.
        /// The refusal may then be transient.
        #[source]
        cause: Option<Box<Error>>,
    },
}

/// The end of [`Error::ProtocolVersionNotActive`]'s message.
fn describe_proven_protocol_version(
    network: u32,
    pinned: bool,
    confirmation_failed: bool,
) -> String {
    if pinned {
        "this SDK is pinned to an earlier protocol version".to_string()
    } else if confirmation_failed {
        "the network's protocol version could not be confirmed".to_string()
    } else if network == 0 {
        "the network's protocol version is not verified yet".to_string()
    } else {
        format!("the network is proven at protocol version {network}")
    }
}

impl Error {
    /// Whether this is an [`Error::ProtocolVersionNotActive`] refusal that may lift soon: the
    /// SDK could not learn the network's version for a reason that can pass, such as nodes
    /// or a quorum source not answering, so the network may serve the request already.
    ///
    /// False for any other error, for a refusal the network's verified version explains,
    /// and for one caused by this SDK's own setup: no context provider, or one configured so
    /// that it cannot give quorum keys at all ([`ContextProviderError::Config`]). Retrying
    /// cannot change those. A provider's other failures, such as a quorum service not
    /// answering, may pass.
    pub fn is_transient_protocol_version_refusal(&self) -> bool {
        match self {
            Self::ProtocolVersionNotActive {
                cause: Some(cause), ..
            } => !cause.cannot_verify_answers(),
            _ => false,
        }
    }

    /// Whether this SDK's own setup keeps it from verifying any answer, looking through
    /// retry-exhaustion wrappers.
    fn cannot_verify_answers(&self) -> bool {
        match self {
            Self::NoAvailableAddressesToRetry(last_error) => last_error.cannot_verify_answers(),
            Self::Proof(drive_proof_verifier::Error::ContextProviderNotSet) => true,
            Self::Proof(drive_proof_verifier::Error::QuorumKeyUnavailable { error, .. })
            | Self::ContextProviderError(error) => {
                matches!(error, ContextProviderError::Config(_))
            }
            _ => false,
        }
    }

    /// Whether this error is a trusted quorum-source failure, looking through
    /// nested retry-exhaustion wrappers. This is distinct from
    /// [`CanRetry::can_retry`] and does not affect node-ban attribution.
    pub fn is_quorum_source_unavailable(&self) -> bool {
        match self {
            Self::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(_)) => true,
            Self::NoAvailableAddressesToRetry(last_error) => {
                last_error.is_quorum_source_unavailable()
            }
            _ => false,
        }
    }

    /// Whether the node that sent the answer leaves the request's rotation without a ban:
    /// the answer cannot be used, but does not show that the node is at fault.
    pub(crate) fn spares_responder(&self) -> bool {
        matches!(
            self,
            Self::StaleNode(StaleNodeError::ProtocolVersion { .. })
        )
    }

    /// The error a request reports when every one of its attempts ended with this kind of
    /// error and no attempt is left.
    ///
    /// When every node asked answered a document query at an older protocol version than the
    /// SDK's, while no verified answer has shown the network running the SDK's version, the
    /// request fails as a configuration error: the SDK may be set ahead of its network.
    pub(crate) fn into_exhausted_error(self) -> Self {
        match self {
            Self::StaleNode(StaleNodeError::ProtocolVersion {
                received_protocol_version: signed,
                expected_protocol_version: sdk,
            }) => Self::Config(format!(
                "the last answer names protocol version {signed}, below the SDK's protocol \
                 version {sdk}, and no verified answer has yet shown the network running \
                 {sdk}; if the SDK was set ahead of its network, let it follow the network's \
                 protocol version instead, or seed it with the network's version \
                 (with_initial_version)"
            )),
            error => error,
        }
    }
}

/// A trusted source outage is not the responding node's fault. Preserve that
/// attribution and quorum context; every other verifier failure stays `Proof`.
impl From<drive_proof_verifier::Error> for Error {
    fn from(value: drive_proof_verifier::Error) -> Self {
        match value {
            drive_proof_verifier::Error::QuorumKeyUnavailable {
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
                error: ContextProviderError::QuorumSourceUnavailable(reason),
            } => Self::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(format!(
                "quorum {quorum_type}:{} at Core chain-locked height {core_chain_locked_height}: {reason}",
                hex::encode(quorum_hash)
            ))),
            error => Self::Proof(error),
        }
    }
}

impl From<dash_platform_queries::Error> for Error {
    fn from(value: dash_platform_queries::Error) -> Self {
        match value {
            dash_platform_queries::Error::Config(msg) => Self::Config(msg),
            dash_platform_queries::Error::Drive(e) => Self::Drive(e),
            dash_platform_queries::Error::Protocol(e) => Self::Protocol(e),
        }
    }
}

/// State transition broadcast error
#[derive(Debug, thiserror::Error)]
#[error("state transition broadcast error: {message}")]
pub struct StateTransitionBroadcastError {
    /// Error code
    pub code: u32,
    /// Error message
    pub message: String,
    /// Consensus error caused the state transition broadcast error
    pub cause: Option<ConsensusError>,
}

impl TryFrom<StateTransitionBroadcastErrorProto> for StateTransitionBroadcastError {
    type Error = Error;

    fn try_from(value: StateTransitionBroadcastErrorProto) -> Result<Self, Self::Error> {
        let cause = if !value.data.is_empty() {
            let consensus_error = ConsensusError::deserialize_from_bytes_untrusted(&value.data)
                .map_err(|e| {
                    tracing::debug!("Failed to deserialize consensus error: {}", e);

                    Error::Protocol(e)
                })?;

            Some(consensus_error)
        } else {
            None
        };

        Ok(Self {
            code: value.code,
            message: value.message,
            cause,
        })
    }
}

// TODO: Decompose DapiClientError to more specific errors like connection, node error instead of DAPI client error
impl From<DapiClientError> for Error {
    fn from(value: DapiClientError) -> Self {
        if let DapiClientError::Transport(TransportError::Grpc(status)) = &value {
            // If we have some consensus error metadata, we deserialize it and return as ConsensusError
            if let Some(consensus_error_value) = status
                .metadata()
                .get_bin("dash-serialized-consensus-error-bin")
            {
                return consensus_error_value
                    .to_bytes()
                    .map(|bytes| {
                        ConsensusError::deserialize_from_bytes_untrusted(&bytes)
                            .map(|consensus_error| {
                                Self::Protocol(ProtocolError::ConsensusError(Box::new(
                                    consensus_error,
                                )))
                            })
                            .unwrap_or_else(|e| {
                                tracing::debug!("Failed to deserialize consensus error: {}", e);
                                Self::Protocol(e)
                            })
                    })
                    .unwrap_or_else(|e| {
                        tracing::debug!("Failed to deserialize consensus error: {}", e);
                        // TODO: Introduce a specific error for this case
                        Self::Generic(format!("Invalid consensus error encoding: {e}"))
                    });
            }
            // Check drive-error-data-bin for decoded Drive error messages
            if status.code() == Code::Internal {
                if let Some(drive_error_value) = status.metadata().get_bin("drive-error-data-bin") {
                    match drive_error_value.to_bytes() {
                        Ok(bytes) => {
                            if let Some(message) = extract_drive_error_message(&bytes) {
                                return Self::DriveInternalError(message);
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
                                "Failed to decode drive-error-data-bin metadata: {}",
                                e
                            );
                        }
                    }
                }
            }

            // Otherwise we parse the error code and act accordingly
            if status.code() == Code::AlreadyExists {
                return Self::AlreadyExists(status.message().to_string());
            }
        }

        // Preserve the original DAPI client error for structured inspection
        Self::DapiClientError(value)
    }
}

/// Hard cap on the length of attacker-influenceable CBOR payloads accepted
/// before decoding the `drive-error-data-bin` gRPC metadata.
///
/// gRPC metadata is conventionally bounded around 8 KiB; 64 KiB is comfortably
/// above any legitimate payload. The cap bounds memory only — `ciborium`'s
/// own recursion limit (256) bounds nesting depth and returns
/// `RecursionLimitExceeded` rather than recursing into the stack.
const MAX_CBOR_INPUT_SIZE: usize = 65_536;

// `ciborium` caps recursion at depth 256 and returns
// `Error::RecursionLimitExceeded` (a normal `Err`, not a panic) for deeper
// input, so a hostile peer cannot exhaust the stack here.
fn decode_cbor_value(bytes: &[u8]) -> Option<ciborium::Value> {
    ciborium::from_reader::<ciborium::Value, _>(bytes).ok()
}

/// Extract the `message` text from CBOR-encoded `drive-error-data-bin` metadata.
///
/// The metadata is a CBOR map with optional fields `code`, `message`,
/// `consensus_error`. Returns `Some(message)` when a non-empty `message`
/// text is present. Inputs larger than [`MAX_CBOR_INPUT_SIZE`] are rejected
/// unread.
//
// MIRROR: keep in sync with `walk_cbor_for_key` in
// packages/rs-dapi/src/services/platform_service/error_mapping.rs.
fn extract_drive_error_message(bytes: &[u8]) -> Option<String> {
    if bytes.len() > MAX_CBOR_INPUT_SIZE {
        tracing::debug!(
            len = bytes.len(),
            max = MAX_CBOR_INPUT_SIZE,
            "drive-error-data-bin exceeds size cap; refusing to decode"
        );
        return None;
    }
    let value = decode_cbor_value(bytes)?;
    let map = value.as_map()?;
    for (key, val) in map {
        if key.as_text() == Some("message") {
            if let Some(msg) = val.as_text() {
                if !msg.is_empty() {
                    return Some(msg.to_string());
                }
            }
        }
    }
    None
}

impl From<PlatformVersionError> for Error {
    fn from(value: PlatformVersionError) -> Self {
        Self::Protocol(value.into())
    }
}

impl From<ConsensusError> for Error {
    fn from(value: ConsensusError) -> Self {
        Self::Protocol(ProtocolError::ConsensusError(Box::new(value)))
    }
}

impl From<TransitionNoInputsError> for Error {
    fn from(value: TransitionNoInputsError) -> Self {
        Self::Protocol(ProtocolError::ConsensusError(Box::new(value.into())))
    }
}

impl From<TransitionNoOutputsError> for Error {
    fn from(value: TransitionNoOutputsError) -> Self {
        Self::Protocol(ProtocolError::ConsensusError(Box::new(value.into())))
    }
}

impl From<OutputBelowMinimumError> for Error {
    fn from(value: OutputBelowMinimumError) -> Self {
        Self::Protocol(ProtocolError::ConsensusError(Box::new(value.into())))
    }
}

impl From<SimpleConsensusValidationResult> for Error {
    fn from(value: SimpleConsensusValidationResult) -> Self {
        value
            .errors
            .into_iter()
            .next()
            .map(Error::from)
            .unwrap_or_else(|| {
                Error::Protocol(ProtocolError::CorruptedCodeExecution(
                    "state transition structure validation failed without an error".to_string(),
                ))
            })
    }
}

impl From<AddressDoesNotExistError> for Error {
    fn from(value: AddressDoesNotExistError) -> Self {
        Self::Protocol(ProtocolError::ConsensusError(Box::new(value.into())))
    }
}

impl From<AddressNotEnoughFundsError> for Error {
    fn from(value: AddressNotEnoughFundsError) -> Self {
        Self::Protocol(ProtocolError::ConsensusError(Box::new(value.into())))
    }
}

// Retain legacy behavior for generic execution errors that are not DapiClientError
impl<T> From<ExecutionError<T>> for Error
where
    ExecutionError<T>: ToString,
{
    fn from(value: ExecutionError<T>) -> Self {
        // Fallback to a generic string representation
        Self::Generic(value.to_string())
    }
}

impl CanRetry for Error {
    fn can_retry(&self) -> bool {
        matches!(
            self,
            Error::StaleNode(..) | Error::TimeoutReached(_, _) | Error::Proof(_)
        )
    }

    fn is_no_available_addresses(&self) -> bool {
        matches!(
            self,
            Error::DapiClientError(DapiClientError::NoAvailableAddresses)
                | Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(_))
        )
    }
}

/// Server returned stale metadata
#[derive(Debug, thiserror::Error)]
pub enum StaleNodeError {
    /// Server returned metadata with outdated height
    #[error("received height is outdated: expected {expected_height}, received {received_height}, tolerance {tolerance_blocks}; try another server")]
    Height {
        /// Expected height - last block height seen by the Sdk
        expected_height: u64,
        /// Block height received from the server
        received_height: u64,
        /// Tolerance - how many blocks can be behind the expected height
        tolerance_blocks: u64,
    },
    /// Server returned metadata with time outside of the tolerance
    #[error(
        "received invalid time: expected {expected_timestamp_ms}ms, received {received_timestamp_ms} ms, tolerance {tolerance_ms} ms; try another server"
    )]
    Time {
        /// Expected time in milliseconds - is local time when the message was received
        expected_timestamp_ms: u64,
        /// Time received from the server in the message, in milliseconds
        received_timestamp_ms: u64,
        /// Tolerance in milliseconds
        tolerance_ms: u64,
    },
    /// Server kept reporting a current epoch that its own proofs contradict
    ///
    /// The epoch index in response metadata is not covered by the quorum
    /// signature, so `ExtendedEpochInfo::fetch_current` only uses it to shape a
    /// proved query and then checks it against the proof. This error means the
    /// check kept failing: every proof showed a newer epoch already started.
    #[error("received epoch is outdated: hinted {hinted_epoch}, proven started epoch {proven_epoch}; try another server")]
    Epoch {
        /// Epoch index the server reported as current in unsigned response metadata
        hinted_epoch: EpochIndex,
        /// Newer epoch index that the server's own proof showed as already started
        proven_epoch: EpochIndex,
    },
    /// Server answered a document query at an older protocol version than the SDK's, and no
    /// verified answer has yet shown the network running the SDK's version
    ///
    /// A document query answer is not verified with the rules of another protocol version than
    /// the one that signed it. The SDK may be set ahead of its network, where every node answers
    /// this way, so the node is not banned: the request moves to another node. A request whose
    /// every attempt ends with this error fails with [`Error::Config`] instead, also when no
    /// node is left to ask.
    #[error("received protocol version {received_protocol_version} is below the SDK's protocol version {expected_protocol_version}; try another server")]
    ProtocolVersion {
        /// Protocol version the SDK verifies the query with
        expected_protocol_version: u32,
        /// Protocol version named in the server's response metadata
        received_protocol_version: u32,
    },
    /// Server answered a request that needs a newer protocol version with a response
    /// signed at an older one: it is behind the network's protocol upgrade.
    ///
    /// Such a request is sent only once a verified answer has shown the network running
    /// `required`, so unlike [`StaleNodeError::ProtocolVersion`] the node is at fault: it is
    /// banned, and the request moves to another node.
    #[error("received an answer signed at protocol version {signed} to a request that needs {required}; try another server")]
    BehindProtocolUpgrade {
        /// Protocol version from which every node serves the request
        required: u32,
        /// Protocol version in the response metadata
        signed: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the typed source failure, including nested retry exhaustion,
    /// receives source attribution; sibling variants and messages do not.
    #[test]
    fn should_identify_only_typed_quorum_source_failures_through_retry_exhaustion() {
        let mut source = Error::ContextProviderError(
            ContextProviderError::QuorumSourceUnavailable("offline".to_string()),
        );
        assert!(source.is_quorum_source_unavailable());
        for _ in 0..2 {
            source = Error::NoAvailableAddressesToRetry(Box::new(source));
            assert!(source.is_quorum_source_unavailable());
        }
        for unrelated in [
            Error::ContextProviderError(ContextProviderError::InvalidQuorum(
                "quorum source unavailable".to_string(),
            )),
            Error::Generic("QuorumSourceUnavailable: quorum source unavailable".to_string()),
        ] {
            assert!(!unrelated.is_quorum_source_unavailable());
            let wrapped = Error::NoAvailableAddressesToRetry(Box::new(unrelated));
            assert!(!wrapped.is_quorum_source_unavailable());
        }
    }

    /// Another node cannot serve a request the network has not activated yet, and the
    /// refusal names no node to ban. The message says why it was refused. A stale answer to
    /// such a request is retried elsewhere.
    #[test]
    fn should_not_retry_a_request_the_network_cannot_serve_yet() {
        let refused = |network, pinned, cause: Option<Error>| Error::ProtocolVersionNotActive {
            method: "get_contract_fee_pots",
            required: 14,
            network,
            pinned,
            cause: cause.map(Box::new),
        };
        for (error, reason) in [
            (refused(13, false, None), "proven at protocol version 13"),
            (refused(0, false, None), "not verified"),
            (refused(0, true, None), "pinned"),
            (refused(13, true, None), "pinned"),
            (
                refused(0, false, Some(Error::Generic("offline".to_string()))),
                "could not be confirmed",
            ),
        ] {
            assert!(!error.can_retry(), "{error}");
            let message = error.to_string();
            assert!(
                message.contains("get_contract_fee_pots")
                    && message.contains("version 14")
                    && message.contains(reason),
                "{message}"
            );
        }
        assert!(std::error::Error::source(&refused(
            0,
            false,
            Some(Error::Generic("offline".into()))
        ))
        .is_some());
        let behind = Error::StaleNode(StaleNodeError::BehindProtocolUpgrade {
            required: 14,
            signed: 13,
        });
        assert!(behind.can_retry());
        assert!(
            !behind.spares_responder(),
            "a node behind an upgrade the network has proven is banned"
        );
    }

    /// A refusal may lift soon only when the SDK could not learn the network's version for a
    /// reason that can pass; not when its own setup can never verify an answer, nor when the
    /// network's verified version explains it.
    #[test]
    fn should_call_a_refusal_transient_only_when_its_cause_can_pass() {
        let refused = |cause: Option<Error>| Error::ProtocolVersionNotActive {
            method: "get_contract_fee_pots",
            required: 14,
            network: 0,
            pinned: false,
            cause: cause.map(Box::new),
        };
        let quorum_key = |error| {
            Error::Proof(drive_proof_verifier::Error::QuorumKeyUnavailable {
                quorum_type: 106,
                quorum_hash: [2; 32],
                core_chain_locked_height: 1,
                error,
            })
        };
        let transient = [
            Error::DapiClientError(DapiClientError::NoAvailableAddresses),
            Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(
                "HTTP 503".to_string(),
            )),
            quorum_key(ContextProviderError::InvalidQuorum(
                "unknown yet".to_string(),
            )),
            // What the trusted HTTP provider reports when refetching the quorum lists fails.
            quorum_key(ContextProviderError::Generic(
                "Failed to find quorum: HTTP 503".to_string(),
            )),
            Error::StaleNode(StaleNodeError::Time {
                expected_timestamp_ms: 1,
                received_timestamp_ms: 2,
                tolerance_ms: 0,
            }),
        ];
        for cause in transient {
            let message = cause.to_string();
            assert!(
                refused(Some(cause)).is_transient_protocol_version_refusal(),
                "{message}"
            );
        }
        let permanent = [
            Error::Proof(drive_proof_verifier::Error::ContextProviderNotSet),
            quorum_key(ContextProviderError::Config(
                "Non-trusted mode is not supported".to_string(),
            )),
            Error::NoAvailableAddressesToRetry(Box::new(quorum_key(ContextProviderError::Config(
                "no quorum source".to_string(),
            )))),
        ];
        for cause in permanent {
            let message = cause.to_string();
            assert!(
                !refused(Some(cause)).is_transient_protocol_version_refusal(),
                "{message}"
            );
        }
        assert!(!refused(None).is_transient_protocol_version_refusal());
        assert!(!Error::Generic("offline".to_string()).is_transient_protocol_version_refusal());
    }

    mod from_dapi_client_error {
        use super::*;
        use assert_matches::assert_matches;
        use base64::Engine;
        use dapi_grpc::tonic::metadata::{MetadataMap, MetadataValue};
        use dpp::consensus::basic::identity::IdentityAssetLockProofLockedTransactionMismatchError;
        use dpp::consensus::basic::BasicError;
        use dpp::dashcore::hashes::Hash;
        use dpp::dashcore::Txid;
        use dpp::serialization::PlatformSerializableWithPlatformVersion;
        use dpp::version::PlatformVersion;

        #[test]
        fn test_already_exists() {
            let error = DapiClientError::Transport(TransportError::Grpc(
                dapi_grpc::tonic::Status::new(Code::AlreadyExists, "Object already exists"),
            ));

            let sdk_error: Error = error.into();
            assert!(matches!(sdk_error, Error::AlreadyExists(_)));
        }

        #[test]
        fn test_consensus_error() {
            let platform_version = PlatformVersion::latest();

            let consensus_error = ConsensusError::BasicError(
                BasicError::IdentityAssetLockProofLockedTransactionMismatchError(
                    IdentityAssetLockProofLockedTransactionMismatchError::new(
                        Txid::from_byte_array([0; 32]),
                        Txid::from_byte_array([1; 32]),
                    ),
                ),
            );

            let consensus_error_bytes = consensus_error
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("serialize consensus error to bytes");

            let mut metadata = MetadataMap::new();
            metadata.insert_bin(
                "dash-serialized-consensus-error-bin",
                MetadataValue::from_bytes(&consensus_error_bytes),
            );

            let status =
                dapi_grpc::tonic::Status::with_metadata(Code::InvalidArgument, "Test", metadata);

            let error = DapiClientError::Transport(TransportError::Grpc(status));

            let sdk_error = Error::from(error);

            assert_matches!(
                sdk_error,
                Error::Protocol(ProtocolError::ConsensusError(e)) if matches!(*e, ConsensusError::BasicError(
                    BasicError::IdentityAssetLockProofLockedTransactionMismatchError(_)
                ))
            );
        }

        #[test]
        fn test_consensus_error_with_fixture() {
            let consensus_error_bytes = base64::engine::general_purpose::STANDARD.decode("ATUgJOJEYbuHBqyTeApO/ptxQ8IAw8nm9NbGROu1nyE/kqcgDTlFeUG0R4wwVcbZJMFErL+VSn63SUpP49cequ3fsKw=").expect("decode base64");
            let consensus_error = MetadataValue::from_bytes(&consensus_error_bytes);

            let mut metadata = MetadataMap::new();
            metadata.insert_bin("dash-serialized-consensus-error-bin", consensus_error);

            let status =
                dapi_grpc::tonic::Status::with_metadata(Code::InvalidArgument, "Test", metadata);

            let error = DapiClientError::Transport(TransportError::Grpc(status));

            let sdk_error = Error::from(error);

            assert_matches!(
                sdk_error,
                Error::Protocol(ProtocolError::ConsensusError(e)) if matches!(*e, ConsensusError::BasicError(
                    BasicError::IdentityAssetLockProofLockedTransactionMismatchError(_)
                ))
            );
        }

        #[test]
        fn test_drive_error_data_bin_maps_to_drive_internal_error() {
            let cbor_map = ciborium::Value::Map(vec![
                (
                    ciborium::Value::Text("code".to_string()),
                    ciborium::Value::Integer(13.into()),
                ),
                (
                    ciborium::Value::Text("message".to_string()),
                    ciborium::Value::Text(
                        "storage: identity: a unique key with that hash already exists: \
                         the key already exists in the non unique set [1, 2, 3]"
                            .to_string(),
                    ),
                ),
            ]);
            let mut cbor_bytes = Vec::new();
            ciborium::into_writer(&cbor_map, &mut cbor_bytes).expect("CBOR serialization");

            let mut metadata = MetadataMap::new();
            metadata.insert_bin(
                "drive-error-data-bin",
                MetadataValue::from_bytes(&cbor_bytes),
            );

            let status =
                dapi_grpc::tonic::Status::with_metadata(Code::Internal, "internal", metadata);
            let error = DapiClientError::Transport(TransportError::Grpc(status));

            let sdk_error = Error::from(error);

            assert_matches!(sdk_error, Error::DriveInternalError(msg) if msg.contains("unique key"));
        }

        #[test]
        fn test_internal_error_without_drive_metadata_falls_through() {
            let status = dapi_grpc::tonic::Status::new(Code::Internal, "Internal error");
            let error = DapiClientError::Transport(TransportError::Grpc(status));

            let sdk_error = Error::from(error);

            assert_matches!(sdk_error, Error::DapiClientError(_));
        }

        #[test]
        fn test_non_internal_code_with_drive_metadata_not_intercepted() {
            let cbor_map = ciborium::Value::Map(vec![(
                ciborium::Value::Text("message".to_string()),
                ciborium::Value::Text("some drive error".to_string()),
            )]);
            let mut cbor_bytes = Vec::new();
            ciborium::into_writer(&cbor_map, &mut cbor_bytes).expect("CBOR serialization");

            let mut metadata = MetadataMap::new();
            metadata.insert_bin(
                "drive-error-data-bin",
                MetadataValue::from_bytes(&cbor_bytes),
            );

            let status =
                dapi_grpc::tonic::Status::with_metadata(Code::Unavailable, "unavailable", metadata);
            let error = DapiClientError::Transport(TransportError::Grpc(status));

            let sdk_error = Error::from(error);

            assert_matches!(sdk_error, Error::DapiClientError(_));
        }

        #[test]
        fn test_malformed_cbor_in_drive_error_data_bin_falls_through() {
            let garbage_bytes = vec![0xFF, 0xFE, 0x00, 0x01, 0x02];

            let mut metadata = MetadataMap::new();
            metadata.insert_bin(
                "drive-error-data-bin",
                MetadataValue::from_bytes(&garbage_bytes),
            );

            let status =
                dapi_grpc::tonic::Status::with_metadata(Code::Internal, "internal", metadata);
            let error = DapiClientError::Transport(TransportError::Grpc(status));

            let sdk_error = Error::from(error);

            assert_matches!(sdk_error, Error::DapiClientError(_));
        }

        // Pathological CBOR: 60_000 nested single-pair-map openers (`0xA1`).
        // `ciborium` rejects this at its depth-256 recursion limit with a
        // normal `Err`, so the decode returns `None` without exhausting the
        // stack.
        #[test]
        fn test_deeply_nested_cbor_rejected_without_stack_exhaustion() {
            let payload = vec![0xA1u8; 60_000];
            assert!(super::extract_drive_error_message(&payload).is_none());
        }
    }
}
