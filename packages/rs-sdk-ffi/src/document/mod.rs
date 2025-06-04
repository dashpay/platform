//! Document operations

pub mod create;
pub mod delete;
pub mod helpers;
pub mod info;
pub mod price;
pub mod purchase;
pub mod put;
pub mod queries;
pub mod replace;
pub mod transfer;

// Re-export functions from submodules
pub use create::{dash_sdk_document_create, DashSDKDocumentCreateParams};
pub use delete::{dash_sdk_document_delete, dash_sdk_document_delete_and_wait};
pub use info::{
    dash_sdk_document_destroy, dash_sdk_document_get_info, dash_sdk_document_handle_destroy,
};
pub use price::{
    dash_sdk_document_update_price_of_document, dash_sdk_document_update_price_of_document_and_wait,
};
pub use purchase::{dash_sdk_document_purchase, dash_sdk_document_purchase_and_wait};
pub use put::{dash_sdk_document_put_to_platform, dash_sdk_document_put_to_platform_and_wait};
pub use queries::{dash_sdk_document_fetch, dash_sdk_document_search, DashSDKDocumentSearchParams};
pub use replace::{
    dash_sdk_document_replace_on_platform, dash_sdk_document_replace_on_platform_and_wait,
};
pub use transfer::{
    dash_sdk_document_transfer_to_identity, dash_sdk_document_transfer_to_identity_and_wait,
};

// Re-export helper functions for use by submodules
pub use helpers::{
    convert_gas_fees_paid_by, convert_state_transition_creation_options, convert_token_payment_info,
};
