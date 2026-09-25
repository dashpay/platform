mod action_fees;
mod contract_owner_requirement;
mod creation;
mod deletable_document_reference;
mod deletion;
mod distinct_from;
mod dpns;
mod encrypted_for;
mod gas_sponsorship;
mod id_reuse;
mod immutable;
mod index_only;
mod keep_history;
mod list_element_reference;
mod lookup_reference;
mod max_bytes;
mod nft;
mod owner_balance_proof;
mod owner_reference;
mod property_constraints;
mod ranked_group_drain;
mod reference_expression;
mod reference_test_setup;
mod replacement;
mod required_since;
mod system_agreement;
mod transfer;
mod typed_array_references;

use super::*;

use crate::execution::validation::state_transition::tests::create_card_game_internal_token_contract_with_owner_identity_burn_tokens;

use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::version::PlatformVersion;

pub(super) const REFERENCE_VALIDATION_IDENTITY_KEY_REQUIREMENTS_CONTRACT_PATH: &str =
    "tests/supporting_files/contract/reference-validation/reference-validation-contract-identity-key-requirements.json";

/// The keys of the test identity the key-requirement tests can point at. The fixture's
/// `message.recipientId` requires a decryption key bound to the fixture contract's
/// `inbox` document type. Both fixture types declare
/// `requiresIdentityEncryptionBoundedKey` and `requiresIdentityDecryptionBoundedKey`,
/// without which Drive registers no encryption or decryption key bound to them.
pub(super) struct IdentityKeyRequirementTargets {
    pub(super) identity_id: Identifier,
    /// The critical authentication key the identity registered with: the wrong purpose
    pub(super) authentication_key_id: KeyID,
    /// A decryption key bound to (fixture contract, `inbox`): meets both requirements
    pub(super) decryption_key_bound_to_inbox_id: KeyID,
    /// An encryption key bound to (fixture contract, `inbox`): the wrong purpose
    pub(super) encryption_key_bound_to_inbox_id: KeyID,
    /// A decryption key bound to (fixture contract, `message`): the wrong document type
    pub(super) decryption_key_bound_to_message_id: KeyID,
    /// A decryption key without contract bounds
    pub(super) unbound_decryption_key_id: KeyID,
}

/// Adds the four keys of [`IdentityKeyRequirementTargets`] to `identity` in state, bound
/// to `contract_id` where bound, and returns the targets.
pub(super) fn add_identity_key_requirement_targets(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    identity: &Identity,
    authentication_key_id: KeyID,
    contract_id: Identifier,
    platform_version: &PlatformVersion,
) -> IdentityKeyRequirementTargets {
    let key = |id: KeyID, purpose: Purpose, contract_bounds: Option<ContractBounds>| {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: SecurityLevel::HIGH,
            contract_bounds,
            key_type: KeyType::ECDSA_HASH160,
            data: BinaryData::new(vec![0x70 + id as u8; 20]),
            read_only: false,
            disabled_at: None,
        })
    };
    let bound_to = |document_type_name: &str| {
        Some(ContractBounds::SingleContractDocumentType {
            id: contract_id,
            document_type_name: document_type_name.to_string(),
        })
    };

    let targets = IdentityKeyRequirementTargets {
        identity_id: identity.id(),
        authentication_key_id,
        decryption_key_bound_to_inbox_id: 2,
        encryption_key_bound_to_inbox_id: 3,
        decryption_key_bound_to_message_id: 4,
        unbound_decryption_key_id: 5,
    };

    platform
        .drive
        .add_new_non_unique_keys_to_identity(
            identity.id().to_buffer(),
            vec![
                key(
                    targets.decryption_key_bound_to_inbox_id,
                    Purpose::DECRYPTION,
                    bound_to("inbox"),
                ),
                key(
                    targets.encryption_key_bound_to_inbox_id,
                    Purpose::ENCRYPTION,
                    bound_to("inbox"),
                ),
                key(
                    targets.decryption_key_bound_to_message_id,
                    Purpose::DECRYPTION,
                    bound_to("message"),
                ),
                key(targets.unbound_decryption_key_id, Purpose::DECRYPTION, None),
            ],
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to add the keys to the identity");

    targets
}
