use dpp::bls_signatures::BlsError;

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
    /// Withdrawal transactions mismatch
    #[error("vote extensions mismatch: got {got:?}, expected {expected:?}")]
    #[allow(missing_docs)]
    VoteExtensionMismatchReceived { got: String, expected: String },
    /// Vote extensions signature is invalid
    #[error("one of vote extension signatures is invalid")]
    VoteExtensionsSignatureInvalid,
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

    /// Generic with code should only be used in tests
    #[error("generic with code: {0}")]
    GenericWithCode(u32),
}

// used by `?` operator
impl From<AbciError> for String {
    fn from(value: AbciError) -> Self {
        value.to_string()
    }
}
