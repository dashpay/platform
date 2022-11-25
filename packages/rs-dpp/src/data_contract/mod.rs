pub use data_contract::*;
pub use data_contract_factory::*;
pub use generate_data_contract::*;

mod data_contract;
pub mod errors;
pub mod extra;

mod data_contract_factory;
pub mod enrich_data_contract_with_base_schema;
mod generate_data_contract;
pub mod get_binary_properties_from_schema;
pub mod get_property_definition_by_path;
pub mod state_transition;
pub mod validation;

pub(self) mod property_names {
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const ID: &str = "$id";
    pub const OWNER_ID: &str = "ownerId";
    pub const VERSION: &str = "version";
    pub const SCHEMA: &str = "$schema";
    pub const DOCUMENTS: &str = "documents";
    pub const DEFINITIONS: &str = "$defs";
    pub const ENTROPY: &str = "entropy";
}
