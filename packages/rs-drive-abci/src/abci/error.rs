use dpp::bls_signatures::BlsError;
use dpp::consensus::ConsensusError;
use tenderdash_abci::proto::abci::ExtendVoteExtension;
use tenderdash_abci::proto::types::VoteExtension;

// @append_only
/// Error returned within ABCI server
#[derive(Debug, thiserror::Error)]
pub enum AbciError {
    /// Invalid system state
    #[error("invalid state: {0}")]
    InvalidState(String),
    /// Request does not match currently processed block
    #[error("request does not match current block: {0}")]
    RequestForWrongBlockReceived(String),
    /// Withdrawal votes extensions mismatch
    #[error("votes extensions mismatch: got {got:?}, expected {expected:?}")]
    #[allow(missing_docs)]
    VoteExtensionMismatchReceived {
        got: Vec<VoteExtension>,
        expected: Vec<ExtendVoteExtension>,
    },
    /// Vote extensions signature is invalid
    #[error("one of votes extension signatures is invalid")]
    VoteExtensionsSignatureInvalid,
    /// Invalid votes extensions verification
    #[error("invalid votes extensions verification")]
    InvalidVoteExtensionsVerification,
    /// Cannot load withdrawal transactions
    #[error("cannot load withdrawal transactions: {0}")]
    WithdrawalTransactionsDBLoadError(String),
    /// Wrong finalize block received
    #[error("finalize block received before processing from Tenderdash: {0}")]
    FinalizeBlockReceivedBeforeProcessing(String),
    /// Wrong finalize block received
    #[error("wrong block from Tenderdash: {0}")]
    WrongBlockReceived(String),
    /// Wrong finalize block received
    #[error("wrong finalize block from Tenderdash: {0}")]
    WrongFinalizeBlockReceived(String),
    /// Bad request received from Tenderdash that can't be translated to the correct size
    /// This often happens if a Vec<> can not be translated into a \[u8;32\]
    #[error("data received from Tenderdash could not be converted: {0}")]
    BadRequestDataSize(String),
    /// Bad request received from Tenderdash
    #[error("bad request received from Tenderdash: {0}")]
    BadRequest(String),

    /// Bad initialization from Tenderdash
    #[error("bad initialization: {0}")]
    BadInitialization(String),

    /// Bad commit signature from Tenderdash
    #[error("bad commit signature: {0}")]
    BadCommitSignature(String),

    /// Invalid state sync request received from Tenderdash or a peer
    #[error("bad request state sync: {0}")]
    StateSyncBadRequest(String),

    /// Internal error during state sync
    #[error("internal error state sync: {0}")]
    StateSyncInternalError(String),

    /// The chain lock received was invalid
    #[error("invalid chain lock: {0}")]
    InvalidChainLock(String),

    /// The chain lock received was invalid
    #[error("chain lock is for a block not known by core: {0}")]
    ChainLockedBlockNotKnownByCore(String),

