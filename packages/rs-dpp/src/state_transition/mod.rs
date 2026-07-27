use derive_more::From;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransition;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use platform_value::{BinaryData, Identifier};
pub use state_transition_types::*;

use bincode::{Decode, Encode};
#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use dashcore::signer;
#[cfg(feature = "state-transition-validation")]
use dashcore::signer::double_sha;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::{PlatformVersion, ProtocolVersion, ALL_VERSIONS, LATEST_VERSION};

#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::BlsModule;
use crate::ProtocolError;

mod state_transition_types;

pub mod state_transition_factory;

pub mod errors;
#[cfg(feature = "state-transition-signing")]
use crate::util::hash::ripemd160_sha256;
use crate::util::hash::{hash_double_to_vec, hash_single};

pub mod proof_result;
mod serialization;
pub mod state_transitions;
mod traits;

// pub mod state_transition_fee;

#[cfg(feature = "state-transition-validation")]
use crate::consensus::basic::UnsupportedFeatureError;
#[cfg(feature = "state-transition-signing")]
use crate::consensus::signature::InvalidSignaturePublicKeySecurityLevelError;
#[cfg(feature = "state-transition-validation")]
use crate::consensus::signature::{
    InvalidStateTransitionSignatureError, PublicKeyIsDisabledError, SignatureError,
};
#[cfg(feature = "state-transition-validation")]
use crate::consensus::ConsensusError;
pub use traits::*;

use crate::address_funds::PlatformAddress;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::fee::Credits;
#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::identity::Purpose;
#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::identity::{IdentityPublicKey, KeyType};
use crate::identity::{KeyID, SecurityLevel};
use crate::prelude::{AddressNonce, AssetLockProof, UserFeeIncrease};
use crate::serialization::{PlatformDeserializable, Signable};
use crate::state_transition::address_credit_withdrawal_transition::{
    AddressCreditWithdrawalTransition, AddressCreditWithdrawalTransitionSignable,
};
use crate::state_transition::address_funding_from_asset_lock_transition::{
    AddressFundingFromAssetLockTransition, AddressFundingFromAssetLockTransitionSignable,
};
use crate::state_transition::address_funds_transfer_transition::{
    AddressFundsTransferTransition, AddressFundsTransferTransitionSignable,
};
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use crate::state_transition::batch_transition::{BatchTransition, BatchTransitionSignable};
use crate::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionSignable,
};
use crate::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionSignable,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::errors::InvalidSignaturePublicKeyError;
#[cfg(all(feature = "state-transitions", feature = "validation"))]
use crate::state_transition::errors::StateTransitionError::StateTransitionIsNotActiveError;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::errors::WrongPublicKeyPurposeError;
#[cfg(feature = "state-transition-validation")]
use crate::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, PublicKeyMismatchError, StateTransitionIsNotSignedError,
};
use crate::state_transition::identity_create_from_addresses_transition::{
    IdentityCreateFromAddressesTransition, IdentityCreateFromAddressesTransitionSignable,
};
use crate::state_transition::identity_create_from_shielded_pool_transition::{
    IdentityCreateFromShieldedPoolTransition, IdentityCreateFromShieldedPoolTransitionSignable,
};
use crate::state_transition::identity_create_transition::{
    IdentityCreateTransition, IdentityCreateTransitionSignable,
};
use crate::state_transition::identity_credit_transfer_to_addresses_transition::{
    IdentityCreditTransferToAddressesTransition,
    IdentityCreditTransferToAddressesTransitionSignable,
};
use crate::state_transition::identity_credit_transfer_transition::{
    IdentityCreditTransferTransition, IdentityCreditTransferTransitionSignable,
};
use crate::state_transition::identity_credit_withdrawal_transition::{
    IdentityCreditWithdrawalTransition, IdentityCreditWithdrawalTransitionSignable,
};
use crate::state_transition::identity_topup_from_addresses_transition::{
    IdentityTopUpFromAddressesTransition, IdentityTopUpFromAddressesTransitionSignable,
};
use crate::state_transition::identity_topup_transition::{
    IdentityTopUpTransition, IdentityTopUpTransitionSignable,
};
use crate::state_transition::identity_update_transition::{
    IdentityUpdateTransition, IdentityUpdateTransitionSignable,
};
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransitionSignable;
use crate::state_transition::shield_from_asset_lock_transition::{
    ShieldFromAssetLockTransition, ShieldFromAssetLockTransitionSignable,
};
use crate::state_transition::shield_transition::{ShieldTransition, ShieldTransitionSignable};
use crate::state_transition::shielded_transfer_transition::{
    ShieldedTransferTransition, ShieldedTransferTransitionSignable,
};
use crate::state_transition::shielded_withdrawal_transition::{
    ShieldedWithdrawalTransition, ShieldedWithdrawalTransitionSignable,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::state_transitions::document::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::unshield_transition::{
    UnshieldTransition, UnshieldTransitionSignable,
};
use state_transitions::document::batch_transition::batched_transition::token_transition::TokenTransition;
pub use state_transitions::*;

pub type GetDataContractSecurityLevelRequirementFn =
    fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>;

macro_rules! call_method {
    ($state_transition:expr, $method:ident, $args:tt ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method($args),
            StateTransition::DataContractUpdate(st) => st.$method($args),
            StateTransition::Batch(st) => st.$method($args),
            StateTransition::IdentityCreate(st) => st.$method($args),
            StateTransition::IdentityTopUp(st) => st.$method($args),
            StateTransition::IdentityCreditWithdrawal(st) => st.$method($args),
            StateTransition::IdentityUpdate(st) => st.$method($args),
            StateTransition::IdentityCreditTransfer(st) => st.$method($args),
            StateTransition::MasternodeVote(st) => st.$method($args),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.$method($args),
            StateTransition::IdentityCreateFromAddresses(st) => st.$method($args),
            StateTransition::IdentityTopUpFromAddresses(st) => st.$method($args),
            StateTransition::AddressFundsTransfer(st) => st.$method($args),
            StateTransition::AddressFundingFromAssetLock(st) => st.$method($args),
            StateTransition::AddressCreditWithdrawal(st) => st.$method($args),
            StateTransition::Shield(st) => st.$method($args),
            StateTransition::ShieldedTransfer(st) => st.$method($args),
            StateTransition::Unshield(st) => st.$method($args),
            StateTransition::ShieldFromAssetLock(st) => st.$method($args),
            StateTransition::ShieldedWithdrawal(st) => st.$method($args),
            StateTransition::IdentityCreateFromShieldedPool(st) => st.$method($args),
        }
    };
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method(),
            StateTransition::DataContractUpdate(st) => st.$method(),
            StateTransition::Batch(st) => st.$method(),
            StateTransition::IdentityCreate(st) => st.$method(),
            StateTransition::IdentityTopUp(st) => st.$method(),
            StateTransition::IdentityCreditWithdrawal(st) => st.$method(),
            StateTransition::IdentityUpdate(st) => st.$method(),
            StateTransition::IdentityCreditTransfer(st) => st.$method(),
            StateTransition::MasternodeVote(st) => st.$method(),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.$method(),
            StateTransition::IdentityCreateFromAddresses(st) => st.$method(),
            StateTransition::IdentityTopUpFromAddresses(st) => st.$method(),
            StateTransition::AddressFundsTransfer(st) => st.$method(),
            StateTransition::AddressFundingFromAssetLock(st) => st.$method(),
            StateTransition::AddressCreditWithdrawal(st) => st.$method(),
            StateTransition::Shield(st) => st.$method(),
            StateTransition::ShieldedTransfer(st) => st.$method(),
            StateTransition::Unshield(st) => st.$method(),
            StateTransition::ShieldFromAssetLock(st) => st.$method(),
            StateTransition::ShieldedWithdrawal(st) => st.$method(),
            StateTransition::IdentityCreateFromShieldedPool(st) => st.$method(),
        }
    };
}

macro_rules! call_getter_method_identity_signed {
    ($state_transition:expr, $method:ident, $args:tt ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => Some(st.$method($args)),
            StateTransition::DataContractUpdate(st) => Some(st.$method($args)),
            StateTransition::Batch(st) => Some(st.$method($args)),
            StateTransition::IdentityCreate(_) => None,
            StateTransition::IdentityTopUp(_) => None,
            StateTransition::IdentityCreditWithdrawal(st) => Some(st.$method($args)),
            StateTransition::IdentityUpdate(st) => Some(st.$method($args)),
            StateTransition::IdentityCreditTransfer(st) => Some(st.$method($args)),
            StateTransition::MasternodeVote(st) => Some(st.$method($args)),
            StateTransition::IdentityCreditTransferToAddresses(st) => Some(st.$method($args)),
            StateTransition::IdentityCreateFromAddresses(_) => None,
            StateTransition::IdentityTopUpFromAddresses(_) => None,
            StateTransition::AddressFundsTransfer(_) => None,
            StateTransition::AddressFundingFromAssetLock(_) => None,
            StateTransition::AddressCreditWithdrawal(_) => None,
            StateTransition::Shield(_) => None,
            StateTransition::ShieldedTransfer(_) => None,
            StateTransition::Unshield(_) => None,
            StateTransition::ShieldFromAssetLock(_) => None,
            StateTransition::ShieldedWithdrawal(_) => None,
            StateTransition::IdentityCreateFromShieldedPool(_) => None,
        }
    };
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => Some(st.$method()),
            StateTransition::DataContractUpdate(st) => Some(st.$method()),
            StateTransition::Batch(st) => Some(st.$method()),
            StateTransition::IdentityCreate(_) => None,
            StateTransition::IdentityTopUp(_) => None,
            StateTransition::IdentityCreditWithdrawal(st) => Some(st.$method()),
            StateTransition::IdentityUpdate(st) => Some(st.$method()),
            StateTransition::IdentityCreditTransfer(st) => Some(st.$method()),
            StateTransition::MasternodeVote(st) => Some(st.$method()),
            StateTransition::IdentityCreditTransferToAddresses(st) => Some(st.$method()),
            StateTransition::IdentityCreateFromAddresses(_) => None,
            StateTransition::IdentityTopUpFromAddresses(_) => None,
            StateTransition::AddressFundsTransfer(_) => None,
            StateTransition::AddressFundingFromAssetLock(_) => None,
            StateTransition::AddressCreditWithdrawal(_) => None,
            StateTransition::Shield(_) => None,
            StateTransition::ShieldedTransfer(_) => None,
            StateTransition::Unshield(_) => None,
            StateTransition::ShieldFromAssetLock(_) => None,
            StateTransition::ShieldedWithdrawal(_) => None,
            StateTransition::IdentityCreateFromShieldedPool(_) => None,
        }
    };
}

macro_rules! call_method_identity_signed {
    ($state_transition:expr, $method:ident, $args:tt ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method($args),
            StateTransition::DataContractUpdate(st) => st.$method($args),
            StateTransition::Batch(st) => st.$method($args),
            StateTransition::IdentityCreate(_st) => {}
            StateTransition::IdentityTopUp(_st) => {}
            StateTransition::IdentityCreditWithdrawal(st) => st.$method($args),
            StateTransition::IdentityUpdate(st) => st.$method($args),
            StateTransition::IdentityCreditTransfer(st) => st.$method($args),
            StateTransition::MasternodeVote(st) => st.$method($args),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.$method($args),
            StateTransition::IdentityCreateFromAddresses(_) => {}
            StateTransition::IdentityTopUpFromAddresses(_) => {}
            StateTransition::AddressFundsTransfer(_) => {}
            StateTransition::AddressFundingFromAssetLock(_) => {}
            StateTransition::AddressCreditWithdrawal(_) => {}
            StateTransition::Shield(_) => {}
            StateTransition::ShieldedTransfer(_) => {}
            StateTransition::Unshield(_) => {}
            StateTransition::ShieldFromAssetLock(_) => {}
            StateTransition::ShieldedWithdrawal(_) => {}
            StateTransition::IdentityCreateFromShieldedPool(_) => {}
        }
    };
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method(),
            StateTransition::DataContractUpdate(st) => st.$method(),
            StateTransition::Batch(st) => st.$method(),
            StateTransition::IdentityCreate(st) => {}
            StateTransition::IdentityTopUp(st) => {}
            StateTransition::IdentityCreditWithdrawal(st) => st.$method(),
            StateTransition::IdentityUpdate(st) => st.$method(),
            StateTransition::IdentityCreditTransfer(st) => st.$method(),
            StateTransition::MasternodeVote(st) => st.$method(),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.$method(),
            StateTransition::IdentityCreateFromAddresses(_) => {}
            StateTransition::IdentityTopUpFromAddresses(_) => {}
            StateTransition::AddressFundsTransfer(_) => {}
            StateTransition::AddressFundingFromAssetLock(_) => {}
            StateTransition::AddressCreditWithdrawal(_) => {}
            StateTransition::Shield(_) => {}
            StateTransition::ShieldedTransfer(_) => {}
            StateTransition::Unshield(_) => {}
            StateTransition::ShieldFromAssetLock(_) => {}
            StateTransition::ShieldedWithdrawal(_) => {}
            StateTransition::IdentityCreateFromShieldedPool(_) => {}
        }
    };
}

