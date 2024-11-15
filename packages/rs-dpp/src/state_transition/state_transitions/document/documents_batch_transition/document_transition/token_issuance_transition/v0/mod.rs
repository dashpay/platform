pub mod v0_methods;

use bincode::{Decode, Encode};

#[cfg(feature = "state-transition-value-conversion")]
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use std::string::ToString;

#[cfg(feature = "state-transition-value-conversion")]
use crate::data_contract::DataContract;

use crate::{document, errors::ProtocolError};

use crate::block::block_info::BlockInfo;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0};
use crate::fee::Credits;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentTransitionObjectLike;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use derive_more::Display;
#[cfg(feature = "state-transition-value-conversion")]
use platform_value::btreemap_extensions::BTreeValueRemoveTupleFromMapHelper;
use platform_version::version::PlatformVersion;

#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::documents_batch_transition;

mod property_names {
    pub const AMOUNT: &str = "$amount";

}
/// The Identifier fields in [`TokenIssuanceTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("Base: {base}, Amount: {amount}")]
pub struct TokenIssuanceTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,

    /// How much should we issue
    #[cfg_attr(feature = "state-transition-serde-conversion")]
    pub amount: u64,
}