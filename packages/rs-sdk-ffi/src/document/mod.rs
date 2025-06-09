//! Document operations

pub mod create;
pub mod delete;
pub mod helpers;
pub mod price;
pub mod purchase;
pub mod put;
pub mod queries;
pub mod replace;
pub mod set_properties;
pub mod transfer;
mod util;

// Re-export functions from submodules
pub use create::{dash_sdk_document_create, DashSDKDocumentCreateParams};
pub use delete::{dash_sdk_document_delete, dash_sdk_document_delete_and_wait};
pub use price::{
    dash_sdk_document_update_price_of_document, dash_sdk_document_update_price_of_document_and_wait,
};
pub use purchase::{dash_sdk_document_purchase, dash_sdk_document_purchase_and_wait};
pub use put::{dash_sdk_document_put_to_platform, dash_sdk_document_put_to_platform_and_wait};
pub use queries::info::dash_sdk_document_get_info;
pub use queries::{dash_sdk_document_fetch, dash_sdk_document_search, DashSDKDocumentSearchParams};
pub use replace::{
    dash_sdk_document_replace_on_platform, dash_sdk_document_replace_on_platform_and_wait,
};
pub use set_properties::{
    dash_sdk_document_remove, dash_sdk_document_set, dash_sdk_document_set_properties,
};
pub use transfer::{
    dash_sdk_document_transfer_to_identity, dash_sdk_document_transfer_to_identity_and_wait,
};
pub use util::{dash_sdk_document_destroy, dash_sdk_document_handle_destroy};

// Re-export helper functions for use by submodules
pub use helpers::{
    convert_gas_fees_paid_by, convert_state_transition_creation_options, convert_token_payment_info,
};