#[cfg(feature = "state-transition-signing")]
macro_rules! call_errorable_method_identity_signed {
    ($state_transition:expr, $method:ident, $( $arg:expr ),* ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method($( $arg ),*),
            StateTransition::DataContractUpdate(st) => st.$method($( $arg ),*),
            StateTransition::Batch(st) => st.$method($( $arg ),*),
            StateTransition::IdentityCreate(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity create can not be called for identity signing".to_string(),
            )),
            StateTransition::IdentityTopUp(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity top up can not be called for identity signing".to_string(),
            )),
            StateTransition::IdentityCreditWithdrawal(st) => st.$method($( $arg ),*),
            StateTransition::IdentityUpdate(st) => st.$method($( $arg ),*),
            StateTransition::IdentityCreditTransfer(st) => st.$method($( $arg ),*),
            StateTransition::MasternodeVote(st) => st.$method($( $arg ),*),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.$method($( $arg ),*),
            StateTransition::IdentityCreateFromAddresses(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity create from addresses can not be called for identity signing".to_string(),
            )),
            StateTransition::IdentityTopUpFromAddresses(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity top up from addresses can not be called for identity signing".to_string(),
            )),
            StateTransition::AddressFundsTransfer(_) => Err(ProtocolError::CorruptedCodeExecution(
                "address funds transfer can not be called for identity signing".to_string(),
            )),
            StateTransition::AddressFundingFromAssetLock(_) => Err(ProtocolError::CorruptedCodeExecution(
                "address funding from asset lock can not be called for identity signing".to_string(),
            )),
            StateTransition::AddressCreditWithdrawal(_) => Err(ProtocolError::CorruptedCodeExecution(
                "address credit withdrawal can not be called for identity signing".to_string(),
            )),
            StateTransition::Shield(_) => Err(ProtocolError::CorruptedCodeExecution(
                "shield transition can not be called for identity signing".to_string(),
            )),
            StateTransition::ShieldedTransfer(_) => Err(ProtocolError::CorruptedCodeExecution(
                "shielded transfer transition can not be called for identity signing".to_string(),
            )),
            StateTransition::Unshield(_) => Err(ProtocolError::CorruptedCodeExecution(
                "unshield transition can not be called for identity signing".to_string(),
            )),
            StateTransition::ShieldFromAssetLock(_) => Err(ProtocolError::CorruptedCodeExecution(
                "shield from asset lock transition can not be called for identity signing".to_string(),
            )),
            StateTransition::ShieldedWithdrawal(_) => Err(ProtocolError::CorruptedCodeExecution(
                "shielded withdrawal transition can not be called for identity signing".to_string(),
            )),
            StateTransition::IdentityCreateFromShieldedPool(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity create from shielded pool transition can not be called for identity signing".to_string(),
            )),
        }
    };
    ($state_transition:expr, $method:ident) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method(),
            StateTransition::DataContractUpdate(st) => st.$method(),
            StateTransition::Batch(st) => st.$method(),
            StateTransition::IdentityCreate(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity create can not be called for identity signing".to_string(),
            )),
            StateTransition::IdentityTopUp(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity top up can not be called for identity signing".to_string(),
            )),
            StateTransition::IdentityCreditWithdrawal(st) => st.$method(),
            StateTransition::IdentityUpdate(st) => st.$method(),
            StateTransition::IdentityCreditTransfer(st) => st.$method(),
            StateTransition::MasternodeVote(st) => st.$method(),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.$method(),
            StateTransition::IdentityCreateFromAddresses(st) => Err(ProtocolError::CorruptedCodeExecution(
                "identity create from addresses can not be called for identity signing".to_string(),
            )),
            StateTransition::IdentityTopUpFromAddresses(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity top up from addresses can not be called for identity signing".to_string(),
            )),
            StateTransition::AddressFundsTransfer(_) => Err(ProtocolError::CorruptedCodeExecution(
                "address funds transfer can not be called for identity signing".to_string(),
            )),
            StateTransition::AddressFundingFromAssetLock(_) => Err(ProtocolError::CorruptedCodeExecution(
                "address funding from asset lock can not be called for identity signing".to_string(),
            )),
            StateTransition::AddressCreditWithdrawal(_) => Err(ProtocolError::CorruptedCodeExecution(
                "address credit withdrawal can not be called for identity signing".to_string(),
            )),
            StateTransition::Shield(_) => Err(ProtocolError::CorruptedCodeExecution(
                "shield transition can not be called for identity signing".to_string(),
            )),
            StateTransition::ShieldedTransfer(_) => Err(ProtocolError::CorruptedCodeExecution(
                "shielded transfer transition can not be called for identity signing".to_string(),
            )),
            StateTransition::Unshield(_) => Err(ProtocolError::CorruptedCodeExecution(
                "unshield transition can not be called for identity signing".to_string(),
            )),
            StateTransition::ShieldFromAssetLock(_) => Err(ProtocolError::CorruptedCodeExecution(
                "shield from asset lock transition can not be called for identity signing".to_string(),
            )),
            StateTransition::ShieldedWithdrawal(_) => Err(ProtocolError::CorruptedCodeExecution(
                "shielded withdrawal transition can not be called for identity signing".to_string(),
            )),
            StateTransition::IdentityCreateFromShieldedPool(_) => Err(ProtocolError::CorruptedCodeExecution(
                "identity create from shielded pool transition can not be called for identity signing".to_string(),
            )),
        }
    };
}

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformSignable,
    From,
    PartialEq,
)]
// `tag = "$type"` matches the system-field convention: every serde-injected
// discriminator key in this crate carries a `$` prefix so it never collides
// with user-data field names. Discriminates between **semantically
// different variants** of the same kind (rather than **versions** of one
// logical type, which use `tag = "$formatVersion"`).
//
// `$type` here is at the OUTERMOST level — there's no flatten path that
// would put it next to a base's `document_type_name` (renamed to `$type`
// in the wire). Inner umbrellas (`DocumentTransition`, `TokenTransition`)
// use `$action` instead because they DO flatten the document base.
//
// Was previously `serde(untagged)`, which made deserialize ambiguous (each
// variant tried in order until one matched structurally). The new
// self-describing wire shape is `{"$type": "dataContractCreate", ...inner
// fields...}`.
//
// The binary wire path (`PlatformSerialize`) is unchanged — only JSON/Value
// consumers see the new shape, and there are no rs-drive / rs-drive-abci /
// rs-sdk callers that route the umbrella through to_json/to_object today.
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$type", rename_all = "camelCase")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_serialize(limit = 100000)]
pub enum StateTransition {
    DataContractCreate(DataContractCreateTransition),
    DataContractUpdate(DataContractUpdateTransition),
    Batch(BatchTransition),
    IdentityCreate(IdentityCreateTransition),
    IdentityTopUp(IdentityTopUpTransition),
    IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition),
    IdentityUpdate(IdentityUpdateTransition),
    IdentityCreditTransfer(IdentityCreditTransferTransition),
    MasternodeVote(MasternodeVoteTransition),
    IdentityCreditTransferToAddresses(IdentityCreditTransferToAddressesTransition),
    IdentityCreateFromAddresses(IdentityCreateFromAddressesTransition),
    IdentityTopUpFromAddresses(IdentityTopUpFromAddressesTransition),
    AddressFundsTransfer(AddressFundsTransferTransition),
    AddressFundingFromAssetLock(AddressFundingFromAssetLockTransition),
    AddressCreditWithdrawal(AddressCreditWithdrawalTransition),
    Shield(ShieldTransition),
    ShieldedTransfer(ShieldedTransferTransition),
    Unshield(UnshieldTransition),
    ShieldFromAssetLock(ShieldFromAssetLockTransition),
    ShieldedWithdrawal(ShieldedWithdrawalTransition),
    IdentityCreateFromShieldedPool(IdentityCreateFromShieldedPoolTransition),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for StateTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for StateTransition {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;

    /// Round-trip a StateTransition through both JSON and Value, asserting:
    /// 1. The wire emits `{"$type": "<expected_tag>", ...}` (umbrella's
    ///    `tag = "$type", rename_all = "camelCase"` is correctly applied).
    /// 2. Round-trip preserves the variant.
    /// 3. Round-trip preserves structural equality (PartialEq on the inner).
    ///
    /// Inner field shapes are covered by each inner type's dedicated
    /// `*_with_full_wire_shape` test — this helper only exercises the
    /// umbrella's tag-dispatch boundary. The risk it catches: an inner
    /// variant whose serde body conflicts with the umbrella's `"$type"` key,
    /// or a serde rename that resolves to something other than the
    /// expected camelCase form.
    ///
    /// `lossy_json_int_variants`: when true, the JSON-side equality assertion
    /// runs after `normalize_integer_variants_for_json_round_trip` on both
    /// sides. Required for variants that embed a `DataContract` —
    /// `document_schemas` carry sized integer variants (`U32`/`I32`) that
    /// JSON's single Number type cannot preserve. See commit 7397c73f31.
    fn assert_umbrella_round_trip_inner(
        original: StateTransition,
        expected_type_tag: &str,
        lossy_json_int_variants: bool,
    ) {
        use crate::serialization::{JsonConvertible, ValueConvertible};

        // JSON
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json["$type"], expected_type_tag,
            "json type tag for {expected_type_tag}",
        );
        let recovered = StateTransition::from_json(json).expect("from_json round-trip");
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&recovered),
            "json round-trip variant for {expected_type_tag}",
        );
        if lossy_json_int_variants {
            use crate::tests::utils::normalize_integer_variants_for_json_round_trip;
            let mut original_canon = original.to_object().expect("to_object");
            let mut recovered_canon = recovered.to_object().expect("to_object");
            normalize_integer_variants_for_json_round_trip(&mut original_canon);
            normalize_integer_variants_for_json_round_trip(&mut recovered_canon);
            assert_eq!(
                original_canon, recovered_canon,
                "json round-trip equality (modulo int-variant) for {expected_type_tag}",
            );
        } else {
            assert_eq!(
                original, recovered,
                "json round-trip equality for {expected_type_tag}"
            );
        }

        // Value
        let value = original.to_object().expect("to_object");
        let map = value.as_map().expect("Value::Map");
        let tag = map
            .iter()
            .find(|(k, _)| k.as_text() == Some("$type"))
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("type tag missing for {expected_type_tag}"));
        assert_eq!(
            *tag,
            platform_value::Value::Text(expected_type_tag.to_string()),
            "value type tag for {expected_type_tag}",
        );
        let recovered = StateTransition::from_object(value).expect("from_object round-trip");
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&recovered),
            "value round-trip variant for {expected_type_tag}",
        );
        assert_eq!(
            original, recovered,
            "value round-trip equality for {expected_type_tag}"
        );
    }

    fn assert_umbrella_round_trip(original: StateTransition, expected_type_tag: &str) {
        assert_umbrella_round_trip_inner(original, expected_type_tag, false);
    }

    /// Variant of `assert_umbrella_round_trip` for transitions that embed a
    /// `DataContract` (`DataContractCreate`, `DataContractUpdate`). JSON's
    /// single Number type collapses sized-int variants in the embedded
    /// `document_schemas` tree, so the JSON-side equality assertion is
    /// run modulo integer-variant normalization. The Value path keeps its
    /// strict bit-exact assertion (platform_value preserves sized ints).
    fn assert_umbrella_round_trip_lossy_json_int_variants(
        original: StateTransition,
        expected_type_tag: &str,
    ) {
        assert_umbrella_round_trip_inner(original, expected_type_tag, true);
    }

    // Per-variant umbrella round-trip tests. Inner fixtures are reused from
    // each transition's own `json_convertible_tests::fixture()` (made
    // `pub(crate)` for this purpose) — keeps the umbrella tests in sync
    // with the inner-type tests automatically.

    #[test]
    fn umbrella_data_contract_create() {
        let inner = crate::state_transition::data_contract_create_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip_lossy_json_int_variants(
            StateTransition::DataContractCreate(inner),
            "dataContractCreate",
        );
    }

    #[test]
    fn umbrella_data_contract_update() {
        let inner = crate::state_transition::data_contract_update_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip_lossy_json_int_variants(
            StateTransition::DataContractUpdate(inner),
            "dataContractUpdate",
        );
    }

    #[test]
    fn umbrella_batch() {
        let inner = crate::state_transition::batch_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(StateTransition::Batch(inner), "batch");
    }

    #[test]
    fn umbrella_identity_create() {
        let inner =
            crate::state_transition::identity_create_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(StateTransition::IdentityCreate(inner), "identityCreate");
    }

    #[test]
    fn umbrella_identity_top_up() {
        let inner =
            crate::state_transition::identity_topup_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(StateTransition::IdentityTopUp(inner), "identityTopUp");
    }

    #[test]
    fn umbrella_identity_credit_withdrawal() {
        let inner = crate::state_transition::identity_credit_withdrawal_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::IdentityCreditWithdrawal(inner),
            "identityCreditWithdrawal",
        );
    }

    #[test]
    fn umbrella_identity_update() {
        let inner =
            crate::state_transition::identity_update_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(StateTransition::IdentityUpdate(inner), "identityUpdate");
    }

    #[test]
    fn umbrella_identity_credit_transfer() {
        let inner = crate::state_transition::identity_credit_transfer_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::IdentityCreditTransfer(inner),
            "identityCreditTransfer",
        );
    }

    #[test]
    fn umbrella_masternode_vote() {
        let inner =
            crate::state_transition::masternode_vote_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(StateTransition::MasternodeVote(inner), "masternodeVote");
    }

    #[test]
    fn umbrella_identity_credit_transfer_to_addresses() {
        let inner = crate::state_transition::identity_credit_transfer_to_addresses_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::IdentityCreditTransferToAddresses(inner),
            "identityCreditTransferToAddresses",
        );
    }

    #[test]
    fn umbrella_identity_create_from_addresses() {
        let inner = crate::state_transition::identity_create_from_addresses_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::IdentityCreateFromAddresses(inner),
            "identityCreateFromAddresses",
        );
    }

    #[test]
    fn umbrella_identity_top_up_from_addresses() {
        let inner = crate::state_transition::identity_topup_from_addresses_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::IdentityTopUpFromAddresses(inner),
            "identityTopUpFromAddresses",
        );
    }

    #[test]
    fn umbrella_address_funds_transfer() {
        let inner = crate::state_transition::address_funds_transfer_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::AddressFundsTransfer(inner),
            "addressFundsTransfer",
        );
    }

    #[test]
    fn umbrella_address_funding_from_asset_lock() {
        let inner = crate::state_transition::address_funding_from_asset_lock_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::AddressFundingFromAssetLock(inner),
            "addressFundingFromAssetLock",
        );
    }

    #[test]
    fn umbrella_address_credit_withdrawal() {
        let inner = crate::state_transition::address_credit_withdrawal_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::AddressCreditWithdrawal(inner),
            "addressCreditWithdrawal",
        );
    }

    #[test]
    fn umbrella_shield() {
        let inner = crate::state_transition::shield_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(StateTransition::Shield(inner), "shield");
    }

    #[test]
    fn umbrella_shielded_transfer() {
        let inner =
            crate::state_transition::shielded_transfer_transition::json_convertible_tests::fixture(
            );
        assert_umbrella_round_trip(StateTransition::ShieldedTransfer(inner), "shieldedTransfer");
    }

    #[test]
    fn umbrella_unshield() {
        let inner = crate::state_transition::unshield_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(StateTransition::Unshield(inner), "unshield");
    }

    #[test]
    fn umbrella_shield_from_asset_lock() {
        let inner = crate::state_transition::shield_from_asset_lock_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::ShieldFromAssetLock(inner),
            "shieldFromAssetLock",
        );
    }

    #[test]
    fn umbrella_shielded_withdrawal() {
        let inner = crate::state_transition::shielded_withdrawal_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::ShieldedWithdrawal(inner),
            "shieldedWithdrawal",
        );
    }

    #[test]
    fn umbrella_identity_create_from_shielded_pool() {
        let inner = crate::state_transition::identity_create_from_shielded_pool_transition::json_convertible_tests::fixture();
        assert_umbrella_round_trip(
            StateTransition::IdentityCreateFromShieldedPool(inner),
            "identityCreateFromShieldedPool",
        );
    }
}

impl OptionallyAssetLockProved for StateTransition {
    fn optional_asset_lock_proof(&self) -> Option<&AssetLockProof> {
        match self {
            StateTransition::IdentityCreate(st) => st.optional_asset_lock_proof(),
            StateTransition::IdentityTopUp(st) => st.optional_asset_lock_proof(),
            StateTransition::ShieldFromAssetLock(st) => st.optional_asset_lock_proof(),
            _ => None,
        }
    }
}

/// The state transition signing options
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct StateTransitionSigningOptions {
    /// This will allow signing with any security level for debugging purposes
    pub allow_signing_with_any_security_level: bool,
    /// This will allow signing with any purpose for debugging purposes
    pub allow_signing_with_any_purpose: bool,
}

impl StateTransition {
    #[allow(unused_variables)]
    pub fn deserialize_from_bytes_in_version(
        bytes: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let max_value_depth = platform_version
            .system_limits
            .max_document_value_depth
            .map(usize::from);
        let state_transition =
            platform_value::with_value_decode_depth_limit(max_value_depth, || {
                StateTransition::deserialize_from_bytes(bytes)
            })?;
        #[cfg(all(feature = "state-transitions", feature = "validation"))]
        {
            let active_version_range = state_transition.active_version_range();

            // Tests are done with very high protocol ranges, while we could put this behind a feature,
            // that would probably be overkill.
            if active_version_range.contains(&platform_version.protocol_version)
                || platform_version.protocol_version > 268435456
            {
                Ok(state_transition)
            } else {
                Err(ProtocolError::StateTransitionError(
                    StateTransitionIsNotActiveError {
                        state_transition_type: state_transition.name(),
                        active_version_range,
                        current_protocol_version: platform_version.protocol_version,
                    },
                ))
            }
        }
        #[cfg(not(all(feature = "state-transitions", feature = "validation")))]
        Ok(state_transition)
    }

