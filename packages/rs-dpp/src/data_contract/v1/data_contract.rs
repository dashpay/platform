use std::collections::BTreeMap;

use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::group::Group;
use crate::data_contract::{
    DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use crate::identity::TimestampMillis;
use crate::prelude::BlockHeight;
use platform_value::Identifier;
use platform_value::Value;

/// `DataContractV1` represents a data contract in a decentralized platform.
///
/// It contains information about the contract, such as its protocol version, unique identifier,
/// schema, version, and owner identifier. The struct also includes details about the document
/// types, metadata, configuration, and document schemas associated with the contract.
///
/// Additionally, `DataContractV1` holds definitions for JSON schemas, entropy, and binary properties
/// of the documents.
///
/// # Changes from `DataContractV0` to `DataContractV1`
///
/// In `DataContractV1`, several enhancements were introduced to improve contract governance,
/// support token-related operations, and enhance auditability and traceability of contract updates.
///
/// ## 1. **Groups** (`groups: BTreeMap<GroupContractPosition, Group>`)
/// - Groups allow for specific multiparty actions on the contract. Each group is defined with a
///   set of members (`Identifier`) and their corresponding member power (`u32`).  
/// - Groups facilitate fine-grained access control and decision-making processes by enabling
///   required power thresholds for group actions.
/// - This is particularly useful for contracts where multiple parties are involved in controlling
///   or managing contract-specific features.
///
/// ## 2. **Tokens** (`tokens: BTreeMap<TokenName, TokenConfiguration>`)  
/// - Tokens introduce configurable token-related functionality within the contract, such as
///   base supply, maximum supply, and manual minting/burning rules.  
/// - Token configurations include change control rules, ensuring proper governance for
///   modifying supply limits and token-related settings.
/// - This addition enables contracts to define and manage tokens while ensuring compliance
///   with governance rules (e.g., who can mint or burn tokens).
///
/// ## 3. **Timestamps and Block Height Tracking**
/// To improve traceability and accountability of contract creation and modifications, four
/// new fields were added:
///
/// - **`created_at`** (`Option<TimestampMillis>`)  
///   - Stores the timestamp (in milliseconds) when the contract was originally created.  
///   - This provides an immutable record of when the contract came into existence.
/// - **`updated_at`** (`Option<TimestampMillis>`)  
///   - Stores the timestamp of the most recent update to the contract.  
///   - This helps in tracking contract modifications over time.
/// - **`created_at_block_height`** (`Option<BlockHeight>`)  
///   - Captures the block height at which the contract was created.  
///   - This provides an on-chain reference for the state of the contract at creation.
/// - **`updated_at_block_height`** (`Option<BlockHeight>`)  
///   - Captures the block height of the last contract update.  
///   - Useful for historical analysis, rollback mechanisms, and ensuring changes are anchored
///     to specific blockchain states.
///
/// ## 4. **Keywords** (`keywords: Vec<String>`)
/// - Keywords which contracts can be searched for via the new `search` system contract.
/// - This vector can be left empty, but if populated, it must contain unique keywords.
/// - The maximum number of keywords is limited to 20.
///
/// ## 5. **Description** (`description: Option<String>`)
/// - A human-readable description of the contract.
/// - This field is optional but if provided, must be between 3 and 100 characters.
/// - The description is automatically added to the new `search` system contract as well.
///
/// These additions ensure that data contracts are not only more flexible and governed but also
/// fully auditable in terms of when and how they evolve over time.
#[derive(Debug, Clone, PartialEq)]
pub struct DataContractV1 {
    /// A unique identifier for the data contract.
    /// This field must always present in all versions.
    pub id: Identifier,

    /// The version of this data contract.
    pub version: u32,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// A mapping of document names to their corresponding document types.
    pub document_types: BTreeMap<DocumentName, DocumentType>,

    /// Internal configuration for the contract.
    pub config: DataContractConfig,

    /// Shared subschemas to reuse across documents (see $defs)
    pub schema_defs: Option<BTreeMap<DefinitionName, Value>>,

    /// The time in milliseconds that the contract was created.
    pub created_at: Option<TimestampMillis>,
    /// The time in milliseconds that the contract was last updated.
    pub updated_at: Option<TimestampMillis>,
    /// The block that the document was created.
    pub created_at_block_height: Option<BlockHeight>,
    /// The block that the contract was last updated
    pub updated_at_block_height: Option<BlockHeight>,
    /// The epoch at which the contract was created.
    pub created_at_epoch: Option<EpochIndex>,
    /// The epoch at which the contract was last updated.
    pub updated_at_epoch: Option<EpochIndex>,

    /// Groups that allow for specific multiparty actions on the contract
    pub groups: BTreeMap<GroupContractPosition, Group>,

    /// The tokens on the contract.
    pub tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,

    /// The contract's keywords for searching
    pub keywords: Vec<String>,

    /// The contract's description
    pub description: Option<String>,
}
