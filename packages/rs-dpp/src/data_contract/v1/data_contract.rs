use std::collections::BTreeMap;

use platform_value::Identifier;
use platform_value::Value;

use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::group::Group;
use crate::data_contract::{
    DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use crate::metadata::Metadata;

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
/// In `DataContractV1`, two significant features were introduced to enhance contract governance
/// and support token-related operations:
///
/// 1. **Groups** (`groups: BTreeMap<GroupContractPosition, Group>`)
///    - Groups allow for specific multiparty actions on the contract. Each group is defined with a
///      set of members (`Identifier`) and their corresponding member power (`u32`).  
///    - Groups facilitate fine-grained access control and decision-making processes by enabling
///      required power thresholds for group actions.
///    - This is particularly useful for contracts where multiple parties are involved in controlling
///      or managing contract-specific features.
///
/// 2. **Tokens** (`tokens: BTreeMap<TokenName, TokenConfiguration>`)  
///    - Tokens introduce configurable token-related functionality within the contract, such as
///      base supply, maximum supply, and manual minting/burning rules.  
///    - Token configurations include change control rules, ensuring proper governance for
///      modifying supply limits and token-related settings.
///    - This addition enables contracts to define and manage tokens while ensuring compliance
///      with governance rules (e.g., who can mint or burn tokens).
///
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
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

    // TODO: Move metadata from here
    /// Optional metadata associated with the contract.
    pub metadata: Option<Metadata>,

    /// Internal configuration for the contract.
    pub config: DataContractConfig,

    /// Shared subschemas to reuse across documents (see $defs)
    pub schema_defs: Option<BTreeMap<DefinitionName, Value>>,

    /// Groups that allow for specific multiparty actions on the contract
    pub groups: BTreeMap<GroupContractPosition, Group>,

    /// The tokens on the contract.
    pub tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,
}
