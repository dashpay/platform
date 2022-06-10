mod data_contract;
pub use data_contract::*;

pub mod errors;

mod generate_data_contract;
pub use generate_data_contract::*;

mod data_contract_factory;
pub use data_contract_factory::*;

pub mod enrich_data_contract_with_base_schema;
pub mod get_binary_properties_from_schema;
pub mod get_property_definition_by_path;
pub mod state_transition;
pub mod validation;

pub(self) mod properties {
    pub const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";
    pub const PROPERTY_ID: &str = "$id";
    pub const PROPERTY_OWNER_ID: &str = "ownerId";
    pub const PROPERTY_VERSION: &str = "version";
    pub const PROPERTY_SCHEMA: &str = "$schema";
    pub const PROPERTY_DOCUMENTS: &str = "documents";
    pub const PROPERTY_DEFINITIONS: &str = "$defs";
    pub const PROPERTY_ENTROPY: &str = "entropy";
}