    pub fn active_version_range(&self) -> RangeInclusive<ProtocolVersion> {
        match self {
            StateTransition::DataContractCreate(data_contract_create_transition) => {
                match data_contract_create_transition.data_contract() {
                    DataContractInSerializationFormat::V0(_) => ALL_VERSIONS,
                    DataContractInSerializationFormat::V1(_) => 9..=LATEST_VERSION,
                }
            }
            StateTransition::DataContractUpdate(data_contract_update_transition) => {
                match data_contract_update_transition.data_contract() {
                    DataContractInSerializationFormat::V0(_) => ALL_VERSIONS,
                    DataContractInSerializationFormat::V1(_) => 9..=LATEST_VERSION,
                }
            }
            StateTransition::Batch(batch_transition) => match batch_transition {
                BatchTransition::V0(_) => ALL_VERSIONS,
                BatchTransition::V1(_) => 9..=LATEST_VERSION,
            },
            StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_) => ALL_VERSIONS,
            StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_) => 11..=LATEST_VERSION,
            StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => 12..=LATEST_VERSION,
        }
    }

    pub fn is_identity_signed(&self) -> bool {
        !matches!(
            self,
            StateTransition::IdentityCreate(_)
                | StateTransition::IdentityTopUp(_)
                | StateTransition::Shield(_)
                | StateTransition::ShieldedTransfer(_)
                | StateTransition::Unshield(_)
                | StateTransition::ShieldFromAssetLock(_)
                | StateTransition::ShieldedWithdrawal(_)
                | StateTransition::IdentityCreateFromShieldedPool(_)
        )
    }

    pub fn required_asset_lock_balance_for_processing_start(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match self {
            StateTransition::IdentityCreate(st) => {
                st.calculate_min_required_fee(platform_version)
            }
            StateTransition::IdentityTopUp(st) => {
                st.calculate_min_required_fee(platform_version)
            }
            StateTransition::AddressFundingFromAssetLock(st) => {
                st.calculate_min_required_fee(platform_version)
            }
            StateTransition::ShieldFromAssetLock(st) => {
                st.calculate_min_required_fee(platform_version)
            }
            st => Err(ProtocolError::CorruptedCodeExecution(format!("{} is not an asset lock transaction, but we are calling required_asset_lock_balance_for_processing_start", st.name()))),
        }
    }

    fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        if skip_signature {
            Ok(hash_double_to_vec(self.signable_bytes()?))
        } else {
            Ok(hash_double_to_vec(
                crate::serialization::PlatformSerializable::serialize_to_bytes(self)?,
            ))
        }
    }

    /// Returns state transition name
    pub fn name(&self) -> String {
        match self {
            Self::DataContractCreate(_) => "DataContractCreate".to_string(),
            Self::DataContractUpdate(_) => "DataContractUpdate".to_string(),
            Self::Batch(batch_transition) => {
                let mut document_transition_types = vec![];
                for transition in batch_transition.transitions_iter() {
                    let document_transition_name = match transition {
                        BatchedTransitionRef::Document(DocumentTransition::Create(_)) => "Create",
                        BatchedTransitionRef::Document(DocumentTransition::Replace(_)) => "Replace",
                        BatchedTransitionRef::Document(DocumentTransition::Delete(_)) => "Delete",
                        BatchedTransitionRef::Document(DocumentTransition::Transfer(_)) => {
                            "Transfer"
                        }
                        BatchedTransitionRef::Document(DocumentTransition::UpdatePrice(_)) => {
                            "UpdatePrice"
                        }
                        BatchedTransitionRef::Document(DocumentTransition::Purchase(_)) => {
                            "Purchase"
                        }
                        BatchedTransitionRef::Token(TokenTransition::Transfer(_)) => {
                            "TokenTransfer"
                        }
                        BatchedTransitionRef::Token(TokenTransition::Mint(_)) => "TokenMint",
                        BatchedTransitionRef::Token(TokenTransition::Burn(_)) => "TokenBurn",
                        BatchedTransitionRef::Token(TokenTransition::Freeze(_)) => "TokenFreeze",
                        BatchedTransitionRef::Token(TokenTransition::Unfreeze(_)) => {
                            "TokenUnfreeze"
                        }
                        BatchedTransitionRef::Token(TokenTransition::DestroyFrozenFunds(_)) => {
                            "TokenDestroyFrozenFunds"
                        }
                        BatchedTransitionRef::Token(TokenTransition::EmergencyAction(_)) => {
                            "TokenEmergencyAction"
                        }
                        BatchedTransitionRef::Token(TokenTransition::ConfigUpdate(_)) => {
                            "TokenConfigUpdate"
                        }
                        BatchedTransitionRef::Token(TokenTransition::Claim(_)) => "TokenClaim",
                        BatchedTransitionRef::Token(TokenTransition::DirectPurchase(_)) => {
                            "TokenDirectPurchase"
                        }
                        BatchedTransitionRef::Token(
                            TokenTransition::SetPriceForDirectPurchase(_),
                        ) => "SetPriceForDirectPurchase",
                    };
                    document_transition_types.push(document_transition_name);
                }
                format!("DocumentsBatch([{}])", document_transition_types.join(", "))
            }
            Self::IdentityCreate(_) => "IdentityCreate".to_string(),
            Self::IdentityTopUp(_) => "IdentityTopUp".to_string(),
            Self::IdentityCreditWithdrawal(_) => "IdentityCreditWithdrawal".to_string(),
            Self::IdentityUpdate(_) => "IdentityUpdate".to_string(),
            Self::IdentityCreditTransfer(_) => "IdentityCreditTransfer".to_string(),
            Self::MasternodeVote(_) => "MasternodeVote".to_string(),
            Self::IdentityCreditTransferToAddresses(_) => {
                "IdentityCreditTransferToAddresses".to_string()
            }
            Self::IdentityCreateFromAddresses(_) => "IdentityCreateFromAddresses".to_string(),
            Self::IdentityTopUpFromAddresses(_) => "IdentityTopUpFromAddresses".to_string(),
            Self::AddressFundsTransfer(_) => "AddressFundsTransfer".to_string(),
            Self::AddressFundingFromAssetLock(_) => "AddressFundingFromAssetLock".to_string(),
            Self::AddressCreditWithdrawal(_) => "AddressCreditWithdrawal".to_string(),
            Self::Shield(_) => "Shield".to_string(),
            Self::ShieldedTransfer(_) => "ShieldedTransfer".to_string(),
            Self::Unshield(_) => "Unshield".to_string(),
            Self::ShieldFromAssetLock(_) => "ShieldFromAssetLock".to_string(),
            Self::ShieldedWithdrawal(_) => "ShieldedWithdrawal".to_string(),
            Self::IdentityCreateFromShieldedPool(_) => "IdentityCreateFromShieldedPool".to_string(),
        }
    }

    /// returns the signature as a byte-array
    pub fn signature(&self) -> Option<&BinaryData> {
        match self {
            StateTransition::DataContractCreate(st) => Some(st.signature()),
            StateTransition::DataContractUpdate(st) => Some(st.signature()),
            StateTransition::Batch(st) => Some(st.signature()),
            StateTransition::IdentityCreate(st) => Some(st.signature()),
            StateTransition::IdentityTopUp(st) => Some(st.signature()),
            StateTransition::IdentityCreditWithdrawal(st) => Some(st.signature()),
            StateTransition::IdentityUpdate(st) => Some(st.signature()),
            StateTransition::IdentityCreditTransfer(st) => Some(st.signature()),
            StateTransition::MasternodeVote(st) => Some(st.signature()),
            StateTransition::IdentityCreditTransferToAddresses(st) => Some(st.signature()),
            StateTransition::IdentityCreateFromAddresses(_) => None,
            StateTransition::IdentityTopUpFromAddresses(_) => None,
            StateTransition::AddressFundsTransfer(_) => None,
            StateTransition::AddressFundingFromAssetLock(st) => Some(st.signature()),
            StateTransition::AddressCreditWithdrawal(_) => None,
            StateTransition::Shield(_) => None,
            StateTransition::ShieldedTransfer(_) => None,
            StateTransition::Unshield(_) => None,
            StateTransition::ShieldFromAssetLock(st) => Some(st.signature()),
            StateTransition::ShieldedWithdrawal(_) => None,
            StateTransition::IdentityCreateFromShieldedPool(_) => None,
        }
    }

    /// returns the number of private keys
    pub fn required_number_of_private_keys(&self) -> u16 {
        match self {
            StateTransition::IdentityCreateFromAddresses(st) => st.inputs().len() as u16,
            StateTransition::IdentityTopUpFromAddresses(st) => st.inputs().len() as u16,
            StateTransition::AddressFundsTransfer(st) => st.inputs().len() as u16,
            StateTransition::AddressCreditWithdrawal(st) => st.inputs().len() as u16,
            StateTransition::Shield(st) => st.inputs().len() as u16,
            StateTransition::ShieldedTransfer(_) => 0,
            StateTransition::Unshield(_) => 0,
            StateTransition::ShieldFromAssetLock(_) => 0,
            StateTransition::ShieldedWithdrawal(_) => 0,
            StateTransition::IdentityCreateFromShieldedPool(_) => 0,
            _ => 1,
        }
    }

    /// returns the fee_increase additional percentage multiplier, it affects only processing costs
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            StateTransition::DataContractCreate(st) => st.user_fee_increase(),
            StateTransition::DataContractUpdate(st) => st.user_fee_increase(),
            StateTransition::Batch(st) => st.user_fee_increase(),
            StateTransition::IdentityCreate(st) => st.user_fee_increase(),
            StateTransition::IdentityTopUp(st) => st.user_fee_increase(),
            StateTransition::IdentityCreditWithdrawal(st) => st.user_fee_increase(),
            StateTransition::IdentityUpdate(st) => st.user_fee_increase(),
            StateTransition::IdentityCreditTransfer(st) => st.user_fee_increase(),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.user_fee_increase(),
            StateTransition::IdentityCreateFromAddresses(st) => st.user_fee_increase(),
            StateTransition::IdentityTopUpFromAddresses(st) => st.user_fee_increase(),
            StateTransition::AddressFundsTransfer(st) => st.user_fee_increase(),
            StateTransition::AddressFundingFromAssetLock(st) => st.user_fee_increase(),
            StateTransition::AddressCreditWithdrawal(st) => st.user_fee_increase(),
            StateTransition::Shield(st) => st.user_fee_increase(),
            // These transitions don't support user fee adjustment
            StateTransition::ShieldFromAssetLock(_) => 0,
            StateTransition::MasternodeVote(_) => 0,
            StateTransition::ShieldedTransfer(_) => 0,
            StateTransition::Unshield(_) => 0,
            StateTransition::ShieldedWithdrawal(_) => 0,
            StateTransition::IdentityCreateFromShieldedPool(_) => 0,
        }
    }

    /// Calculates the estimated minimum fee required for this state transition.
    ///
    /// The fee is calculated based on the number of inputs, outputs, and any
    /// transition-specific costs (e.g., key creation costs for identity creation).
    ///
    /// # Arguments
    ///
    /// * `platform_version` - The platform version containing fee configuration.
    ///
    /// # Returns
    ///
    /// The estimated fee in credits.
    fn calculate_estimated_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        call_method!(self, calculate_min_required_fee, platform_version)
    }

    /// The transaction id is a single hash of the data with the signature
    pub fn transaction_id(&self) -> Result<[u8; 32], ProtocolError> {
        Ok(hash_single(
            crate::serialization::PlatformSerializable::serialize_to_bytes(self)?,
        ))
    }

    /// returns the signature as a byte-array
    pub fn signature_public_key_id(&self) -> Option<KeyID> {
        call_getter_method_identity_signed!(self, signature_public_key_id)
    }

    /// returns the key security level requirement for the state transition
    pub fn security_level_requirement(&self, purpose: Purpose) -> Option<Vec<SecurityLevel>> {
        call_getter_method_identity_signed!(self, security_level_requirement, purpose)
    }

    /// returns the key purpose requirement for the state transition
    pub fn purpose_requirement(&self) -> Option<Vec<Purpose>> {
        call_getter_method_identity_signed!(self, purpose_requirement)
    }

    /// returns the signature as a byte-array
    pub fn owner_id(&self) -> Option<Identifier> {
        match self {
            StateTransition::DataContractCreate(st) => Some(st.owner_id()),
            StateTransition::DataContractUpdate(st) => Some(st.owner_id()),
            StateTransition::Batch(st) => Some(st.owner_id()),
            StateTransition::IdentityCreate(st) => Some(st.owner_id()),
            StateTransition::IdentityTopUp(st) => Some(st.owner_id()),
            StateTransition::IdentityCreditWithdrawal(st) => Some(st.owner_id()),
            StateTransition::IdentityUpdate(st) => Some(st.owner_id()),
            StateTransition::IdentityCreditTransfer(st) => Some(st.owner_id()),
            StateTransition::MasternodeVote(st) => Some(st.owner_id()),
            StateTransition::IdentityCreditTransferToAddresses(st) => Some(st.owner_id()),
            StateTransition::IdentityCreateFromAddresses(_) => None,
            StateTransition::IdentityTopUpFromAddresses(_) => None,
            StateTransition::AddressFundsTransfer(_) => None,
            StateTransition::AddressFundingFromAssetLock(_) => None,
            StateTransition::AddressCreditWithdrawal(_) => None,
            StateTransition::Shield(_) => None,
            StateTransition::ShieldedTransfer(_) => None,
            StateTransition::Unshield(_) => None,
            StateTransition::ShieldFromAssetLock(_) => None,
            StateTransition::ShieldedWithdrawal(_) => None,
            StateTransition::IdentityCreateFromShieldedPool(_) => None,
        }
    }

    /// returns the signature as a byte-array
    pub fn inputs(&self) -> Option<&BTreeMap<PlatformAddress, (AddressNonce, Credits)>> {
        match self {
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreditTransferToAddresses(_) => None,
            StateTransition::IdentityCreateFromAddresses(st) => Some(st.inputs()),
            StateTransition::IdentityTopUpFromAddresses(st) => Some(st.inputs()),
            StateTransition::AddressFundsTransfer(st) => Some(st.inputs()),
            StateTransition::AddressFundingFromAssetLock(st) => Some(st.inputs()),
            StateTransition::AddressCreditWithdrawal(st) => Some(st.inputs()),
            StateTransition::Shield(st) => Some(st.inputs()),
            StateTransition::ShieldedTransfer(_) => None,
            StateTransition::Unshield(_) => None,
            StateTransition::ShieldFromAssetLock(_) => None,
            StateTransition::ShieldedWithdrawal(_) => None,
            StateTransition::IdentityCreateFromShieldedPool(_) => None,
        }
    }

    /// returns the state transition type
    pub fn state_transition_type(&self) -> StateTransitionType {
        call_method!(self, state_transition_type)
    }

    /// returns the unique identifiers for the state transition
    pub fn unique_identifiers(&self) -> Vec<String> {
        call_method!(self, unique_identifiers)
    }

    /// set a new signature
    pub fn set_signature(&mut self, signature: BinaryData) -> bool {
        match self {
            StateTransition::DataContractCreate(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::DataContractUpdate(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::Batch(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::IdentityCreate(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::IdentityTopUp(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::IdentityUpdate(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::IdentityCreditTransfer(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::MasternodeVote(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => false,
            StateTransition::AddressFundingFromAssetLock(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::ShieldFromAssetLock(st) => {
                st.set_signature(signature);
                true
            }
            StateTransition::AddressCreditWithdrawal(_) => false,
        }
    }

    /// set fee multiplier
    pub fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        match self {
            StateTransition::DataContractCreate(st) => st.set_user_fee_increase(user_fee_increase),
            StateTransition::DataContractUpdate(st) => st.set_user_fee_increase(user_fee_increase),
            StateTransition::Batch(st) => st.set_user_fee_increase(user_fee_increase),
            StateTransition::IdentityCreate(st) => st.set_user_fee_increase(user_fee_increase),
            StateTransition::IdentityTopUp(st) => st.set_user_fee_increase(user_fee_increase),
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.set_user_fee_increase(user_fee_increase)
            }
            StateTransition::IdentityUpdate(st) => st.set_user_fee_increase(user_fee_increase),
            StateTransition::IdentityCreditTransfer(st) => {
                st.set_user_fee_increase(user_fee_increase)
            }
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                st.set_user_fee_increase(user_fee_increase)
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                st.set_user_fee_increase(user_fee_increase)
            }
            StateTransition::IdentityTopUpFromAddresses(st) => {
                st.set_user_fee_increase(user_fee_increase)
            }
            StateTransition::AddressFundsTransfer(st) => {
                st.set_user_fee_increase(user_fee_increase)
            }
            StateTransition::AddressFundingFromAssetLock(st) => {
                st.set_user_fee_increase(user_fee_increase)
            }
            StateTransition::AddressCreditWithdrawal(st) => {
                st.set_user_fee_increase(user_fee_increase)
            }
            StateTransition::Shield(st) => st.set_user_fee_increase(user_fee_increase),
            // These transitions don't support user fee adjustment — no-op
            StateTransition::ShieldFromAssetLock(_) => {}
            StateTransition::MasternodeVote(_) => {}
            StateTransition::ShieldedTransfer(_) => {}
            StateTransition::Unshield(_) => {}
            StateTransition::ShieldedWithdrawal(_) => {}
            StateTransition::IdentityCreateFromShieldedPool(_) => {}
        }
    }

    /// set a new signature
    pub fn set_signature_public_key_id(&mut self, public_key_id: KeyID) {
        call_method_identity_signed!(self, set_signature_public_key_id, public_key_id)
    }

    #[cfg(feature = "state-transition-signing")]
    pub async fn sign_external<S: Signer<IdentityPublicKey>>(
        &mut self,
        identity_public_key: &IdentityPublicKey,
        signer: &S,
        get_data_contract_security_level_requirement: Option<
            impl Fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>,
        >,
    ) -> Result<(), ProtocolError> {
        self.sign_external_with_options(
            identity_public_key,
            signer,
            get_data_contract_security_level_requirement,
            StateTransitionSigningOptions::default(),
        )
        .await
    }

    #[cfg(feature = "state-transition-signing")]
    pub async fn sign_external_with_options<S: Signer<IdentityPublicKey>>(
        &mut self,
        identity_public_key: &IdentityPublicKey,
        signer: &S,
        get_data_contract_security_level_requirement: Option<
            impl Fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>,
        >,
        options: StateTransitionSigningOptions,
    ) -> Result<(), ProtocolError> {
        match self {
            StateTransition::DataContractCreate(st) => {
                st.verify_public_key_level_and_purpose(identity_public_key, options)?;
                st.verify_public_key_is_enabled(identity_public_key)?;
            }
            StateTransition::DataContractUpdate(st) => {
                st.verify_public_key_level_and_purpose(identity_public_key, options)?;
                st.verify_public_key_is_enabled(identity_public_key)?;
            }
            StateTransition::Batch(st) => {
                let allow_token_transfer_keys = st.transitions_len() == 1
                    && (st
                        .first_transition()
                        .expect("expected first transition with len 1")
                        .as_transition_token_claim()
                        .is_some()
                        || st
                            .first_transition()
                            .expect("expected first transition with len 1")
                            .as_transition_token_transfer()
                            .is_some());
                let allowed_key_purposes = if allow_token_transfer_keys {
                    vec![Purpose::AUTHENTICATION, Purpose::TRANSFER]
                } else {
                    vec![Purpose::AUTHENTICATION]
                };
                if !options.allow_signing_with_any_purpose
                    && !allowed_key_purposes.contains(&identity_public_key.purpose())
                {
                    return Err(ProtocolError::WrongPublicKeyPurposeError(
                        WrongPublicKeyPurposeError::new(
                            identity_public_key.purpose(),
                            allowed_key_purposes,
                        ),
                    ));
                }
                if !options.allow_signing_with_any_security_level {
                    let security_level_requirement = st.combined_security_level_requirement(
                        get_data_contract_security_level_requirement,
                    )?;
                    if !security_level_requirement.contains(&identity_public_key.security_level()) {
                        return Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                            InvalidSignaturePublicKeySecurityLevelError::new(
                                identity_public_key.security_level(),
                                security_level_requirement,
                            ),
                        ));
                    }
                }
                st.verify_public_key_is_enabled(identity_public_key)?;
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.verify_public_key_level_and_purpose(identity_public_key, options)?;
                st.verify_public_key_is_enabled(identity_public_key)?;
            }
            StateTransition::IdentityUpdate(st) => {
                st.verify_public_key_level_and_purpose(identity_public_key, options)?;
                st.verify_public_key_is_enabled(identity_public_key)?;
            }
            StateTransition::IdentityCreditTransfer(st) => {
                st.verify_public_key_level_and_purpose(identity_public_key, options)?;
                st.verify_public_key_is_enabled(identity_public_key)?;
            }
            StateTransition::IdentityCreate(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "identity create can not be called for identity signing".to_string(),
                ))
            }
            StateTransition::IdentityTopUp(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "identity top up can not be called for identity signing".to_string(),
                ))
            }
            StateTransition::MasternodeVote(st) => {
                st.verify_public_key_level_and_purpose(identity_public_key, options)?;
                st.verify_public_key_is_enabled(identity_public_key)?;
            }
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                st.verify_public_key_level_and_purpose(identity_public_key, options)?;
                st.verify_public_key_is_enabled(identity_public_key)?;
            }
            StateTransition::IdentityCreateFromAddresses(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "identity create from addresses can not be called for identity signing"
                        .to_string(),
                ))
            }
            StateTransition::IdentityTopUpFromAddresses(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "identity top up from addresses can not be called for identity signing"
                        .to_string(),
                ))
            }
            StateTransition::AddressFundsTransfer(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "address funds transfer transition can not be called for identity signing"
                        .to_string(),
                ))
            }
            StateTransition::AddressFundingFromAssetLock(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "address funding from asset lock transition can not be called for identity signing"
                        .to_string(),
                ))
            }
            StateTransition::AddressCreditWithdrawal(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "address credit withdrawal transition can not be called for identity signing"
                        .to_string(),
                ))
            }
            StateTransition::Shield(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "shield transition can not be called for identity signing".to_string(),
                ))
            }
            StateTransition::ShieldedTransfer(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "shielded transfer transition can not be called for identity signing"
                        .to_string(),
                ))
            }
            StateTransition::Unshield(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "unshield transition can not be called for identity signing".to_string(),
                ))
            }
            StateTransition::ShieldFromAssetLock(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "shield from asset lock transition can not be called for identity signing"
                        .to_string(),
                ))
            }
            StateTransition::ShieldedWithdrawal(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "shielded withdrawal transition can not be called for identity signing"
                        .to_string(),
                ))
            }
            StateTransition::IdentityCreateFromShieldedPool(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "identity create from shielded pool transition can not be called for identity signing"
                        .to_string(),
                ))
            }
        }
        let data = self.signable_bytes()?;
        self.set_signature(signer.sign(identity_public_key, data.as_slice()).await?);
        self.set_signature_public_key_id(identity_public_key.id());
        Ok(())
    }

    #[cfg(feature = "state-transition-signing")]
    pub fn sign(
        &mut self,
        identity_public_key: &IdentityPublicKey,
        private_key: &[u8],
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        self.sign_with_options(
            identity_public_key,
            private_key,
            bls,
            StateTransitionSigningOptions::default(),
        )
    }

    #[cfg(feature = "state-transition-signing")]
    pub fn sign_with_options(
        &mut self,
        identity_public_key: &IdentityPublicKey,
        private_key: &[u8],
        bls: &impl BlsModule,
        options: StateTransitionSigningOptions,
    ) -> Result<(), ProtocolError> {
        call_errorable_method_identity_signed!(
            self,
            verify_public_key_level_and_purpose,
            identity_public_key,
            options
        )?;
        call_errorable_method_identity_signed!(
            self,
            verify_public_key_is_enabled,
            identity_public_key
        )?;

        match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 => {
                let public_key_compressed = get_compressed_public_ec_key(private_key)?;

                // we store compressed public key in the identity ,
                // and here we compare the private key used to sing the state transition with
                // the compressed key stored in the identity

                if public_key_compressed.as_slice() != identity_public_key.data().as_slice() {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError(
                        InvalidSignaturePublicKeyError::new(identity_public_key.data().to_vec()),
                    ));
                }

                self.sign_by_private_key(private_key, identity_public_key.key_type(), bls)
            }
            KeyType::ECDSA_HASH160 => {
                let public_key_compressed = get_compressed_public_ec_key(private_key)?;
                let pub_key_hash = ripemd160_sha256(&public_key_compressed);

                if identity_public_key.data().as_slice() != pub_key_hash {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError(
                        InvalidSignaturePublicKeyError::new(identity_public_key.data().to_vec()),
                    ));
                }
                self.sign_by_private_key(private_key, identity_public_key.key_type(), bls)
            }
            KeyType::BLS12_381 => {
                let public_key = bls.private_key_to_public_key(private_key)?;

                if public_key != identity_public_key.data().as_slice() {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError(
                        InvalidSignaturePublicKeyError::new(identity_public_key.data().to_vec()),
                    ));
                }
                self.sign_by_private_key(private_key, identity_public_key.key_type(), bls)
            }

            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransitionIdentitySigned.js#L108
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(identity_public_key.key_type()),
                ))
            }
        }?;

        self.set_signature_public_key_id(identity_public_key.id());

        Ok(())
    }

    #[cfg(feature = "state-transition-signing")]
    /// Signs data with the private key
    pub fn sign_by_private_key(
        &mut self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        let data = self.signable_bytes()?;
        match key_type {
            KeyType::BLS12_381 => {
                if !self.set_signature(bls.sign(&data, private_key)?.into()) {
                    return Err(ProtocolError::InvalidVerificationWrongNumberOfElements {
                        needed: self.required_number_of_private_keys(),
                        using: 1,
                        msg: "failed to set BLS signature",
                    });
                }
            }

            // https://github.com/dashevo/platform/blob/9c8e6a3b6afbc330a6ab551a689de8ccd63f9120/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L169
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(&data, private_key)?;
                if !self.set_signature(signature.to_vec().into()) {
                    return Err(ProtocolError::InvalidVerificationWrongNumberOfElements {
                        needed: self.required_number_of_private_keys(),
                        using: 1,
                        msg: "failed to set ECDSA signature",
                    });
                };
            }

            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                return Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(key_type),
                ))
            }
        };
        Ok(())
    }

    /// Sign `self.signable_bytes()` with an external Core-wallet signer and
    /// store the resulting Core-ECDSA signature in the transition's wrapper
    /// signature field.
    ///
    /// # Position in the signing-primitive family
    ///
    /// This is a **primitive** in the same family as
    /// [`Self::sign_by_private_key`] — it performs no validation of the
    /// transition variant, the key, or the relationship between them. It is
    /// the external-custody sibling of `sign_by_private_key`:
    ///
    /// | Primitive | Key source | Validation |
    /// |---|---|---|
    /// | [`Self::sign_by_private_key`] | raw `&[u8]` in host memory | none |
    /// | `sign_with_core_signer` | external signer (HSM / hardware wallet / secure enclave / remote signing service), key reached via BIP32 [`DerivationPath`] | none |
    ///
    /// Both produce **byte-identical** wrapper signatures over the same
    /// digest when given the same underlying private key (proven by
    /// `sign_with_signer_matches_sign_by_private_key_byte_for_byte` in this
    /// file's tests). The only difference is where the key bytes live: in
    /// host memory vs inside the signer's trust boundary. The signer
    /// performs the derive + sign + zeroise sequence atomically; this
    /// function never sees raw key material, only a 32-byte digest and the
    /// resulting signature.
    ///
    /// # Scope (what the BIP32 path means)
    ///
    /// The `path` parameter selects a key in the signer's Core wallet
    /// (BIP32-derived). For that path's signature to be **meaningful** the
    /// transition's wrapper signature field must itself carry a Core-key
    /// signature. Today that is exactly the four asset-lock-signed
    /// variants — `IdentityCreate`, `IdentityTopUp`,
    /// `AddressFundingFromAssetLock`, `ShieldFromAssetLock` — where the
    /// wrapper signature is the asset-lock proof signed by the credit
    /// output's Core key.
    ///
    /// For identity-signed variants (`DataContractCreate`, `Batch`,
    /// `IdentityCreditTransfer`, etc.) the wrapper signature is an
    /// identity-key signature paired with a `signature_public_key_id`,
    /// and the right external-signer entry point is [`Self::sign_external`]
    /// with a [`Signer<IdentityPublicKey>`](crate::identity::signer::Signer).
    /// Calling `sign_with_core_signer` on such a variant compiles and
    /// produces a structurally valid 65-byte signature, but the signature
    /// is **semantically meaningless** — Platform validation will reject
    /// the transition because the signature doesn't match the expected
    /// identity public key and `signature_public_key_id` isn't set. The
    /// same caveat applies to misusing `sign_by_private_key`, the sibling
    /// primitive — both rely on the caller passing a key the wrapper
    /// signature is *meant* to carry.
    ///
    /// # Wire-format parity with `sign_by_private_key`
    ///
    /// The byte layout of the stored signature mirrors
    /// `dashcore::signer::sign`:
    ///
    /// 1. `digest = double_sha256(self.signable_bytes()?)`
    /// 2. `signer.sign_ecdsa(path, digest).await` → non-recoverable
    ///    `(secp256k1::ecdsa::Signature, secp256k1::PublicKey)`.
    /// 3. Recover the recovery id by trying all four candidates against the
    ///    returned public key (libsecp256k1 normalises both signing paths to
    ///    low-s form so the 64-byte `r||s` payload is bit-identical).
    /// 4. Serialise as a 65-byte compact recoverable signature with the
    ///    `compressed` prefix convention used by `CompactSignature` — i.e.
    ///    `[recovery_id + 27 + 4, r (32) || s (32)]`.
    ///
    /// # Errors
    ///
    /// - Returns [`ProtocolError::ExternalSignerError`] wrapping the signer's
    ///   `Display` error when the underlying signer fails.
    /// - Returns [`ProtocolError::ExternalSignerError`] if no recovery id
    ///   matches the public key returned by the signer — this should be
    ///   unreachable for a conformant signer (invariant violation by a
    ///   non-conformant signer) but is surfaced rather than panicked on.
    /// - Returns [`ProtocolError::Generic`] if the SHA-256 transform did not
    ///   yield a 32-byte digest (defensive — should never happen).
    /// - Returns [`ProtocolError::InvalidVerificationWrongNumberOfElements`] if
    ///   `set_signature` rejects the result (matches `sign_by_private_key`).
    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    pub async fn sign_with_core_signer<S: ::key_wallet::signer::Signer>(
        &mut self,
        path: &::key_wallet::bip32::DerivationPath,
        signer: &S,
    ) -> Result<(), ProtocolError> {
        use dashcore::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
        use dashcore::secp256k1::{Message, Secp256k1};
        use dashcore::signer::{double_sha, CompactSignature};

        let data = self.signable_bytes()?;
        // Pre-image transform matches `dashcore::signer::sign`: double-SHA256
        // of the signable bytes is the actual ECDSA message digest.
        let data_hash = double_sha(&data);
        let digest: [u8; 32] = data_hash.as_slice().try_into().map_err(|_| {
            ProtocolError::Generic("double_sha did not return 32 bytes".to_string())
        })?;

        let (signature, public_key) = signer
            .sign_ecdsa(path, digest)
            .await
            .map_err(|e| ProtocolError::ExternalSignerError(format!("signer failed: {}", e)))?;

        // The signer returns a non-recoverable signature. The legacy path
        // stores a 65-byte recoverable compact signature, so we brute-force
        // the recovery id (0..3) by reconstructing a `RecoverableSignature`
        // and comparing the recovered public key with the one the signer
        // returned. secp256k1 normalises both `sign_ecdsa` and
        // `sign_ecdsa_recoverable` outputs to low-s form, so the 64-byte
        // `r||s` payload is bit-identical to what `dashcore::signer::sign`
        // produces.
        let compact_64 = signature.serialize_compact();
        let secp = Secp256k1::new();
        let msg = Message::from_digest(digest);

        let mut found: Option<RecoverableSignature> = None;
        for id in 0..4i32 {
            let recid = match RecoveryId::try_from(id) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let candidate = match RecoverableSignature::from_compact(&compact_64, recid) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if let Ok(recovered) = secp.recover_ecdsa(&msg, &candidate) {
                if recovered == public_key {
                    found = Some(candidate);
                    break;
                }
            }
        }
        let recoverable = found.ok_or_else(|| {
            // Invariant violation by a non-conformant signer: the
            // signature returned does not correspond to the public
            // key the signer claims. Surface as ExternalSignerError
            // (NOT Generic) so callers can distinguish signer-side
            // failures from protocol-level invariants.
            ProtocolError::ExternalSignerError(
                "signer returned a signature whose recovery id does not match the returned public key".to_string(),
            )
        })?;

        // Compressed-pubkey convention matches `dashcore::signer::sign`, which
        // always passes `true` regardless of the underlying key encoding. The
        // signer's `sign_ecdsa` returns the compressed `secp256k1::PublicKey`,
        // so this is consistent.
        let compact_65 = recoverable.to_compact_signature(true);

        if !self.set_signature(compact_65.to_vec().into()) {
            return Err(ProtocolError::InvalidVerificationWrongNumberOfElements {
                needed: self.required_number_of_private_keys(),
                using: 1,
                msg: "failed to set ECDSA signature",
            });
        }
        Ok(())
    }

    #[cfg(feature = "state-transition-validation")]
    fn verify_by_raw_public_key<T: BlsModule>(
        &self,
        public_key: &[u8],
        public_key_type: KeyType,
        bls: &T,
    ) -> Result<(), ProtocolError> {
        match public_key_type {
            KeyType::ECDSA_SECP256K1 => self.verify_ecdsa_signature_by_public_key(public_key),
            KeyType::ECDSA_HASH160 => {
                self.verify_ecdsa_hash_160_signature_by_public_key_hash(public_key)
            }
            KeyType::BLS12_381 => self.verify_bls_signature_by_public_key(public_key, bls),
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(public_key_type),
                ))
            }
        }
    }

    #[cfg(feature = "state-transition-validation")]
    pub fn verify_identity_signed_signature(
        &self,
        public_key: &IdentityPublicKey,
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        // self.verify_public_key_level_and_purpose(public_key)?;
        if public_key.disabled_at().is_some() {
            return Err(ProtocolError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(public_key.id()),
            ));
        }

        let Some(signature) = self.signature() else {
            return Err(ProtocolError::CorruptedCodeExecution("verifying identity signature for a state transition that doesn't use identity signatures".to_string()));
        };
        if signature.is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone()),
            ));
        }

        if self.signature_public_key_id() != Some(public_key.id()) {
            return Err(ProtocolError::PublicKeyMismatchError(
                PublicKeyMismatchError::new(public_key.clone()),
            ));
        }

        let public_key_bytes = public_key.data().as_slice();
        match public_key.key_type() {
            KeyType::ECDSA_HASH160 => {
                self.verify_ecdsa_hash_160_signature_by_public_key_hash(public_key_bytes)
            }

            KeyType::ECDSA_SECP256K1 => self.verify_ecdsa_signature_by_public_key(public_key_bytes),

            KeyType::BLS12_381 => self.verify_bls_signature_by_public_key(public_key_bytes, bls),

            // per https://github.com/dashevo/platform/pull/353, signing and verification is not supported
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => Ok(()),
        }
    }

    #[cfg(feature = "state-transition-validation")]
    fn verify_ecdsa_hash_160_signature_by_public_key_hash(
        &self,
        public_key_hash: &[u8],
    ) -> Result<(), ProtocolError> {
        let Some(signature) = self.signature() else {
            return Err(ProtocolError::InvalidVerificationWrongNumberOfElements {
                needed: self.required_number_of_private_keys(),
                using: 1,
                msg: "This state transition type should a single signature",
            });
        };
        if signature.is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone()),
            ));
        }
        let data = self.signable_bytes()?;
        let data_hash = double_sha(data);
        signer::verify_hash_signature(&data_hash, signature.as_slice(), public_key_hash).map_err(
            |e| {
                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError(
                        InvalidStateTransitionSignatureError::new(e.to_string()),
                    ),
                ))
            },
        )
    }

    #[cfg(feature = "state-transition-validation")]
    /// Verifies an ECDSA signature with the public key
    fn verify_ecdsa_signature_by_public_key(&self, public_key: &[u8]) -> Result<(), ProtocolError> {
        let Some(signature) = self.signature() else {
            return Err(ProtocolError::InvalidVerificationWrongNumberOfElements {
                needed: self.required_number_of_private_keys(),
                using: 1,
                msg: "This state transition type should a single signature",
            });
        };
        if signature.is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone()),
            ));
        }
        let data = self.signable_bytes()?;
        signer::verify_data_signature(&data, signature.as_slice(), public_key).map_err(|e| {
            // TODO: it shouldn't respond with consensus error

            ProtocolError::from(ConsensusError::SignatureError(
                SignatureError::InvalidStateTransitionSignatureError(
                    InvalidStateTransitionSignatureError::new(e.to_string()),
                ),
            ))
        })
    }

    #[cfg(feature = "state-transition-validation")]
    /// Verifies a BLS signature with the public key
    fn verify_bls_signature_by_public_key<T: BlsModule>(
        &self,
        public_key: &[u8],
        bls: &T,
    ) -> Result<(), ProtocolError> {
        let Some(signature) = self.signature() else {
            return Err(ProtocolError::InvalidVerificationWrongNumberOfElements {
                needed: self.required_number_of_private_keys(),
                using: 1,
                msg: "This state transition type should a single signature",
            });
        };
        if signature.is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone()),
            ));
        }

        let data = self.signable_bytes()?;

        bls.verify_signature(signature.as_slice(), &data, public_key)
            .map(|_| ())
            .map_err(|e| {
                // TODO: it shouldn't respond with consensus error
                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError(
                        InvalidStateTransitionSignatureError::new(e.to_string()),
                    ),
                ))
            })
    }
}

