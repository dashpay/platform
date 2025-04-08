use bincode::{Decode, Encode};

use std::convert::TryInto;

use derive_more::From;

use platform_value::Value;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::ProtocolError;
use crate::{identity::SecurityLevel, state_transition::StateTransitionFieldTypes};

pub use self::batched_transition::{
    document_base_transition, document_create_transition,
    document_create_transition::DocumentCreateTransition, document_delete_transition,
    document_delete_transition::DocumentDeleteTransition, document_replace_transition,
    document_replace_transition::DocumentReplaceTransition, token_base_transition,
    token_burn_transition, token_burn_transition::TokenBurnTransition, token_claim_transition,
    token_claim_transition::TokenClaimTransition, token_config_update_transition,
    token_config_update_transition::TokenConfigUpdateTransition,
    token_destroy_frozen_funds_transition,
    token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransition,
    token_direct_purchase_transition,
    token_direct_purchase_transition::TokenDirectPurchaseTransition,
    token_emergency_action_transition,
    token_emergency_action_transition::TokenEmergencyActionTransition, token_freeze_transition,
    token_freeze_transition::TokenFreezeTransition, token_mint_transition,
    token_mint_transition::TokenMintTransition, token_set_price_for_direct_purchase_transition,
    token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransition,
    token_transfer_transition, token_transfer_transition::TokenTransferTransition,
    token_unfreeze_transition, token_unfreeze_transition::TokenUnfreezeTransition,
};

use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::PlatformVersioned;

pub mod accessors;
pub mod batched_transition;
pub mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
pub mod resolvers;
mod state_transition_like;
mod v0;
mod v1;
#[cfg(feature = "validation")]
mod validation;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::state_transition::data_contract_update_transition::{
    SIGNATURE, SIGNATURE_PUBLIC_KEY_ID,
};

use crate::state_transition::batch_transition::fields::property_names;

use crate::identity::state_transition::OptionallyAssetLockProved;
pub use v0::*;
pub use v1::*;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.batch_state_transition"
)]
pub enum BatchTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
    V0(BatchTransitionV0),
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "1"))]
    V1(BatchTransitionV1),
}

impl StateTransitionFieldTypes for BatchTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::OWNER_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }
}

// TODO: Make a DocumentType method
pub fn get_security_level_requirement(v: &Value, default: SecurityLevel) -> SecurityLevel {
    let maybe_security_level: Option<u64> = v
        .get_optional_integer(property_names::SECURITY_LEVEL_REQUIREMENT)
        // TODO: Data Contract must already valid so there is no chance that this will fail
        .expect("document schema must be a map");

    match maybe_security_level {
        Some(some_level) => (some_level as u8).try_into().unwrap_or(default),
        None => default,
    }
}

impl OptionallyAssetLockProved for BatchTransition {}