    /// Error returned by Tenderdash-abci library
    #[error("tenderdash: {0}")]
    Tenderdash(#[from] tenderdash_abci::Error),

    /// Error occurred during protobuf data manipulation
    #[error("tenderdash data: {0}")]
    TenderdashProto(tenderdash_abci::proto::Error),

    /// Error occurred during signature verification or deserializing a BLS primitive
    #[error("bls error from user message: {0}")]
    BlsErrorFromUserMessage(BlsError),

    /// Error occurred related to threshold signing, either of commit
    #[error("bls error from Tenderdash for threshold mechanisms: {1}: {0}")]
    BlsErrorOfTenderdashThresholdMechanism(BlsError, String),

    /// Incompatibility version Error on info handshake between Drive ABCI and Tenderdash
    #[error("ABCI version mismatch. Tenderdash requires ABCI protobuf definitions version {tenderdash}, our version is {drive}")]
    AbciVersionMismatch {
        /// ABCI version in Tenderdash
        tenderdash: String,
        /// ABCI version in Drive ABCI
        drive: String,
    },

    /// Generic with code should only be used in tests
    #[error("invalid state transition error: {0}")]
    InvalidStateTransition(#[from] ConsensusError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_state_display() {
        let err = AbciError::InvalidState("bad state".to_string());
        assert_eq!(err.to_string(), "invalid state: bad state");
    }

    #[test]
    fn request_for_wrong_block_received_display() {
        let err = AbciError::RequestForWrongBlockReceived("wrong block".to_string());
        assert_eq!(
            err.to_string(),
            "request does not match current block: wrong block"
        );
    }

    #[test]
    fn vote_extension_mismatch_received_display() {
        let err = AbciError::VoteExtensionMismatchReceived {
            got: vec![],
            expected: vec![],
        };
        assert_eq!(
            err.to_string(),
            "votes extensions mismatch: got [], expected []"
        );
    }

    #[test]
    fn vote_extensions_signature_invalid_display() {
        let err = AbciError::VoteExtensionsSignatureInvalid;
        assert_eq!(
            err.to_string(),
            "one of votes extension signatures is invalid"
        );
    }

    #[test]
    fn invalid_vote_extensions_verification_display() {
        let err = AbciError::InvalidVoteExtensionsVerification;
        assert_eq!(err.to_string(), "invalid votes extensions verification");
    }

    #[test]
    fn withdrawal_transactions_db_load_error_display() {
        let err = AbciError::WithdrawalTransactionsDBLoadError("db fail".to_string());
        assert_eq!(
            err.to_string(),
            "cannot load withdrawal transactions: db fail"
        );
    }

    #[test]
    fn finalize_block_received_before_processing_display() {
        let err = AbciError::FinalizeBlockReceivedBeforeProcessing("not processed yet".to_string());
        assert_eq!(
            err.to_string(),
            "finalize block received before processing from Tenderdash: not processed yet"
        );
    }

    #[test]
    fn wrong_block_received_display() {
        let err = AbciError::WrongBlockReceived("bad block".to_string());
        assert_eq!(err.to_string(), "wrong block from Tenderdash: bad block");
    }

    #[test]
    fn wrong_finalize_block_received_display() {
        let err = AbciError::WrongFinalizeBlockReceived("bad finalize".to_string());
        assert_eq!(
            err.to_string(),
            "wrong finalize block from Tenderdash: bad finalize"
        );
    }

    #[test]
    fn bad_request_data_size_display() {
        let err = AbciError::BadRequestDataSize("size mismatch".to_string());
        assert_eq!(
            err.to_string(),
            "data received from Tenderdash could not be converted: size mismatch"
        );
    }

    #[test]
    fn bad_request_display() {
        let err = AbciError::BadRequest("invalid request".to_string());
        assert_eq!(
            err.to_string(),
            "bad request received from Tenderdash: invalid request"
        );
    }

    #[test]
    fn bad_initialization_display() {
        let err = AbciError::BadInitialization("init failed".to_string());
        assert_eq!(err.to_string(), "bad initialization: init failed");
    }

    #[test]
    fn bad_commit_signature_display() {
        let err = AbciError::BadCommitSignature("sig mismatch".to_string());
        assert_eq!(err.to_string(), "bad commit signature: sig mismatch");
    }

    #[test]
    fn invalid_chain_lock_display() {
        let err = AbciError::InvalidChainLock("lock invalid".to_string());
        assert_eq!(err.to_string(), "invalid chain lock: lock invalid");
    }

    #[test]
    fn chain_locked_block_not_known_by_core_display() {
        let err = AbciError::ChainLockedBlockNotKnownByCore("unknown block".to_string());
        assert_eq!(
            err.to_string(),
            "chain lock is for a block not known by core: unknown block"
        );
    }

    #[test]
    fn abci_version_mismatch_display() {
        let err = AbciError::AbciVersionMismatch {
            tenderdash: "1.0".to_string(),
            drive: "2.0".to_string(),
        };
        assert!(err.to_string().contains("1.0"));
        assert!(err.to_string().contains("2.0"));
    }

    #[test]
    fn invalid_state_transition_display() {
        let consensus_error = ConsensusError::DefaultError;
        let err = AbciError::InvalidStateTransition(consensus_error);
        assert_eq!(
            err.to_string(),
            "invalid state transition error: default error"
        );
    }

    #[test]
    fn tenderdash_proto_display() {
        let proto_err = tenderdash_abci::proto::Error::try_from_protobuf("proto fail".to_string());
        let err = AbciError::TenderdashProto(proto_err);
        assert!(err.to_string().contains("proto fail"));
    }

    #[test]
    fn error_debug_format_is_not_empty() {
        let err = AbciError::InvalidState("test".to_string());
        let debug = format!("{:?}", err);
        assert!(!debug.is_empty());
        assert!(debug.contains("InvalidState"));
    }

    #[test]
    fn consensus_error_converts_to_abci_error() {
        let consensus_err = ConsensusError::DefaultError;
        let abci_err: AbciError = consensus_err.into();
        assert!(matches!(abci_err, AbciError::InvalidStateTransition(_)));
    }
}