#[cfg(feature = "state-transition-validation")]
impl StateTransitionStructureValidation for StateTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> crate::validation::SimpleConsensusValidationResult {
        match self {
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_) => {
                crate::validation::SimpleConsensusValidationResult::new_with_error(
                    UnsupportedFeatureError::new(
                        "structure validation for identity-based state transitions".to_string(),
                        platform_version.protocol_version,
                    )
                    .into(),
                )
            }
            StateTransition::IdentityCreditTransferToAddresses(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::IdentityCreateFromAddresses(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::IdentityTopUpFromAddresses(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::AddressFundsTransfer(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::AddressFundingFromAssetLock(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::AddressCreditWithdrawal(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::Shield(transition) => transition.validate_structure(platform_version),
            StateTransition::ShieldedTransfer(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::Unshield(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::ShieldFromAssetLock(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::ShieldedWithdrawal(transition) => {
                transition.validate_structure(platform_version)
            }
            StateTransition::IdentityCreateFromShieldedPool(transition) => {
                transition.validate_structure(platform_version)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // StateTransitionSigningOptions tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_signing_options_default() {
        let opts = StateTransitionSigningOptions::default();
        assert!(!opts.allow_signing_with_any_security_level);
        assert!(!opts.allow_signing_with_any_purpose);
    }

    #[test]
    fn test_signing_options_equality() {
        let a = StateTransitionSigningOptions {
            allow_signing_with_any_security_level: true,
            allow_signing_with_any_purpose: false,
        };
        let b = StateTransitionSigningOptions {
            allow_signing_with_any_security_level: true,
            allow_signing_with_any_purpose: false,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_signing_options_inequality() {
        let a = StateTransitionSigningOptions {
            allow_signing_with_any_security_level: true,
            allow_signing_with_any_purpose: false,
        };
        let b = StateTransitionSigningOptions {
            allow_signing_with_any_security_level: false,
            allow_signing_with_any_purpose: false,
        };
        assert_ne!(a, b);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_signing_options_clone() {
        let original = StateTransitionSigningOptions {
            allow_signing_with_any_security_level: true,
            allow_signing_with_any_purpose: true,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_signing_options_copy() {
        let original = StateTransitionSigningOptions {
            allow_signing_with_any_security_level: true,
            allow_signing_with_any_purpose: false,
        };
        let copied = original;
        assert_eq!(original, copied);
    }

    #[test]
    fn test_signing_options_debug() {
        let opts = StateTransitionSigningOptions::default();
        let debug_str = format!("{:?}", opts);
        assert!(debug_str.contains("StateTransitionSigningOptions"));
        assert!(debug_str.contains("allow_signing_with_any_security_level"));
        assert!(debug_str.contains("allow_signing_with_any_purpose"));
    }

    // -----------------------------------------------------------------------
    // StateTransition enum accessor / mutator / classification tests
    //
    // These exercise the non-trivial match arms across the large enum, using
    // the IdentityCreditTransfer, MasternodeVote, IdentityCreditWithdrawal and
    // DataContractCreate variants as representative signed / unsigned /
    // voting / contract cases. They intentionally do NOT use `sign`/`verify`
    // (those go through BLS/ECDSA and have their own coverage elsewhere).
    // -----------------------------------------------------------------------
    use crate::identity::core_script::CoreScript;
    use crate::identity::{Purpose, SecurityLevel};
    use crate::prelude::Identifier;
    use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use crate::withdrawal::Pooling;

    fn sample_transfer_st() -> StateTransition {
        let v0 = IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::from([1u8; 32]),
            recipient_id: Identifier::from([2u8; 32]),
            amount: 1_000,
            nonce: 7,
            user_fee_increase: 3,
            signature_public_key_id: 11,
            signature: BinaryData::new(vec![0u8; 65]),
        };
        StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(v0))
    }

    fn sample_masternode_vote_st() -> StateTransition {
        let v0 = MasternodeVoteTransitionV0 {
            pro_tx_hash: Identifier::from([3u8; 32]),
            voter_identity_id: Identifier::from([4u8; 32]),
            vote: Default::default(),
            nonce: 2,
            signature_public_key_id: 5,
            signature: BinaryData::new(vec![9u8; 10]),
        };
        StateTransition::MasternodeVote(MasternodeVoteTransition::V0(v0))
    }

    fn sample_withdrawal_st() -> StateTransition {
        let v0 = IdentityCreditWithdrawalTransitionV0 {
            identity_id: Identifier::from([5u8; 32]),
            amount: 42,
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes(vec![0x76, 0xa9]),
            nonce: 4,
            user_fee_increase: 1,
            signature_public_key_id: 3,
            signature: BinaryData::new(vec![8u8; 65]),
        };
        StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V0(v0))
    }

    #[test]
    fn test_name_returns_variant_names() {
        assert_eq!(sample_transfer_st().name(), "IdentityCreditTransfer");
        assert_eq!(sample_masternode_vote_st().name(), "MasternodeVote");
        assert_eq!(sample_withdrawal_st().name(), "IdentityCreditWithdrawal");
    }

    #[test]
    fn test_state_transition_type_matches_variant() {
        assert_eq!(
            sample_transfer_st().state_transition_type(),
            StateTransitionType::IdentityCreditTransfer
        );
        assert_eq!(
            sample_masternode_vote_st().state_transition_type(),
            StateTransitionType::MasternodeVote
        );
        assert_eq!(
            sample_withdrawal_st().state_transition_type(),
            StateTransitionType::IdentityCreditWithdrawal
        );
    }

    #[test]
    fn test_is_identity_signed_excludes_asset_lock_and_shielded() {
        assert!(sample_transfer_st().is_identity_signed());
        assert!(sample_masternode_vote_st().is_identity_signed());
        assert!(sample_withdrawal_st().is_identity_signed());
    }

    #[test]
    fn test_signature_accessor() {
        let st = sample_transfer_st();
        let sig = st.signature().expect("transfer should expose signature");
        assert_eq!(sig.len(), 65);

        let st = sample_masternode_vote_st();
        let sig = st.signature().expect("masternode vote has signature");
        assert_eq!(sig.as_slice(), &[9u8; 10]);
    }

    #[test]
    fn test_owner_id_accessor() {
        let transfer = sample_transfer_st();
        assert_eq!(transfer.owner_id(), Some(Identifier::from([1u8; 32])));

        let vote = sample_masternode_vote_st();
        assert_eq!(vote.owner_id(), Some(Identifier::from([4u8; 32])));

        let withdraw = sample_withdrawal_st();
        assert_eq!(withdraw.owner_id(), Some(Identifier::from([5u8; 32])));
    }

    #[test]
    fn test_signature_public_key_id_accessor() {
        assert_eq!(sample_transfer_st().signature_public_key_id(), Some(11));
        assert_eq!(
            sample_masternode_vote_st().signature_public_key_id(),
            Some(5)
        );
        assert_eq!(sample_withdrawal_st().signature_public_key_id(), Some(3));
    }

    #[test]
    fn test_user_fee_increase_for_various_variants() {
        // Transfer exposes its internal value.
        assert_eq!(sample_transfer_st().user_fee_increase(), 3);
        // Masternode vote returns 0 unconditionally.
        assert_eq!(sample_masternode_vote_st().user_fee_increase(), 0);
        // Withdrawal exposes its internal value.
        assert_eq!(sample_withdrawal_st().user_fee_increase(), 1);
    }

    #[test]
    fn test_set_signature_returns_true_for_supported() {
        let mut st = sample_transfer_st();
        let ok = st.set_signature(BinaryData::new(vec![0xaa; 65]));
        assert!(ok);
        assert_eq!(st.signature().unwrap().as_slice(), &[0xaa; 65]);
    }

    #[test]
    fn test_set_user_fee_increase_updates_value() {
        let mut st = sample_transfer_st();
        st.set_user_fee_increase(42);
        assert_eq!(st.user_fee_increase(), 42);

        // Masternode vote ignores the setter (documented no-op) — still reads 0.
        let mut vote = sample_masternode_vote_st();
        vote.set_user_fee_increase(99);
        assert_eq!(vote.user_fee_increase(), 0);
    }

    #[test]
    fn test_set_signature_public_key_id() {
        let mut st = sample_transfer_st();
        st.set_signature_public_key_id(1234);
        assert_eq!(st.signature_public_key_id(), Some(1234));
    }

    #[test]
    fn test_required_number_of_private_keys_default() {
        // Non asset-lock transitions always require 1 key.
        assert_eq!(sample_transfer_st().required_number_of_private_keys(), 1);
        assert_eq!(
            sample_masternode_vote_st().required_number_of_private_keys(),
            1
        );
        assert_eq!(sample_withdrawal_st().required_number_of_private_keys(), 1);
    }

    #[test]
    fn test_inputs_none_for_legacy_variants() {
        // All these variants have no PlatformAddress inputs.
        assert!(sample_transfer_st().inputs().is_none());
        assert!(sample_masternode_vote_st().inputs().is_none());
        assert!(sample_withdrawal_st().inputs().is_none());
    }

    #[test]
    fn test_active_version_range_legacy_transitions() {
        // These all report ALL_VERSIONS per the mod.rs table.
        assert_eq!(sample_transfer_st().active_version_range(), ALL_VERSIONS);
        assert_eq!(
            sample_masternode_vote_st().active_version_range(),
            ALL_VERSIONS
        );
        assert_eq!(sample_withdrawal_st().active_version_range(), ALL_VERSIONS);
    }

    #[test]
    fn test_unique_identifiers_non_empty() {
        let ids = sample_transfer_st().unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert!(!ids[0].is_empty());
    }

    #[test]
    fn test_required_asset_lock_balance_rejects_non_asset_lock() {
        let platform_version = PlatformVersion::latest();
        let st = sample_transfer_st();
        let err = st
            .required_asset_lock_balance_for_processing_start(platform_version)
            .expect_err("credit transfer is not an asset lock state transition");
        match err {
            ProtocolError::CorruptedCodeExecution(msg) => {
                assert!(
                    msg.contains("is not an asset lock transaction"),
                    "unexpected error message: {msg}"
                );
            }
            other => panic!("expected CorruptedCodeExecution, got {other:?}"),
        }
    }

    #[test]
    fn test_security_level_requirement_for_transfer() {
        // IdentityCreditTransfer requires CRITICAL at TRANSFER purpose.
        let st = sample_transfer_st();
        let levels = st
            .security_level_requirement(Purpose::TRANSFER)
            .expect("transfer state transition should return a requirement");
        assert_eq!(levels, vec![SecurityLevel::CRITICAL]);
    }

    #[test]
    fn test_purpose_requirement_for_transfer() {
        let st = sample_transfer_st();
        let purposes = st
            .purpose_requirement()
            .expect("transfer state transition should have a purpose");
        assert_eq!(purposes, vec![Purpose::TRANSFER]);
    }

    #[test]
    fn test_optional_asset_lock_proof_none_for_transfer() {
        let st = sample_transfer_st();
        assert!(st.optional_asset_lock_proof().is_none());
    }

    // -----------------------------------------------------------------------
    // Enum construction: From<V0 / outer enum> → StateTransition
    // -----------------------------------------------------------------------

    #[test]
    fn test_from_outer_enum_into_state_transition() {
        let outer: IdentityCreditTransferTransition =
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0::default());
        let st: StateTransition = outer.into();
        assert!(matches!(st, StateTransition::IdentityCreditTransfer(_)));
    }

    #[test]
    fn test_from_masternode_vote_outer_into_state_transition() {
        let outer: MasternodeVoteTransition =
            MasternodeVoteTransition::V0(MasternodeVoteTransitionV0::default());
        let st: StateTransition = outer.into();
        assert!(matches!(st, StateTransition::MasternodeVote(_)));
    }

    // -----------------------------------------------------------------------
    // Serialization round-trip: platform serialize / deserialize via enum.
    // Exercises the top-level `StateTransition` (de)serialize glue.
    // -----------------------------------------------------------------------

    #[test]
    fn test_state_transition_platform_serialize_roundtrip() {
        use crate::serialization::{PlatformDeserializable, PlatformSerializable};
        let original = sample_transfer_st();
        let bytes =
            PlatformSerializable::serialize_to_bytes(&original).expect("serialize should succeed");
        let restored =
            StateTransition::deserialize_from_bytes(&bytes).expect("deserialize should succeed");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_deserialize_from_bytes_in_version_succeeds_for_latest() {
        use crate::serialization::PlatformSerializable;
        let original = sample_transfer_st();
        let bytes =
            PlatformSerializable::serialize_to_bytes(&original).expect("serialize succeeds");
        let restored =
            StateTransition::deserialize_from_bytes_in_version(&bytes, PlatformVersion::latest())
                .expect("deserialize_from_bytes_in_version should succeed");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_transaction_id_is_deterministic() {
        let st = sample_transfer_st();
        let a = st.transaction_id().expect("hash should succeed");
        let b = st.transaction_id().expect("hash should succeed");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn test_transaction_id_changes_on_signature_change() {
        let mut st = sample_transfer_st();
        let before = st.transaction_id().expect("hash should succeed");
        st.set_signature(BinaryData::new(vec![0xbb; 65]));
        let after = st.transaction_id().expect("hash should succeed");
        // Different signatures produce a different serialized form.
        assert_ne!(before, after);
    }

    #[test]
    fn test_clone_preserves_inner_state() {
        let st = sample_transfer_st();
        let cloned = st.clone();
        assert_eq!(st, cloned);
    }

    // -----------------------------------------------------------------------
    // Additional coverage: enum arms that weren't previously exercised.
    //
    // The tests below intentionally target variants the earlier tests did not
    // touch (DataContractCreate, DataContractUpdate, Batch, IdentityCreate,
    // IdentityTopUp, IdentityUpdate, shielded / address variants) to cover
    // the remaining match-arm branches in accessor / mutator / classification
    // methods.
    // -----------------------------------------------------------------------

    use crate::data_contract::serialized_version::DataContractInSerializationFormat;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::document_delete_transition::{
        DocumentDeleteTransition, DocumentDeleteTransitionV0,
    };
    use crate::state_transition::batch_transition::{BatchTransition, BatchTransitionV0};
    use crate::state_transition::data_contract_create_transition::{
        DataContractCreateTransition, DataContractCreateTransitionV0,
    };
    use crate::state_transition::data_contract_update_transition::{
        DataContractUpdateTransition, DataContractUpdateTransitionV0,
    };
    use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use crate::state_transition::identity_create_transition::IdentityCreateTransition;
    use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
    use crate::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
    use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
    use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;
    use crate::state_transition::unshield_transition::UnshieldTransition;

    /// Build a DataContractInSerializationFormat from a crate-private v0
    /// constructor via the public TryFromPlatformVersioned impl and DataContract V1.
    fn sample_data_contract_in_serialization_format() -> DataContractInSerializationFormat {
        use crate::data_contract::config::v0::DataContractConfigV0;
        use crate::data_contract::config::DataContractConfig;
        use crate::data_contract::v1::DataContractV1;
        use crate::data_contract::DataContract;
        use platform_version::TryIntoPlatformVersioned;
        use std::collections::BTreeMap;

        let contract = DataContract::V1(DataContractV1 {
            id: Identifier::from([9u8; 32]),
            version: 1,
            owner_id: Identifier::from([7u8; 32]),
            document_types: BTreeMap::new(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::new(),
            tokens: BTreeMap::new(),
            keywords: Vec::new(),
            description: None,
        });

        contract
            .try_into_platform_versioned(PlatformVersion::latest())
            .expect("expected to serialize a trivial contract")
    }

    fn sample_data_contract_create_st() -> StateTransition {
        StateTransition::DataContractCreate(DataContractCreateTransition::V0(
            DataContractCreateTransitionV0 {
                data_contract: sample_data_contract_in_serialization_format(),
                identity_nonce: 1,
                user_fee_increase: 5,
                signature_public_key_id: 2,
                signature: BinaryData::new(vec![0xAB; 65]),
            },
        ))
    }

    fn sample_data_contract_update_st() -> StateTransition {
        StateTransition::DataContractUpdate(DataContractUpdateTransition::V0(
            DataContractUpdateTransitionV0 {
                identity_contract_nonce: 4,
                data_contract: sample_data_contract_in_serialization_format(),
                user_fee_increase: 9,
                signature_public_key_id: 6,
                signature: BinaryData::new(vec![0xCD; 65]),
            },
        ))
    }

    fn sample_batch_st_with_delete() -> StateTransition {
        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([1u8; 32]),
            identity_contract_nonce: 3,
            document_type_name: "preorder".to_string(),
            data_contract_id: Identifier::from([2u8; 32]),
        });
        let delete =
            DocumentTransition::Delete(DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
                base,
            }));
        StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Identifier::from([8u8; 32]),
            transitions: vec![delete],
            user_fee_increase: 2,
            signature_public_key_id: 7,
            signature: BinaryData::new(vec![0xEE; 65]),
        }))
    }

    fn sample_batch_st_empty() -> StateTransition {
        StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Identifier::from([1u8; 32]),
            transitions: vec![],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![]),
        }))
    }

    fn sample_identity_create_st() -> StateTransition {
        StateTransition::IdentityCreate(IdentityCreateTransition::V0(IdentityCreateTransitionV0 {
            identity_id: Identifier::from([3u8; 32]),
            ..Default::default()
        }))
    }

    fn sample_identity_top_up_st() -> StateTransition {
        StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(IdentityTopUpTransitionV0 {
            identity_id: Identifier::from([4u8; 32]),
            ..Default::default()
        }))
    }

    fn sample_identity_update_st() -> StateTransition {
        StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(IdentityUpdateTransitionV0 {
            identity_id: Identifier::from([5u8; 32]),
            revision: 1,
            nonce: 2,
            add_public_keys: vec![],
            disable_public_keys: vec![],
            user_fee_increase: 11,
            signature_public_key_id: 33,
            signature: BinaryData::new(vec![0xFF; 65]),
        }))
    }

    fn sample_unshield_st() -> StateTransition {
        StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address: Default::default(),
            actions: vec![],
            unshielding_amount: 0,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
        }))
    }

    fn sample_shielded_transfer_st() -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
            },
        ))
    }

    fn sample_shielded_withdrawal_st() -> StateTransition {
        use crate::identity::core_script::CoreScript;
        use crate::withdrawal::Pooling;
        StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
            ShieldedWithdrawalTransitionV0 {
                actions: vec![],
                unshielding_amount: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::from_bytes(vec![]),
            },
        ))
    }

    // --- name() covers all previously-untested arms, including the nested
    // match for Batch variants. ---
    #[test]
    fn test_name_for_newly_covered_variants() {
        assert_eq!(
            sample_data_contract_create_st().name(),
            "DataContractCreate"
        );
        assert_eq!(
            sample_data_contract_update_st().name(),
            "DataContractUpdate"
        );
        assert_eq!(sample_identity_create_st().name(), "IdentityCreate");
        assert_eq!(sample_identity_top_up_st().name(), "IdentityTopUp");
        assert_eq!(sample_identity_update_st().name(), "IdentityUpdate");
        assert_eq!(sample_unshield_st().name(), "Unshield");
        assert_eq!(sample_shielded_transfer_st().name(), "ShieldedTransfer");
        assert_eq!(sample_shielded_withdrawal_st().name(), "ShieldedWithdrawal");

        // Batch with a single Delete – exercises the nested DocumentTransition
        // match arm in `name()`.
        let batch_name = sample_batch_st_with_delete().name();
        assert_eq!(batch_name, "DocumentsBatch([Delete])");

        // Empty batch – still renders, with an empty list.
        let empty_name = sample_batch_st_empty().name();
        assert_eq!(empty_name, "DocumentsBatch([])");
    }

    // --- state_transition_type covers the call_method! dispatch. ---
    #[test]
    fn test_state_transition_type_for_newly_covered_variants() {
        assert_eq!(
            sample_data_contract_create_st().state_transition_type(),
            StateTransitionType::DataContractCreate
        );
        assert_eq!(
            sample_data_contract_update_st().state_transition_type(),
            StateTransitionType::DataContractUpdate
        );
        assert_eq!(
            sample_batch_st_with_delete().state_transition_type(),
            StateTransitionType::Batch
        );
        assert_eq!(
            sample_identity_create_st().state_transition_type(),
            StateTransitionType::IdentityCreate
        );
        assert_eq!(
            sample_identity_top_up_st().state_transition_type(),
            StateTransitionType::IdentityTopUp
        );
        assert_eq!(
            sample_identity_update_st().state_transition_type(),
            StateTransitionType::IdentityUpdate
        );
        assert_eq!(
            sample_unshield_st().state_transition_type(),
            StateTransitionType::Unshield
        );
        assert_eq!(
            sample_shielded_transfer_st().state_transition_type(),
            StateTransitionType::ShieldedTransfer
        );
        assert_eq!(
            sample_shielded_withdrawal_st().state_transition_type(),
            StateTransitionType::ShieldedWithdrawal
        );
    }

    // --- active_version_range uses different branches per transition
    // "group". Exercises the contract-format V1 branch for
    // DataContractCreate/Update (9..=LATEST), the BatchTransitionV0 branch
    // (ALL_VERSIONS), and the shielded range (12..=LATEST).
    #[test]
    fn test_active_version_range_contract_and_shielded_branches() {
        // DataContractCreate/Update on PlatformVersion::latest use the V1
        // contract serialization format, which restricts active range.
        let contract_v1_range = 9..=LATEST_VERSION;
        assert_eq!(
            sample_data_contract_create_st().active_version_range(),
            contract_v1_range
        );
        let contract_v1_range = 9..=LATEST_VERSION;
        assert_eq!(
            sample_data_contract_update_st().active_version_range(),
            contract_v1_range
        );
        // BatchTransition::V0 → ALL_VERSIONS
        assert_eq!(
            sample_batch_st_with_delete().active_version_range(),
            ALL_VERSIONS
        );
        // IdentityCreate/TopUp/Update are ALL_VERSIONS.
        assert_eq!(
            sample_identity_create_st().active_version_range(),
            ALL_VERSIONS
        );
        assert_eq!(
            sample_identity_top_up_st().active_version_range(),
            ALL_VERSIONS
        );
        assert_eq!(
            sample_identity_update_st().active_version_range(),
            ALL_VERSIONS
        );
        // Shielded variants report a shielded range (12..=LATEST_VERSION).
        let shielded_range = 12..=LATEST_VERSION;
        assert_eq!(
            sample_shielded_transfer_st().active_version_range(),
            shielded_range.clone()
        );
        assert_eq!(
            sample_unshield_st().active_version_range(),
            shielded_range.clone()
        );
        assert_eq!(
            sample_shielded_withdrawal_st().active_version_range(),
            shielded_range
        );
    }

    // --- is_identity_signed exercises the inverted-match logic for the
    // shielded / identity-create / topup variants. ---
    #[test]
    fn test_is_identity_signed_false_for_identity_create_topup_and_shielded() {
        assert!(!sample_identity_create_st().is_identity_signed());
        assert!(!sample_identity_top_up_st().is_identity_signed());
        assert!(!sample_unshield_st().is_identity_signed());
        assert!(!sample_shielded_transfer_st().is_identity_signed());
        assert!(!sample_shielded_withdrawal_st().is_identity_signed());
    }

    // --- signature accessor for each arm that returns Some/None; previously
    // only IdentityCreditTransfer / MasternodeVote / IdentityCreditWithdrawal
    // were covered.
    #[test]
    fn test_signature_accessor_for_other_variants() {
        // Some(_) arms
        assert_eq!(
            sample_data_contract_create_st().signature().unwrap().len(),
            65
        );
        assert_eq!(
            sample_data_contract_update_st().signature().unwrap().len(),
            65
        );
        assert_eq!(sample_batch_st_with_delete().signature().unwrap().len(), 65);
        assert_eq!(sample_identity_update_st().signature().unwrap().len(), 65);

        // None arms for address / shielded variants.
        assert!(sample_unshield_st().signature().is_none());
        assert!(sample_shielded_transfer_st().signature().is_none());
        assert!(sample_shielded_withdrawal_st().signature().is_none());
    }

    // --- owner_id accessor for each arm.
    #[test]
    fn test_owner_id_accessor_for_other_variants() {
        assert_eq!(
            sample_data_contract_create_st().owner_id(),
            Some(Identifier::from([7u8; 32]))
        );
        assert_eq!(
            sample_data_contract_update_st().owner_id(),
            Some(Identifier::from([7u8; 32]))
        );
        assert_eq!(
            sample_batch_st_with_delete().owner_id(),
            Some(Identifier::from([8u8; 32]))
        );
        assert_eq!(
            sample_identity_update_st().owner_id(),
            Some(Identifier::from([5u8; 32]))
        );
        // These variants unconditionally return None.
        assert!(sample_unshield_st().owner_id().is_none());
        assert!(sample_shielded_transfer_st().owner_id().is_none());
        assert!(sample_shielded_withdrawal_st().owner_id().is_none());
    }

    // --- user_fee_increase accessor — includes arms that return 0
    // unconditionally (shielded/masternode) vs the variants' stored value.
    #[test]
    fn test_user_fee_increase_for_newly_covered_variants() {
        assert_eq!(sample_data_contract_create_st().user_fee_increase(), 5);
        assert_eq!(sample_data_contract_update_st().user_fee_increase(), 9);
        assert_eq!(sample_batch_st_with_delete().user_fee_increase(), 2);
        assert_eq!(sample_identity_update_st().user_fee_increase(), 11);
        // Unconditionally 0 for shielded.
        assert_eq!(sample_shielded_transfer_st().user_fee_increase(), 0);
        assert_eq!(sample_shielded_withdrawal_st().user_fee_increase(), 0);
        assert_eq!(sample_unshield_st().user_fee_increase(), 0);
    }

    // --- set_user_fee_increase for the no-op shielded arms and for the
    // transitions that actually do store the value.
    #[test]
    fn test_set_user_fee_increase_for_newly_covered_variants() {
        let mut st = sample_data_contract_create_st();
        st.set_user_fee_increase(42);
        assert_eq!(st.user_fee_increase(), 42);

        let mut st = sample_data_contract_update_st();
        st.set_user_fee_increase(13);
        assert_eq!(st.user_fee_increase(), 13);

        let mut st = sample_batch_st_with_delete();
        st.set_user_fee_increase(101);
        assert_eq!(st.user_fee_increase(), 101);

        let mut st = sample_identity_update_st();
        st.set_user_fee_increase(77);
        assert_eq!(st.user_fee_increase(), 77);

        // Shielded no-ops: value stays 0.
        let mut shielded = sample_shielded_transfer_st();
        shielded.set_user_fee_increase(99);
        assert_eq!(shielded.user_fee_increase(), 0);

        let mut withdrawal = sample_shielded_withdrawal_st();
        withdrawal.set_user_fee_increase(99);
        assert_eq!(withdrawal.user_fee_increase(), 0);

        let mut unshield = sample_unshield_st();
        unshield.set_user_fee_increase(99);
        assert_eq!(unshield.user_fee_increase(), 0);
    }

    // --- set_signature: exercises the `true` arms we didn't test before
    // (DataContractCreate/Update/Batch/IdentityUpdate) and the `false` arms
    // (shielded transitions).
    #[test]
    fn test_set_signature_false_for_shielded_and_identity_create_topup() {
        // `false` arms: shield*, shielded*, unshield, address* (no-op, returns false).
        let mut st = sample_unshield_st();
        assert!(!st.set_signature(BinaryData::new(vec![0xAB; 65])));
        let mut st = sample_shielded_transfer_st();
        assert!(!st.set_signature(BinaryData::new(vec![0xAB; 65])));
        let mut st = sample_shielded_withdrawal_st();
        assert!(!st.set_signature(BinaryData::new(vec![0xAB; 65])));
    }

    #[test]
    fn test_set_signature_true_for_newly_covered_variants() {
        let mut st = sample_data_contract_create_st();
        assert!(st.set_signature(BinaryData::new(vec![0x11; 65])));
        assert_eq!(st.signature().unwrap().as_slice(), &[0x11; 65]);

        let mut st = sample_data_contract_update_st();
        assert!(st.set_signature(BinaryData::new(vec![0x22; 65])));
        assert_eq!(st.signature().unwrap().as_slice(), &[0x22; 65]);

        let mut st = sample_batch_st_with_delete();
        assert!(st.set_signature(BinaryData::new(vec![0x33; 65])));
        assert_eq!(st.signature().unwrap().as_slice(), &[0x33; 65]);

        let mut st = sample_identity_update_st();
        assert!(st.set_signature(BinaryData::new(vec![0x44; 65])));
        assert_eq!(st.signature().unwrap().as_slice(), &[0x44; 65]);
    }

    // --- signature_public_key_id: identity-signed arms return Some, others
    // (shielded/identity-create/topup/address) return None.
    #[test]
    fn test_signature_public_key_id_returns_none_for_non_signed() {
        // IdentityCreate / IdentityTopUp / shielded / address variants are all
        // "not identity-signed" and return None.
        assert!(sample_identity_create_st()
            .signature_public_key_id()
            .is_none());
        assert!(sample_identity_top_up_st()
            .signature_public_key_id()
            .is_none());
        assert!(sample_unshield_st().signature_public_key_id().is_none());
        assert!(sample_shielded_transfer_st()
            .signature_public_key_id()
            .is_none());
        assert!(sample_shielded_withdrawal_st()
            .signature_public_key_id()
            .is_none());
    }

    #[test]
    fn test_signature_public_key_id_for_signed_variants() {
        assert_eq!(
            sample_data_contract_create_st().signature_public_key_id(),
            Some(2)
        );
        assert_eq!(
            sample_data_contract_update_st().signature_public_key_id(),
            Some(6)
        );
        assert_eq!(
            sample_batch_st_with_delete().signature_public_key_id(),
            Some(7)
        );
        assert_eq!(
            sample_identity_update_st().signature_public_key_id(),
            Some(33)
        );
    }

    // --- set_signature_public_key_id: no-op for IdentityCreate/TopUp and
    // shielded variants; updates for identity-signed variants. ---
    #[test]
    fn test_set_signature_public_key_id_noop_for_non_signed() {
        // These variants are not identity-signed; setter is a no-op in the
        // call_method_identity_signed! macro.
        let mut st = sample_identity_create_st();
        st.set_signature_public_key_id(100);
        assert_eq!(st.signature_public_key_id(), None);

        let mut st = sample_identity_top_up_st();
        st.set_signature_public_key_id(100);
        assert_eq!(st.signature_public_key_id(), None);

        let mut st = sample_unshield_st();
        st.set_signature_public_key_id(100);
        assert_eq!(st.signature_public_key_id(), None);
    }

    #[test]
    fn test_set_signature_public_key_id_updates_for_signed_variants() {
        let mut st = sample_data_contract_create_st();
        st.set_signature_public_key_id(42);
        assert_eq!(st.signature_public_key_id(), Some(42));

        let mut st = sample_batch_st_with_delete();
        st.set_signature_public_key_id(43);
        assert_eq!(st.signature_public_key_id(), Some(43));

        let mut st = sample_identity_update_st();
        st.set_signature_public_key_id(44);
        assert_eq!(st.signature_public_key_id(), Some(44));
    }

    // --- required_number_of_private_keys defaults to 1 for "signed" variants
    // and 0 for shielded ones.
    #[test]
    fn test_required_number_of_private_keys_various_variants() {
        assert_eq!(
            sample_data_contract_create_st().required_number_of_private_keys(),
            1
        );
        assert_eq!(
            sample_data_contract_update_st().required_number_of_private_keys(),
            1
        );
        assert_eq!(
            sample_batch_st_with_delete().required_number_of_private_keys(),
            1
        );
        assert_eq!(
            sample_identity_update_st().required_number_of_private_keys(),
            1
        );
        assert_eq!(
            sample_identity_create_st().required_number_of_private_keys(),
            1
        );
        // Shielded variants return 0 unconditionally.
        assert_eq!(
            sample_shielded_transfer_st().required_number_of_private_keys(),
            0
        );
        assert_eq!(
            sample_shielded_withdrawal_st().required_number_of_private_keys(),
            0
        );
        assert_eq!(sample_unshield_st().required_number_of_private_keys(), 0);
    }

    // --- inputs(): None for all these variants (covers the big
    // wildcard/None arm in the match).
    #[test]
    fn test_inputs_none_for_many_variants() {
        assert!(sample_data_contract_create_st().inputs().is_none());
        assert!(sample_data_contract_update_st().inputs().is_none());
        assert!(sample_batch_st_with_delete().inputs().is_none());
        assert!(sample_identity_create_st().inputs().is_none());
        assert!(sample_identity_top_up_st().inputs().is_none());
        assert!(sample_identity_update_st().inputs().is_none());
        // Shielded variants also return None for inputs().
        assert!(sample_unshield_st().inputs().is_none());
        assert!(sample_shielded_transfer_st().inputs().is_none());
        assert!(sample_shielded_withdrawal_st().inputs().is_none());
    }

    // --- optional_asset_lock_proof: None for everything that isn't
    // IdentityCreate / IdentityTopUp / ShieldFromAssetLock. The IdentityCreate
    // default contains the asset lock proof field, so this forwards to its
    // implementation.
    #[test]
    fn test_optional_asset_lock_proof_returns_none_for_wildcard_arms() {
        assert!(sample_data_contract_create_st()
            .optional_asset_lock_proof()
            .is_none());
        assert!(sample_data_contract_update_st()
            .optional_asset_lock_proof()
            .is_none());
        assert!(sample_batch_st_with_delete()
            .optional_asset_lock_proof()
            .is_none());
        assert!(sample_identity_update_st()
            .optional_asset_lock_proof()
            .is_none());
        assert!(sample_unshield_st().optional_asset_lock_proof().is_none());
        assert!(sample_shielded_transfer_st()
            .optional_asset_lock_proof()
            .is_none());
        assert!(sample_shielded_withdrawal_st()
            .optional_asset_lock_proof()
            .is_none());
    }

    // --- required_asset_lock_balance_for_processing_start returns an
    // CorruptedCodeExecution error for non asset-lock variants. Exercise
    // additional arms beyond what the original transfer test covered.
    #[test]
    fn test_required_asset_lock_balance_errors_for_other_non_asset_lock_variants() {
        let platform_version = PlatformVersion::latest();

        let cases: Vec<(&str, StateTransition)> = vec![
            ("DataContractCreate", sample_data_contract_create_st()),
            ("DataContractUpdate", sample_data_contract_update_st()),
            ("Batch", sample_batch_st_with_delete()),
            ("IdentityUpdate", sample_identity_update_st()),
            ("MasternodeVote", sample_masternode_vote_st()),
            ("Unshield", sample_unshield_st()),
            ("ShieldedTransfer", sample_shielded_transfer_st()),
            ("ShieldedWithdrawal", sample_shielded_withdrawal_st()),
        ];

        for (label, st) in cases {
            let err = st
                .required_asset_lock_balance_for_processing_start(platform_version)
                .expect_err(&format!("expected error for {label}"));
            match err {
                ProtocolError::CorruptedCodeExecution(msg) => {
                    assert!(
                        msg.contains("is not an asset lock transaction"),
                        "unexpected error for {label}: {msg}"
                    );
                }
                other => panic!("expected CorruptedCodeExecution for {label}, got {other:?}"),
            }
        }
    }

    // --- unique_identifiers: covers the call_method! dispatch for arms
    // beyond credit transfer. Each variant's `unique_identifiers`
    // implementation returns a non-empty vector; the individual identifier
    // strings may be empty for some variants whose IDs are encoded as empty
    // (this method simply shouldn't panic or short-circuit).
    #[test]
    fn test_unique_identifiers_non_empty_for_other_variants() {
        for st in [
            sample_data_contract_create_st(),
            sample_data_contract_update_st(),
            sample_batch_st_with_delete(),
            sample_identity_create_st(),
            sample_identity_top_up_st(),
            sample_identity_update_st(),
        ] {
            let ids = st.unique_identifiers();
            assert!(!ids.is_empty(), "unique_identifiers should not be empty");
        }
    }

    // --- security_level_requirement returns None for identity-create/topup
    // and for every shielded/address variant. This hits the None arms in
    // call_getter_method_identity_signed!.
    #[test]
    fn test_security_level_requirement_returns_none_for_non_signed_variants() {
        let purpose = Purpose::AUTHENTICATION;
        assert!(sample_identity_create_st()
            .security_level_requirement(purpose)
            .is_none());
        assert!(sample_identity_top_up_st()
            .security_level_requirement(purpose)
            .is_none());
        assert!(sample_unshield_st()
            .security_level_requirement(purpose)
            .is_none());
        assert!(sample_shielded_transfer_st()
            .security_level_requirement(purpose)
            .is_none());
        assert!(sample_shielded_withdrawal_st()
            .security_level_requirement(purpose)
            .is_none());
    }

    #[test]
    fn test_purpose_requirement_returns_none_for_non_signed_variants() {
        assert!(sample_identity_create_st().purpose_requirement().is_none());
        assert!(sample_identity_top_up_st().purpose_requirement().is_none());
        assert!(sample_unshield_st().purpose_requirement().is_none());
        assert!(sample_shielded_transfer_st()
            .purpose_requirement()
            .is_none());
        assert!(sample_shielded_withdrawal_st()
            .purpose_requirement()
            .is_none());
    }

    // --- From impls: each From<Outer> → StateTransition uses `derive_more::From`.
    #[test]
    fn test_from_outer_data_contract_create_into_state_transition() {
        let outer: DataContractCreateTransition =
            DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
                data_contract: sample_data_contract_in_serialization_format(),
                identity_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            });
        let st: StateTransition = outer.into();
        assert!(matches!(st, StateTransition::DataContractCreate(_)));
    }

    #[test]
    fn test_from_outer_data_contract_update_into_state_transition() {
        let outer: DataContractUpdateTransition =
            DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
                identity_contract_nonce: 2,
                data_contract: sample_data_contract_in_serialization_format(),
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            });
        let st: StateTransition = outer.into();
        assert!(matches!(st, StateTransition::DataContractUpdate(_)));
    }

    #[test]
    fn test_from_outer_batch_into_state_transition() {
        let outer: BatchTransition = BatchTransition::V0(BatchTransitionV0::default());
        let st: StateTransition = outer.into();
        assert!(matches!(st, StateTransition::Batch(_)));
    }

    #[test]
    fn test_from_outer_identity_create_into_state_transition() {
        let outer: IdentityCreateTransition =
            IdentityCreateTransition::V0(IdentityCreateTransitionV0::default());
        let st: StateTransition = outer.into();
        assert!(matches!(st, StateTransition::IdentityCreate(_)));
    }

    #[test]
    fn test_from_outer_identity_update_into_state_transition() {
        let outer: IdentityUpdateTransition =
            IdentityUpdateTransition::V0(IdentityUpdateTransitionV0::default());
        let st: StateTransition = outer.into();
        assert!(matches!(st, StateTransition::IdentityUpdate(_)));
    }

    // --- transaction_id + clone for additional variants — triggers the
    // serialize path for each arm.
    #[test]
    fn test_transaction_id_and_clone_for_identity_update() {
        let st = sample_identity_update_st();
        let id_a = st.transaction_id().expect("hash should succeed");
        let cloned = st.clone();
        let id_b = cloned.transaction_id().expect("hash should succeed");
        assert_eq!(id_a, id_b);
        assert_eq!(id_a.len(), 32);
    }

    #[test]
    fn test_transaction_id_and_clone_for_data_contract_create() {
        let st = sample_data_contract_create_st();
        let id_a = st.transaction_id().expect("hash should succeed");
        let cloned = st.clone();
        let id_b = cloned.transaction_id().expect("hash should succeed");
        assert_eq!(id_a, id_b);
        assert_eq!(id_a.len(), 32);
    }

    // --- serialize round-trip for variants beyond credit transfer. ---
    #[test]
    fn test_serialize_roundtrip_identity_update() {
        use crate::serialization::{PlatformDeserializable, PlatformSerializable};
        let original = sample_identity_update_st();
        let bytes =
            PlatformSerializable::serialize_to_bytes(&original).expect("serialize should succeed");
        let restored =
            StateTransition::deserialize_from_bytes(&bytes).expect("deserialize should succeed");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_serialize_roundtrip_data_contract_update() {
        use crate::serialization::{PlatformDeserializable, PlatformSerializable};
        let original = sample_data_contract_update_st();
        let bytes =
            PlatformSerializable::serialize_to_bytes(&original).expect("serialize should succeed");
        let restored =
            StateTransition::deserialize_from_bytes(&bytes).expect("deserialize should succeed");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_serialize_roundtrip_batch_empty() {
        use crate::serialization::{PlatformDeserializable, PlatformSerializable};
        let original = sample_batch_st_empty();
        let bytes =
            PlatformSerializable::serialize_to_bytes(&original).expect("serialize should succeed");
        let restored =
            StateTransition::deserialize_from_bytes(&bytes).expect("deserialize should succeed");
        assert_eq!(original, restored);
    }

    // --- deserialize_from_bytes_in_version error path: craft bytes for a
    // variant whose `active_version_range()` starts at 11 or 12 and then
    // attempt to deserialize them with a PlatformVersion whose protocol
    // version is below that range. Exercises the
    // `StateTransitionIsNotActiveError` arm.
    // ---
    #[cfg(all(feature = "state-transitions", feature = "validation"))]
    #[test]
    fn test_deserialize_from_bytes_in_version_returns_not_active_error() {
        use crate::serialization::PlatformSerializable;

        // ShieldedTransfer has active_version_range = 12..=LATEST_VERSION.
        let original = sample_shielded_transfer_st();
        let bytes =
            PlatformSerializable::serialize_to_bytes(&original).expect("serialize succeeds");

        // Find a real PlatformVersion whose protocol_version is < 12 so the
        // range check rejects it. PlatformVersion::get(1) corresponds to
        // protocol version 1 which is guaranteed below any shielded range.
        let low_version = PlatformVersion::get(1).expect("platform version 1 exists");
        assert!(
            low_version.protocol_version < 12,
            "expected sub-12 version for this test, got {}",
            low_version.protocol_version
        );

        let err = StateTransition::deserialize_from_bytes_in_version(&bytes, low_version)
            .expect_err("expected StateTransitionIsNotActiveError for sub-12 protocol");
        match err {
            ProtocolError::StateTransitionError(
                crate::state_transition::errors::StateTransitionError::StateTransitionIsNotActiveError {
                    state_transition_type,
                    active_version_range,
                    current_protocol_version,
                },
            ) => {
                assert_eq!(state_transition_type, "ShieldedTransfer");
                assert_eq!(current_protocol_version, low_version.protocol_version);
                assert!(active_version_range.start() >= &12);
            }
            other => panic!("expected StateTransitionIsNotActiveError, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Additional coverage: variants not yet exercised.
    //
    // The tests below target:
    //   * IdentityCreditWithdrawal::V1 (previously only V0 was covered).
    //   * IdentityCreditTransferToAddresses (its own top-level arm).
    //   * AddressFundingFromAssetLock (identity-signed + asset-lock arm).
    //   * AddressCreditWithdrawal (not identity-signed, address-funds arm).
    //   * ShieldFromAssetLock (asset-lock, non-identity-signed).
    //   * Batch with a token transition (nested enum, previously only Delete).
    // -----------------------------------------------------------------------

    use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
    use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
    use crate::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
    use crate::withdrawal::Pooling as WithdrawalPooling;

    fn sample_withdrawal_v1_st() -> StateTransition {
        let v1 = IdentityCreditWithdrawalTransitionV1 {
            identity_id: Identifier::from([12u8; 32]),
            amount: 777,
            core_fee_per_byte: 2,
            pooling: WithdrawalPooling::Standard,
            output_script: None,
            nonce: 9,
            user_fee_increase: 4,
            signature_public_key_id: 21,
            signature: BinaryData::new(vec![0x12; 65]),
        };
        StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V1(v1))
    }

    fn sample_credit_transfer_to_addresses_st() -> StateTransition {
        let v0 = IdentityCreditTransferToAddressesTransitionV0 {
            identity_id: Identifier::from([13u8; 32]),
            ..Default::default()
        };
        StateTransition::IdentityCreditTransferToAddresses(
            IdentityCreditTransferToAddressesTransition::V0(v0),
        )
    }

    fn sample_address_credit_withdrawal_st() -> StateTransition {
        use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
        StateTransition::AddressCreditWithdrawal(AddressCreditWithdrawalTransition::V0(
            AddressCreditWithdrawalTransitionV0::default(),
        ))
    }

    fn sample_shield_from_asset_lock_st() -> StateTransition {
        use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: Default::default(),
                actions: vec![],
                value_balance: 100,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                surplus_output: None,
                signature: BinaryData::new(vec![0x55; 65]),
            },
        ))
    }

    // ---------- IdentityCreditWithdrawal V1 accessors ----------

    #[test]
    fn test_withdrawal_v1_name_and_type() {
        let st = sample_withdrawal_v1_st();
        assert_eq!(st.name(), "IdentityCreditWithdrawal");
        assert_eq!(
            st.state_transition_type(),
            StateTransitionType::IdentityCreditWithdrawal
        );
    }

    #[test]
    fn test_withdrawal_v1_is_identity_signed_true() {
        assert!(sample_withdrawal_v1_st().is_identity_signed());
    }

    #[test]
    fn test_withdrawal_v1_signature_and_owner_and_key_id() {
        let st = sample_withdrawal_v1_st();
        // signature accessor -> Some
        let sig = st.signature().expect("V1 withdrawal has a signature");
        assert_eq!(sig.as_slice(), &[0x12; 65]);
        // owner_id delegates to identity_id
        assert_eq!(st.owner_id(), Some(Identifier::from([12u8; 32])));
        assert_eq!(st.signature_public_key_id(), Some(21));
        assert_eq!(st.user_fee_increase(), 4);
    }

    #[test]
    fn test_withdrawal_v1_set_signature_and_fee_and_key_id() {
        let mut st = sample_withdrawal_v1_st();
        assert!(st.set_signature(BinaryData::new(vec![0x99; 65])));
        assert_eq!(st.signature().unwrap().as_slice(), &[0x99; 65]);

        st.set_user_fee_increase(33);
        assert_eq!(st.user_fee_increase(), 33);

        st.set_signature_public_key_id(64);
        assert_eq!(st.signature_public_key_id(), Some(64));
    }

    #[test]
    fn test_withdrawal_v1_serialize_roundtrip_via_state_transition() {
        use crate::serialization::{PlatformDeserializable, PlatformSerializable};
        let original = sample_withdrawal_v1_st();
        let bytes = PlatformSerializable::serialize_to_bytes(&original).expect("serialize ok");
        let restored = StateTransition::deserialize_from_bytes(&bytes).expect("deserialize ok");
        assert_eq!(original, restored);
        // The restored variant must still be V1, not V0 — exercises the
        // feature-version dispatch in deserialize.
        match restored {
            StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V1(
                _,
            )) => {}
            other => panic!("expected V1 inner variant, got: {:?}", other),
        }
    }

    #[test]
    fn test_withdrawal_v1_transaction_id_differs_from_v0() {
        // V0 and V1 carry different serialized forms → distinct transaction ids.
        let v0 = sample_withdrawal_st();
        let v1 = sample_withdrawal_v1_st();
        let id_v0 = v0.transaction_id().expect("v0 hash");
        let id_v1 = v1.transaction_id().expect("v1 hash");
        assert_ne!(id_v0, id_v1);
    }

    #[test]
    fn test_withdrawal_v1_required_asset_lock_balance_errors() {
        let err = sample_withdrawal_v1_st()
            .required_asset_lock_balance_for_processing_start(PlatformVersion::latest())
            .expect_err("withdrawal is not an asset lock ST");
        matches!(err, ProtocolError::CorruptedCodeExecution(_));
    }

    // ---------- IdentityCreditTransferToAddresses ----------

    #[test]
    fn test_credit_transfer_to_addresses_name_and_type() {
        let st = sample_credit_transfer_to_addresses_st();
        assert_eq!(st.name(), "IdentityCreditTransferToAddresses");
        assert_eq!(
            st.state_transition_type(),
            StateTransitionType::IdentityCreditTransferToAddresses
        );
    }

    #[test]
    fn test_credit_transfer_to_addresses_signature_some_and_owner_some() {
        let st = sample_credit_transfer_to_addresses_st();
        assert!(st.signature().is_some(), "has a signature field");
        assert_eq!(st.owner_id(), Some(Identifier::from([13u8; 32])));
    }

    #[test]
    fn test_credit_transfer_to_addresses_is_identity_signed_true() {
        // Not in the "not identity signed" list → should be true.
        assert!(sample_credit_transfer_to_addresses_st().is_identity_signed());
    }

    #[test]
    fn test_credit_transfer_to_addresses_inputs_none_active_range_11_latest() {
        let st = sample_credit_transfer_to_addresses_st();
        assert!(st.inputs().is_none());
        // Per mod.rs table: this variant is in the 11..=LATEST_VERSION group.
        let range = st.active_version_range();
        assert_eq!(*range.start(), 11);
        assert_eq!(*range.end(), LATEST_VERSION);
    }

    #[test]
    fn test_credit_transfer_to_addresses_set_signature_returns_true() {
        let mut st = sample_credit_transfer_to_addresses_st();
        let ok = st.set_signature(BinaryData::new(vec![0x77; 65]));
        assert!(ok);
        assert_eq!(st.signature().unwrap().as_slice(), &[0x77; 65]);
    }

    #[test]
    fn test_credit_transfer_to_addresses_from_outer_enum() {
        let outer = IdentityCreditTransferToAddressesTransition::V0(
            IdentityCreditTransferToAddressesTransitionV0::default(),
        );
        let st: StateTransition = outer.into();
        assert!(matches!(
            st,
            StateTransition::IdentityCreditTransferToAddresses(_)
        ));
    }

    #[test]
    fn test_credit_transfer_to_addresses_user_fee_increase_setter() {
        let mut st = sample_credit_transfer_to_addresses_st();
        st.set_user_fee_increase(55);
        assert_eq!(st.user_fee_increase(), 55);
    }

    // ---------- AddressCreditWithdrawal ----------

    #[test]
    fn test_address_credit_withdrawal_name_type_and_accessors() {
        let st = sample_address_credit_withdrawal_st();
        assert_eq!(st.name(), "AddressCreditWithdrawal");
        assert_eq!(
            st.state_transition_type(),
            StateTransitionType::AddressCreditWithdrawal
        );
        // signature is None for AddressCreditWithdrawal (see mod.rs arm).
        assert!(st.signature().is_none());
        // owner_id is None for every address-* variant.
        assert!(st.owner_id().is_none());
        // inputs → Some (delegated to inner struct's inputs map, may be empty).
        assert!(st.inputs().is_some());
    }

    #[test]
    fn test_address_credit_withdrawal_set_signature_returns_false() {
        let mut st = sample_address_credit_withdrawal_st();
        assert!(!st.set_signature(BinaryData::new(vec![0xAB; 65])));
    }

    #[test]
    fn test_address_credit_withdrawal_is_identity_signed_true() {
        // Per mod.rs: `is_identity_signed` is !matches!(identity_create/topup/shield*/unshield/shielded*)
        // so address-* variants return true — even though signature() returns None.
        assert!(sample_address_credit_withdrawal_st().is_identity_signed());
    }

    #[test]
    fn test_address_credit_withdrawal_active_range_is_11_latest() {
        let range = sample_address_credit_withdrawal_st().active_version_range();
        assert_eq!(*range.start(), 11);
        assert_eq!(*range.end(), LATEST_VERSION);
    }

    // ---------- ShieldFromAssetLock ----------

    #[test]
    fn test_shield_from_asset_lock_name_type_and_accessors() {
        let st = sample_shield_from_asset_lock_st();
        assert_eq!(st.name(), "ShieldFromAssetLock");
        assert_eq!(
            st.state_transition_type(),
            StateTransitionType::ShieldFromAssetLock
        );
        // signature IS present on ShieldFromAssetLock — Some arm.
        let sig = st
            .signature()
            .expect("shield-from-asset-lock has signature");
        assert_eq!(sig.as_slice(), &[0x55; 65]);
        // owner_id is always None for shielded-* arms.
        assert!(st.owner_id().is_none());
    }

    #[test]
    fn test_shield_from_asset_lock_is_not_identity_signed() {
        assert!(!sample_shield_from_asset_lock_st().is_identity_signed());
    }

    #[test]
    fn test_shield_from_asset_lock_optional_asset_lock_proof_some() {
        // Critical: this is one of the THREE arms where optional_asset_lock_proof
        // actually forwards to Some(_). Other Some arms are covered by
        // IdentityCreate and IdentityTopUp which have default asset lock proof.
        let st = sample_shield_from_asset_lock_st();
        assert!(st.optional_asset_lock_proof().is_some());
    }

    #[test]
    fn test_shield_from_asset_lock_user_fee_increase_is_zero_and_setter_noop() {
        let mut st = sample_shield_from_asset_lock_st();
        assert_eq!(st.user_fee_increase(), 0);
        st.set_user_fee_increase(123);
        // Set is a no-op per mod.rs table.
        assert_eq!(st.user_fee_increase(), 0);
    }

    #[test]
    fn test_shield_from_asset_lock_set_signature_returns_true() {
        let mut st = sample_shield_from_asset_lock_st();
        assert!(st.set_signature(BinaryData::new(vec![0x44; 65])));
        assert_eq!(st.signature().unwrap().as_slice(), &[0x44; 65]);
    }

    #[test]
    fn test_shield_from_asset_lock_required_asset_lock_balance_succeeds() {
        // This is the only arm besides IdentityCreate/TopUp/AddressFundingFromAssetLock
        // that returns Ok from required_asset_lock_balance_for_processing_start.
        let st = sample_shield_from_asset_lock_st();
        let result = st.required_asset_lock_balance_for_processing_start(PlatformVersion::latest());
        assert!(
            result.is_ok(),
            "ShieldFromAssetLock should return Ok, got {:?}",
            result
        );
    }

    #[test]
    fn test_shield_from_asset_lock_active_range_12_latest() {
        let range = sample_shield_from_asset_lock_st().active_version_range();
        assert_eq!(*range.start(), 12);
        assert_eq!(*range.end(), LATEST_VERSION);
    }

    // ---------- Batch with Token transition exercises TokenTransfer arm
    //            in the name() nested match. ----------

    #[test]
    fn test_batch_with_token_transfer_name_contains_token_transfer() {
        use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransition as TT;
        use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
        use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use crate::state_transition::batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
        use crate::state_transition::batch_transition::token_transfer_transition::TokenTransferTransition;
        use crate::state_transition::batch_transition::BatchTransitionV1;

        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: Identifier::from([1u8; 32]),
            token_id: Identifier::from([2u8; 32]),
            using_group_info: None,
        });
        let token_transfer = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base,
            amount: 100,
            recipient_id: Identifier::from([3u8; 32]),
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        });

        // BatchTransitionV1 is used for tokens. Build a single-token batch.
        let batch = BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([9u8; 32]),
            transitions: vec![BatchedTransition::Token(TT::Transfer(token_transfer))],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![0u8; 65]),
        });
        let st = StateTransition::Batch(batch);

        assert_eq!(st.name(), "DocumentsBatch([TokenTransfer])");
    }

    // -----------------------------------------------------------------------
    // Cross-variant consistency: transaction_id is a 32-byte blake3/sha256 hash
    // of the serialized form. Make sure it's stable across clones for the newly
    // covered variants too.
    // -----------------------------------------------------------------------

    #[test]
    fn test_transaction_id_length_32_for_new_variants() {
        for st in [
            sample_withdrawal_v1_st(),
            sample_credit_transfer_to_addresses_st(),
            sample_shield_from_asset_lock_st(),
        ] {
            let id = st.transaction_id().expect("hash");
            assert_eq!(id.len(), 32);
        }
    }

    // -----------------------------------------------------------------------
    // Clone-and-equality coverage for newly added variants (PartialEq via
    // derived impl, exercises the top-level enum's PartialEq arms).
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_eq_for_new_variants() {
        let cases = [
            sample_withdrawal_v1_st(),
            sample_credit_transfer_to_addresses_st(),
            sample_address_credit_withdrawal_st(),
            sample_shield_from_asset_lock_st(),
        ];
        for st in cases {
            let cloned = st.clone();
            assert_eq!(st, cloned, "clone must be equal for {}", st.name());
        }
    }

    // -----------------------------------------------------------------------
    // unique_identifiers() for address-* variants: the implementation
    // dispatches via call_method! and each variant returns a non-empty Vec.
    // -----------------------------------------------------------------------

    #[test]
    fn test_unique_identifiers_for_address_and_shielded_variants() {
        // The address variants compute identifiers from their `inputs` map.
        // With a default (empty inputs) transition, unique_identifiers is empty
        // — that's fine, but we still want to exercise the call_method!
        // dispatch for these arms without panicking.
        for st in [
            sample_address_credit_withdrawal_st(),
            sample_shield_from_asset_lock_st(),
            sample_credit_transfer_to_addresses_st(),
            sample_withdrawal_v1_st(),
        ] {
            // Just calling unique_identifiers exercises the match arm; the
            // result may be empty for default-constructed inputs-based
            // variants, non-empty for identity-based ones.
            let _ids = st.unique_identifiers();
        }
        // For the identity-based variants the result IS non-empty.
        assert!(!sample_withdrawal_v1_st().unique_identifiers().is_empty());
        assert!(!sample_credit_transfer_to_addresses_st()
            .unique_identifiers()
            .is_empty());
    }

    // -----------------------------------------------------------------------
    // sign_with_core_signer byte-parity test
    //
    // Proves that `StateTransition::sign_with_core_signer` produces a
    // byte-identical signature to the legacy `sign_by_private_key` ECDSA path
    // when both are driven by the same underlying secret. This is the on-wire
    // contract the Swift / external-signer flow depends on: changing the
    // digest pre-image or the recoverable-compact encoding would silently
    // break asset-lock verification on testnet/mainnet, so we pin both shapes
    // here.
    // -----------------------------------------------------------------------
    #[cfg(all(
        feature = "state-transition-signing",
        feature = "core_key_wallet",
        feature = "bls-signatures"
    ))]
    #[tokio::test]
    async fn sign_with_core_signer_matches_sign_by_private_key_byte_for_byte() {
        use async_trait::async_trait;
        use dashcore::secp256k1::{
            ecdsa, rand::rngs::OsRng, Message, PublicKey, Secp256k1, SecretKey,
        };
        use key_wallet::bip32::{DerivationPath, ExtendedPubKey};
        use key_wallet::signer::{ExtendedPubKeySigner, Signer as KwSigner, SignerMethod};

        /// Fixed-key in-memory signer used only by this test. Mirrors how a
        /// real KeychainSigner would behave: derive once, sign atomically,
        /// return non-recoverable `(Signature, PublicKey)`. The path is
        /// ignored — the wrapper holds exactly one key.
        #[derive(Debug)]
        struct FixedKeySigner {
            secret: SecretKey,
            public: PublicKey,
        }

        #[async_trait]
        impl KwSigner for FixedKeySigner {
            type Error = String;

            fn supported_methods(&self) -> &[SignerMethod] {
                &[SignerMethod::Digest]
            }

            async fn sign_ecdsa(
                &self,
                _path: &DerivationPath,
                sighash: [u8; 32],
            ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
                let secp = Secp256k1::new();
                let msg = Message::from_digest(sighash);
                let sig = secp.sign_ecdsa(&msg, &self.secret);
                Ok((sig, self.public))
            }

            async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
                Ok(self.public)
            }
        }

        #[async_trait]
        impl ExtendedPubKeySigner for FixedKeySigner {
            async fn extended_public_key(
                &self,
                _path: &DerivationPath,
            ) -> Result<ExtendedPubKey, Self::Error> {
                Err("FixedKeySigner does not derive extended public keys".to_string())
            }
        }

        // Generate a single random key. Using the same key on both sides is
        // load-bearing: the legacy path signs raw bytes, the signer path
        // derives + signs inside the trust boundary. If the digest pre-image
        // or compact-encoding differs, the bytes will diverge.
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let private_key_bytes = secret_key.secret_bytes();

        let signer = FixedKeySigner {
            secret: secret_key,
            public: public_key,
        };
        let path = DerivationPath::default();

        // Use a sample state transition that exercises signable_bytes() —
        // any signable ST works since we're only comparing the signature
        // bytes the two paths produce over the SAME `signable_bytes()`.
        let mut st_legacy = sample_transfer_st();
        let mut st_signer = sample_transfer_st();

        // Sanity: both copies must have identical signable_bytes before signing.
        assert_eq!(
            st_legacy.signable_bytes().expect("legacy signable_bytes"),
            st_signer.signable_bytes().expect("signer signable_bytes"),
            "signable_bytes pre-image must match across copies"
        );

        // Legacy path: raw &[u8] private key → 65-byte recoverable compact.
        // BLS is only used by `sign_by_private_key` when key_type is BLS12_381 —
        // for the ECDSA path it's unused, but the function signature requires
        // it, so we pass the NativeBlsModule that's already in the workspace.
        let bls = crate::bls::native_bls::NativeBlsModule;
        st_legacy
            .sign_by_private_key(&private_key_bytes, KeyType::ECDSA_HASH160, &bls)
            .expect("sign_by_private_key");

        // New signer-driven path: digest → external signer → recovered →
        // 65-byte recoverable compact. Byte-identical to the legacy result.
        st_signer
            .sign_with_core_signer(&path, &signer)
            .await
            .expect("sign_with_core_signer");

        let sig_legacy = st_legacy.signature().expect("legacy signature set");
        let sig_signer = st_signer.signature().expect("signer signature set");

        assert_eq!(
            sig_legacy.as_slice().len(),
            65,
            "legacy ECDSA signature must be 65 bytes (recoverable compact)"
        );
        assert_eq!(
            sig_signer.as_slice().len(),
            65,
            "signer ECDSA signature must be 65 bytes (recoverable compact)"
        );
        assert_eq!(
            sig_legacy.as_slice(),
            sig_signer.as_slice(),
            "sign_with_core_signer must produce byte-identical output to sign_by_private_key"
        );
    }
}
