mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds a list of pre-programmed distributions to the state tree.
    ///
    /// This function inserts pre-programmed token distributions, ensuring they are properly structured
    /// within the storage tree. It creates necessary subtrees, validates input values, and associates
    /// each distribution entry with the appropriate identifiers and timestamps.
    ///
    /// # Parameters
    /// - `token_id`: The unique identifier of the token for which distributions are being added.
    /// - `owner_id`: The identifier of the entity that owns the distributions.
    /// - `distribution`: A `TokenPreProgrammedDistribution` containing the scheduled distributions.
    /// - `block_info`: Metadata about the current block, including epoch information.
    /// - `estimated_costs_only_with_layer_info`: If provided, stores estimated cost calculations
    ///   instead of applying the changes.
    /// - `batch_operations`: The list of low-level operations to be executed as a batch.
    /// - `transaction`: The transaction context for this operation.
    /// - `platform_version`: The version of the platform to determine the correct function variant.
    ///
    /// # Behavior
    /// - Ensures that the root path for pre-programmed distributions exists.
    /// - Inserts a new distribution entry if one does not already exist.
    /// - Stores distributions as sum trees, allowing for quick retrieval of total distributions at
    ///   a given time.
    /// - Uses reference paths to map distributions to their corresponding execution times.
    /// - Prevents overflow errors by ensuring token amounts do not exceed `i64::MAX`.
    ///
    /// # Tree structure
    /// ```text
    ///
    ///                          [ROOT: Tokens]
    ///                                 │
    ///             ┌───────────────────┴───────────────────┐
    ///             │                                       │
    ///  [TOKEN_STATUS_INFO_KEY]                 [TOKEN_DISTRIBUTIONS_KEY]
    ///                                                   │
    ///                                                   ├─────────────────────────┐
    ///                                                   │                         │
    ///                                [TOKEN_TIMED_DISTRIBUTIONS_KEY]   [TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY]
    ///                                                   │                         │
    ///                                                   │                         │
    ///                                      ┌────────────┴────────────┐            │
    ///                                      │                         │            │
    ///             [TOKEN_MS_TIMED_DISTRIBUTIONS_KEY]   ... (other timed trees)    │
    ///                                      │                                      │
    ///                                      │                           For each token (token_id)
    ///                                      │                                      │
    ///                                      │                           ┌──────────┴──────────┐
    ///                                      │                           │                     │
    ///                                      │                  token_id (e.g., TKN)          ...
    ///                                      │                           │
    ///                                      │                           │
    ///                                      │                    For each distribution time:
    ///                                      │                           │
    ///                                      │                   ┌───────┴───────┐
    ///                                      │                   │   time (ts)   │  <-- Key: timestamp (8 bytes)
    ///                                      │                   └───────┬───────┘
    ///                                      │                           │
    ///                                      │                  [ Sum Tree: Recipient → Amount ]
    ///                                      │                           │
    ///                                      │          ┌────────────────┴────────────────┐
    ///                                      │          │                │                │
    ///                                      │      recipient A -> amount    recipient B -> amount, etc.  <──────┐
    ///                                      │                                                                   │
    ///                                      └────────────────────────────────────────────────────               │
    ///                                                   (Separate branch)                                      │
    ///                                                   └─ In the TIMED DISTRIBUTIONS branch:                  │
    ///                                                        For each time:                                    │
    ///                                                        ┌─────────────┐                                   │
    ///                                                        │  time (ts)  │  <-- Key: timestamp (8 bytes)     │
    ///                                                        └─────┬───────┘                                   │
    ///                                                              │                                           │
    ///                                                [ Reference Tree: Serialized Distribution Keys ]  ────────┘
    /// ```
    /// # Explanation of Each Layer
    /// ```text
    ///     1.    Root Level (Tokens):
    ///     •    The top-level of the tree corresponds to the tokens in the system. The path starts with the root identifier for tokens.
    ///     2.    Distributions Branch (TOKEN_DISTRIBUTIONS_KEY):
    ///     •    Under the Tokens root, there is a branch reserved for token distributions.
    ///     •    This branch holds several subtrees for different kinds of distribution data.
    ///     3.    Timed vs. Pre-Programmed Distribution Subtrees:
    ///     •    Timed Distributions:
    ///     •    Located under TOKEN_TIMED_DISTRIBUTIONS_KEY, these trees help organize distributions by time for features like verifying the exact moment a distribution was made.
    ///     •    For example, the Millisecond Timed Distributions branch (TOKEN_MS_TIMED_DISTRIBUTIONS_KEY) contains nodes for each timestamp.
    ///     •    Pre-Programmed Distributions:
    ///     •    Located under TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, this branch stores pre-programmed distribution data.
    ///     •    For each token, a subtree is created using the token’s identifier.
    ///     •    Under that, each distribution time (converted to an 8‑byte big‑endian key) gets its own node (a sum tree).
    ///     4.    Inside Each Pre-Programmed Time Node:
    ///     •    Each node for a specific timestamp is a sum tree that holds key–value pairs where:
    ///     •    Key: The recipient’s identifier (the person or entity receiving tokens).
    ///     •    Value: The token amount (stored as a sum item).
    ///     •    This is the core data for a pre-programmed distribution at a specific time.
    ///     5.    Reference Insertion in Timed Distributions:
    ///     •    In parallel, a reference is inserted into the Millisecond Timed Distributions branch for the same timestamp.
    ///     •    This reference links back to the pre-programmed distribution data. It uses a TokenDistributionKey (which includes the token ID, recipient, and distribution type) to serialize and store a reference.
    ///     •    This reference is stored in a subtree keyed by the timestamp (again, 8 bytes).
    /// ```
    /// # What the Function Does
    /// ```text
    ///     1.    Insert the Pre-Programmed Distributions Tree:
    ///     •    First, the function ensures that the subtree for pre‑programmed distributions for the token exists.
    ///     •    It uses the fixed path:
    ///     [RootTree::Tokens, TOKEN_DISTRIBUTIONS_KEY, TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, token_id].
    ///     2.    For Each Distribution Time:
    ///     •    The function iterates over each distribution time (from the provided distribution data).
    ///     •    It inserts an empty sum tree for that time into the pre‑programmed subtree using the timestamp as a key.
    ///     •    It then creates a corresponding entry in the millisecond timed distributions branch.
    ///     This involves:
    ///     •    Inserting an empty tree if necessary, with storage flags.
    ///     •    Creating a reference path using the distribution key and additional metadata.
    ///     •    Finally, for each recipient at that timestamp, it inserts:
    ///     •    A sum item into the pre‑programmed distribution tree with the recipient’s identifier and the token amount.
    ///     •    A reference into the timed distributions tree that points back to the pre‑programmed entry.
    ///     3.    Why This Structure?
    ///     •    This hierarchical tree structure allows for efficient queries and proofs.
    ///     •    You can query by token, then by time, and then by recipient, and also verify that distributions were made at specific times.
    ///     •    The reference links between the pre-programmed and timed distributions trees help verify the ordering and correctness of distribution events.
    /// ```
    ///
    /// # Returns
    /// - `Ok(())` if the distributions are successfully added.
    /// - `Err(Error::Drive(DriveError::UnknownVersionMismatch))` if an unsupported platform version
    ///   is encountered.
    /// - `Err(Error::Protocol(ProtocolError::Overflow))` if a distribution amount exceeds the
    ///   maximum allowed value.
    #[allow(clippy::too_many_arguments)]
    pub fn add_pre_programmed_distributions(
        &self,
        token_id: [u8; 32],
        owner_id: [u8; 32],
        distribution: &TokenPreProgrammedDistribution,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .distribution
            .add_pre_programmed_distributions
        {
            0 => self.add_pre_programmed_distributions_v0(
                token_id,
                owner_id,
                distribution,
                block_info,
                estimated_costs_only_with_layer_info,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_pre_programmed_distributions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
