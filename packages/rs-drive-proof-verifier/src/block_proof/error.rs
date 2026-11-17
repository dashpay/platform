/// Why a Tenderdash block proof, or what an answer says about the block, was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BlockProofError {
    /// The proof carries no header
    #[error("the block proof has no header")]
    MissingHeader,

    /// The proof carries no block id
    #[error("the block proof has no block id")]
    MissingBlockId,

    /// The header is not the one the signed block hash stands for
    #[error("the header of block {height} does not hash to its block id")]
    HeaderHashMismatch {
        /// The height the header names
        height: u64,
    },

    /// The header is not the one the signed state id stands for
    #[error("the header of block {height} does not hash to its state id")]
    StateIdMismatch {
        /// The height the header names
        height: u64,
    },

    /// The header's time is before 1970 or too large to count in milliseconds
    #[error("block {height} has a time outside the range of a state id")]
    TimeOutOfRange {
        /// The height the header names
        height: u64,
    },

    /// Tenderdash's state id hashing failed
    #[error("the state id of block {height} could not be hashed")]
    StateIdNotHashed {
        /// The height the header names
        height: u64,
    },

    /// A height, round or quorum type no vote can carry
    #[error("block {height} has a {field} outside the range of a vote")]
    VoteFieldOutOfRange {
        /// The height the header names
        height: u64,
        /// Which field is out of range
        field: &'static str,
    },

    /// A transaction is said to be at a position its block does not have
    #[error("position {index} is outside a block of {total} transactions")]
    IndexOutOfRange {
        /// The position the proof names
        index: u32,
        /// How many transactions the proof says the block holds
        total: u32,
    },

    /// A Merkle path has the wrong number of hashes for its position, or an entry that is
    /// not a hash
    #[error("the Merkle path of position {index} in a block of {total} transactions is malformed")]
    MalformedPath {
        /// The position the proof names
        index: u32,
        /// How many transactions the proof says the block holds
        total: u32,
    },

    /// A transaction's path does not lead to the header's `data_hash`
    #[error("the transaction at position {index} is not under the data hash of block {height}")]
    TransactionNotInBlock {
        /// The height the header names
        height: u64,
        /// The position the proof names
        index: u32,
    },

    /// A result's path does not lead to the header's `results_hash`
    #[error(
        "the result of the transaction at position {index} is not under the results hash of block {height}"
    )]
    ResultNotInBlock {
        /// The height the header names
        height: u64,
        /// The position the proof names
        index: u32,
    },

    /// A block's transactions do not hash to the header's `data_hash`
    #[error("the transactions of block {height} do not hash to the root in its header")]
    TransactionsRootMismatch {
        /// The height the header names
        height: u64,
    },

    /// A block's results do not hash to the header's `results_hash`
    #[error("the results of block {height} do not hash to the root in its header")]
    ResultsRootMismatch {
        /// The height the header names
        height: u64,
    },

    /// The proved block is not the one that was asked for
    #[error("asked for block {requested} but the proof is of block {proved}")]
    WrongBlock {
        /// The height the request names
        requested: u64,
        /// The height the proved header names
        proved: u64,
    },

    /// More blocks came back than were asked for
    #[error("asked for {requested} blocks but {returned} came back")]
    TooManyBlocks {
        /// The count the request names
        requested: u32,
        /// How many blocks the answer holds
        returned: usize,
    },

    /// A field a block repeats from its header differs from the signed header
    #[error("the {field} of block {height} is not the one in its signed header")]
    FieldMismatch {
        /// The height the header names
        height: u64,
        /// Which field differs
        field: &'static str,
    },

    /// A transaction comes without the hash Tenderdash knows it by
    #[error("the hash of transaction {position} of block {height} is not its SHA-256")]
    TransactionHashMismatch {
        /// The height the header names
        height: u64,
        /// The transaction's place in the answer
        position: usize,
    },

    /// A transaction of a proved answer comes without its Merkle paths
    #[error("transaction {position} of block {height} has no proof")]
    MissingTransactionProof {
        /// The height the header names
        height: u64,
        /// The transaction's place in the answer
        position: usize,
    },

    /// The transactions of an answer disagree on the size of their block
    #[error("the transactions of block {height} disagree on how many it holds")]
    InconsistentTotal {
        /// The height the header names
        height: u64,
    },

    /// The transactions of an answer are not in block order, or one is there twice
    #[error("the transactions of block {height} are not in block order")]
    NotInBlockOrder {
        /// The height the header names
        height: u64,
    },

    /// A transaction of the answer belongs to someone else, or to no one
    #[error("transaction {position} of block {height} is not the requested owner's")]
    NotTheOwners {
        /// The height the header names
        height: u64,
        /// The transaction's place in the answer
        position: usize,
    },
}
