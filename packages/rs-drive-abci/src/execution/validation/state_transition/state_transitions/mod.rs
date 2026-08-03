/// Module containing functionality related to batch processing of documents.
pub mod batch;

/// Module for creating an identity entity.
pub mod identity_create;

/// Module for managing transfers of credit between identity entities.
pub mod identity_credit_transfer;

/// Module for managing withdrawals of credit from an identity entity.
pub mod identity_credit_withdrawal;

/// Module for topping up credit in an identity entity.
pub mod identity_top_up;

/// Module for updating an existing identity entity.
pub mod identity_update;

/// Module for creating a data contract entity.
pub mod data_contract_create;

/// Module for updating an existing data contract entity.
pub mod data_contract_update;

/// Module for voting from a masternode.
pub mod masternode_vote;

/// Identity create from addresses
pub mod identity_create_from_addresses;

/// Module for validation of address funding from asset lock transitions
pub mod address_funding_from_asset_lock;

/// Module for validation of credit transfer from an identity to addresses
pub mod identity_credit_transfer_to_addresses;

/// Module for validation of address credit withdrawal transitions
pub mod address_credit_withdrawal;

/// Module for validation of address funds transfer transitions
pub mod address_funds_transfer;
mod identity_top_up_from_addresses;

/// Module for identity-create-from-shielded-pool transition validation
pub mod identity_create_from_shielded_pool;
/// Module for shield transition validation
pub mod shield;
/// Module for shield from asset lock transition validation
pub mod shield_from_asset_lock;
/// Common validation logic shared by shielded transitions (proof verification)
pub mod shielded_common;
/// Module for shielded transfer transition validation
pub mod shielded_transfer;
/// Module for shielded withdrawal transition validation
pub mod shielded_withdrawal;
/// Module for unshield transition validation
pub mod unshield;

/// The validation mode we are using
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationMode {
    /// The basic checktx before the state transition is put into mempool
    CheckTx,
    /// Rechecking a state transition every block
    RecheckTx,
    /// The validation during block execution by a proposer or validator
    Validator,
    /// A validation mode used to get the action with no validation
    NoValidation,
}

impl ValidationMode {
    /// Can this validation mode alter cache on drive?
    pub fn can_alter_cache(&self) -> bool {
        match self {
            ValidationMode::CheckTx => false,
            ValidationMode::RecheckTx => false,
            ValidationMode::Validator => true,
            ValidationMode::NoValidation => false,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_helpers;

#[cfg(test)]
pub(in crate::execution) mod tests {
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contracts::SystemDataContract;
    use dpp::fee::Credits;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0, KeyID, KeyType, Purpose, SecurityLevel, TimestampMillis};
    use dpp::prelude::{BlockHeight, Identifier, IdentityNonce};
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use drive::query::DriveDocumentQuery;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};
    use simple_signer::signer::SimpleSigner;
    use std::borrow::Cow;
    use std::collections::BTreeMap;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::ops::Deref;
    use std::sync::Arc;
    use arc_swap::Guard;
    use assert_matches::assert_matches;
    use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem, MasternodeType};
    use dapi_grpc::platform::v0::{get_contested_resource_vote_state_request, get_contested_resource_vote_state_response, GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse};
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::{get_contested_resource_vote_state_request_v0, GetContestedResourceVoteStateRequestV0};
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0};
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::FinishedVoteInfo;
    use dpp::balances::credits::TokenAmount;
    use dpp::dash_to_credits;
    use dpp::dashcore::{ProTxHash, Txid};
    use dpp::dashcore::hashes::Hash;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::{DataContract, GroupContractPosition, TokenContractPosition};
    use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV1Setters};
    use dpp::data_contract::document_type::random_document::{CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType};
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::platform_value::{Bytes32, Value};
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
    use dpp::state_transition::StateTransition;
    use dpp::tokens::calculate_token_id;
    use dpp::util::hash::hash_double;
    use dpp::util::strings::convert_to_homograph_safe_chars;
    use dpp::voting::contender_structs::{Contender, ContenderV0};
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::ResourceVote;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use dpp::voting::votes::Vote;
    use drive::util::object_size_info::DataContractResolvedInfo;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
    use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally;
    use drive::query::vote_poll_vote_state_query::{ContestedDocumentVotePollDriveQueryResultType, ResolvedContestedDocumentVotePollDriveQuery};
    use drive::util::test_helpers::setup_contract;
    use crate::execution::types::block_execution_context::BlockExecutionContext;
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::expect_match;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::platform_types::state_transitions_processing_result::{StateTransitionExecutionResult, StateTransitionsProcessingResult};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::{SuccessfulExecution, UnpaidConsensusError};
    use crate::execution::types::block_state_info::BlockStateInfo;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::execution::types::block_fees::v0::BlockFeesV0;
    use crate::execution::types::processed_block_fees_outcome::v0::ProcessedBlockFeesOutcome;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::group::Group;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_amount_on_contract_token::{DocumentActionTokenCost, DocumentActionTokenEffect};
    use dpp::data_contract::document_type::accessors::DocumentTypeV0MutGetters;
    use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

    /// We add an identity, but we also add the same amount to system credits
    pub(in crate::execution) fn setup_identity_with_system_credits(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        credits: Credits,
    ) -> (Identity, SimpleSigner, IdentityPublicKey) {
        let platform_version = PlatformVersion::latest();
        platform
            .drive
            .add_to_system_credits(credits, None, platform_version)
            .expect("expected to add to system credits");
        setup_identity(platform, seed, credits)
    }

    pub(in crate::execution) fn setup_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        credits: Credits,
    ) -> (Identity, SimpleSigner, IdentityPublicKey) {
        let platform_version = PlatformVersion::latest();
        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(seed);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(critical_public_key.clone(), private_key);

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key.clone()),
            ]),
            balance: credits,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        (identity, signer, critical_public_key)
    }

    pub(in crate::execution) fn setup_identity_without_adding_it(
        seed: u64,
        credits: Credits,
    ) -> (Identity, SimpleSigner, IdentityPublicKey) {
        let platform_version = PlatformVersion::latest();
        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(seed);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(critical_public_key.clone(), private_key);

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key.clone()),
            ]),
            balance: credits,
            revision: 0,
        }
        .into();

        (identity, signer, critical_public_key)
    }

    pub(in crate::execution) fn setup_identity_return_master_key(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        credits: Credits,
    ) -> (Identity, SimpleSigner, IdentityPublicKey, IdentityPublicKey) {
        let platform_version = PlatformVersion::latest();
        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(seed);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(critical_public_key.clone(), private_key);

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key.clone()),
            ]),
            balance: credits,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        (identity, signer, critical_public_key, master_key)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn setup_add_key_to_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        identity: &mut Identity,
        signer: &mut SimpleSigner,
        seed: u64,
        key_id: KeyID,
        purpose: Purpose,
        security_level: SecurityLevel,
        key_type: KeyType,
        contract_bounds: Option<ContractBounds>,
    ) -> IdentityPublicKey {
        let platform_version = PlatformVersion::latest();

        let mut rng = StdRng::seed_from_u64(seed);

        let (key, private_key) = IdentityPublicKey::random_key_with_known_attributes(
            key_id,
            &mut rng,
            purpose,
            security_level,
            key_type,
            contract_bounds,
            platform_version,
        )
        .expect("expected to get key pair");

        signer.add_identity_public_key(key.clone(), private_key);

        identity.add_public_key(key.clone());

        platform
            .drive
            .add_new_unique_keys_to_identity(
                identity.id().to_buffer(),
                vec![key.clone()],
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new key");

        key
    }

    pub(in crate::execution) fn setup_identity_with_withdrawal_key_and_system_credits(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        withdrawal_key_type: KeyType,
        credits: Credits,
    ) -> (Identity, SimpleSigner, IdentityPublicKey, IdentityPublicKey) {
        let platform_version = PlatformVersion::latest();
        platform
            .drive
            .add_to_system_credits(credits, None, platform_version)
            .expect("expected to add to system credits");
        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(seed);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(master_key.clone(), master_private_key);

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(critical_public_key.clone(), private_key);

        let (withdrawal_public_key, withdrawal_private_key) =
            IdentityPublicKey::random_key_with_known_attributes(
                2,
                &mut rng,
                Purpose::TRANSFER,
                SecurityLevel::CRITICAL,
                withdrawal_key_type,
                None,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_identity_public_key(withdrawal_public_key.clone(), withdrawal_private_key);

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key.clone()),
                (2, withdrawal_public_key.clone()),
            ]),
            balance: credits,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        (identity, signer, critical_public_key, withdrawal_public_key)
    }

    pub(in crate::execution) fn add_tokens_to_identity(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        identity_id: Identifier,
        balance_to_add: Credits,
    ) {
        let platform_version = PlatformVersion::latest();
        platform
            .drive
            .add_to_identity_token_balance(
                token_id.to_buffer(),
                identity_id.to_buffer(),
                balance_to_add,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add token balance to identity");
        platform
            .drive
            .add_to_token_total_supply(
                token_id.to_buffer(),
                balance_to_add,
                true,
                false,
                true,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to add to total supply");
    }

    pub(in crate::execution) fn process_state_transitions(
        platform: &TempPlatform<MockCoreRPCLike>,
        state_transitions: &[StateTransition],
        block_info: BlockInfo,
        platform_state: &PlatformState,
    ) -> (Vec<FeeResult>, ProcessedBlockFeesOutcome) {
        let platform_version = PlatformVersion::latest();

        let raw_state_transitions = state_transitions
            .iter()
            .map(|a| a.serialize_to_bytes().expect("expected to serialize"))
            .collect::<Vec<_>>();

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &raw_state_transitions,
                platform_state,
                &block_info,
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        let fee_results = processing_result.execution_results().iter().map(|result| {
            let fee_result = expect_match!(result, StateTransitionExecutionResult::SuccessfulExecution{ fee_result, .. } => fee_result);
            fee_result.clone()
        }).collect();

        // while we have the state transitions executed, we now need to process the block fees
        let block_fees_v0: BlockFeesV0 = processing_result.aggregated_fees().clone().into();

        let block_execution_context = BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height: block_info.height,
                round: 0,
                block_time_ms: block_info.time_ms,
                previous_block_time_ms: platform_state.last_committed_block_time_ms(),
                proposer_pro_tx_hash: Default::default(),
                core_chain_locked_height: 0,
                block_hash: None,
                app_hash: None,
            }),
            epoch_info: EpochInfo::V0(EpochInfoV0::default()),
            unsigned_withdrawal_transactions: Default::default(),
            block_address_balance_changes: Default::default(),
            block_platform_state: platform_state.clone(),
            proposer_results: None,
        });

        // Process fees
        let processed_block_fees = platform
            .process_block_fees_and_validate_sum_trees(
                &block_execution_context,
                block_fees_v0.into(),
                &transaction,
                platform_version,
            )
            .expect("expected to process block fees");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        (fee_results, processed_block_fees)
    }

    pub(in crate::execution) fn fetch_expected_identity_balance(
        platform: &TempPlatform<MockCoreRPCLike>,
        identity_id: Identifier,
        platform_version: &PlatformVersion,
        expected_balance: Credits,
    ) {
        assert_eq!(
            expected_balance,
            platform
                .drive
                .fetch_identity_balance(identity_id.to_buffer(), None, platform_version)
                .expect("expected to be able to fetch balance")
                .expect("expected a balance")
        );
    }

    pub(in crate::execution) fn setup_masternode_owner_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        credits: Credits,
        platform_version: &PlatformVersion,
    ) -> (Identity, SimpleSigner, IdentityPublicKey, IdentityPublicKey) {
        let mut signer = SimpleSigner::default();

        platform
            .drive
            .add_to_system_credits(credits, None, platform_version)
            .expect("expected to add to system credits");

        let mut rng = StdRng::seed_from_u64(seed);

        let (transfer_key, transfer_private_key) =
            IdentityPublicKey::random_masternode_transfer_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        let (owner_key, owner_private_key) =
            IdentityPublicKey::random_masternode_owner_key_with_rng(1, &mut rng, platform_version)
                .expect("expected to get key pair");

        let owner_address = owner_key
            .public_key_hash()
            .expect("expected a public key hash");

        let payout_address = transfer_key
            .public_key_hash()
            .expect("expected a public key hash");

        signer.add_identity_public_key(transfer_key.clone(), transfer_private_key);
        signer.add_identity_public_key(owner_key.clone(), owner_private_key);

        let pro_tx_hash_bytes: [u8; 32] = rng.gen();

        let identity: Identity = IdentityV0 {
            id: pro_tx_hash_bytes.into(),
            public_keys: BTreeMap::from([(0, transfer_key.clone()), (1, owner_key.clone())]),
            balance: credits,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        let mut platform_state = platform.state.load().clone().deref().clone();

        let pro_tx_hash = ProTxHash::from_byte_array(pro_tx_hash_bytes);

        let random_ip = Ipv4Addr::new(
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
        );

        platform_state.full_masternode_list_mut().insert(
            pro_tx_hash,
            MasternodeListItem {
                node_type: MasternodeType::Regular,
                pro_tx_hash,
                collateral_hash: Txid::from_byte_array(rng.gen()),
                collateral_index: 0,
                collateral_address: rng.gen(),
                operator_reward: 0.0,
                state: DMNState {
                    service: SocketAddr::new(IpAddr::V4(random_ip), 19999),
                    registered_height: 0,
                    pose_revived_height: None,
                    pose_ban_height: None,
                    revocation_reason: 0,
                    owner_address,
                    voting_address: rng.gen(),
                    payout_address,
                    pub_key_operator: vec![],
                    operator_payout_address: None,
                    platform_node_id: None,
                    platform_p2p_port: None,
                    platform_http_port: None,
                },
            },
        );

        platform.state.store(Arc::new(platform_state));

        (identity, signer, owner_key, transfer_key)
    }

    pub(in crate::execution) fn setup_masternode_voting_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        platform_version: &PlatformVersion,
    ) -> (Identifier, Identity, SimpleSigner, IdentityPublicKey) {
        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(seed);

        let (voting_key, voting_private_key) =
            IdentityPublicKey::random_voting_key_with_rng(0, &mut rng, platform_version)
                .expect("expected to get key pair");

        signer.add_identity_public_key(voting_key.clone(), voting_private_key);

        let pro_tx_hash_bytes: [u8; 32] = rng.gen();

        let voting_address = voting_key
            .public_key_hash()
            .expect("expected a public key hash");

        let voter_identifier =
            Identifier::create_voter_identifier(&pro_tx_hash_bytes, &voting_address);

        let identity: Identity = IdentityV0 {
            id: voter_identifier,
            public_keys: BTreeMap::from([(0, voting_key.clone())]),
            balance: 0,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        let mut platform_state = platform.state.load().clone().deref().clone();

        let pro_tx_hash = ProTxHash::from_byte_array(pro_tx_hash_bytes);

        let random_ip = Ipv4Addr::new(
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
        );

        platform_state.full_masternode_list_mut().insert(
            pro_tx_hash,
            MasternodeListItem {
                node_type: MasternodeType::Regular,
                pro_tx_hash,
                collateral_hash: Txid::from_byte_array(rng.gen()),
                collateral_index: 0,
                collateral_address: rng.gen(),
                operator_reward: 0.0,
                state: DMNState {
                    service: SocketAddr::new(IpAddr::V4(random_ip), 19999),
                    registered_height: 0,
                    pose_revived_height: None,
                    pose_ban_height: None,
                    revocation_reason: 0,
                    owner_address: rng.gen(),
                    voting_address,
                    payout_address: rng.gen(),
                    pub_key_operator: vec![],
                    operator_payout_address: None,
                    platform_node_id: None,
                    platform_p2p_port: None,
                    platform_http_port: None,
                },
            },
        );

        platform.state.store(Arc::new(platform_state));

        (pro_tx_hash_bytes.into(), identity, signer, voting_key)
    }

    pub(in crate::execution) fn take_down_masternode_identities(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        masternode_identities: &Vec<Identifier>,
    ) {
        let mut platform_state = platform.state.load().clone().deref().clone();

        let list = platform_state.full_masternode_list_mut();

        for masternode_identifiers in masternode_identities {
            let pro_tx_hash = ProTxHash::from_byte_array(masternode_identifiers.to_buffer());

            list.remove(&pro_tx_hash);
        }

        platform.state.store(Arc::new(platform_state));
    }

    #[allow(dead_code)]
    pub(in crate::execution) enum IdentityTestInfo<'a> {
        Given {
            identity: &'a Identity,
            signer: &'a SimpleSigner,
            public_key: &'a IdentityPublicKey,
            identity_nonce: IdentityNonce,
        },
        UseSeed(u64),
        UseRng(&'a mut StdRng),
    }

    pub(in crate::execution) async fn register_contract_from_bytes(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract_bytes: Vec<u8>,
        identity_info: IdentityTestInfo<'_>,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        // Deserialize the data contract from bytes
        let mut data_contract =
            DataContract::versioned_deserialize(&contract_bytes, false, platform_version)
                .expect("expected to deserialize data contract");

        // Get identity info based on the enum variant
        let (identity_cow, signer_cow, key_cow, nonce) = match identity_info {
            IdentityTestInfo::Given {
                identity,
                signer,
                public_key,
                identity_nonce,
            } => (
                Cow::Borrowed(identity),
                Cow::Borrowed(signer),
                Cow::Borrowed(public_key),
                identity_nonce,
            ),
            IdentityTestInfo::UseSeed(seed) => {
                let (identity, signer, key) = setup_identity(platform, seed, dash_to_credits!(1));
                (Cow::Owned(identity), Cow::Owned(signer), Cow::Owned(key), 1)
            }
            IdentityTestInfo::UseRng(rng) => {
                let seed = rng.gen();
                let (identity, signer, key) = setup_identity(platform, seed, dash_to_credits!(1));
                (Cow::Owned(identity), Cow::Owned(signer), Cow::Owned(key), 1)
            }
        };

        let contract_id = DataContract::generate_data_contract_id_v0(identity_cow.id(), nonce);

        data_contract.set_id(contract_id);

        // Create and sign the data contract create transition
        let state_transition = DataContractCreateTransition::new_from_data_contract(
            data_contract.clone(),
            nonce,
            &identity_cow.as_ref().clone().into_partial_identity_info(),
            key_cow.id(),
            signer_cow.as_ref(),
            platform_version,
            None,
        )
        .await
        .expect("expected to create and sign data contract create transition");

        // Serialize the state transition
        let state_transition_bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[state_transition_bytes],
                platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let execution_result = processing_result.into_execution_results().remove(0);
        assert_matches!(execution_result, SuccessfulExecution { .. });

        data_contract
    }

    pub(in crate::execution) async fn create_dpns_name_contest_give_key_info(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        seed: u64,
        name: &str,
        platform_version: &PlatformVersion,
    ) -> (
        (
            Identity,
            SimpleSigner,
            IdentityPublicKey,
            (Document, Bytes32),
            (Document, Bytes32),
        ),
        (
            Identity,
            SimpleSigner,
            IdentityPublicKey,
            (Document, Bytes32),
            (Document, Bytes32),
        ),
        Arc<DataContract>,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);

        let identity_1_info = setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

        let identity_2_info = setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

        // Flip them if needed so identity 1 id is always smaller than identity 2 id
        let (identity_1_info, identity_2_info) = if identity_1_info.0.id() < identity_2_info.0.id()
        {
            (identity_1_info, identity_2_info)
        } else {
            (identity_2_info, identity_1_info)
        };

        let ((preorder_document_1, document_1), (preorder_document_2, document_2), dpns_contract) =
            create_dpns_name_contest_on_identities(
                platform,
                &identity_1_info,
                &identity_2_info,
                platform_state,
                rng,
                name,
                None,
                false,
                platform_version,
            )
            .await;

        let (identity_1, signer_1, identity_key_1) = identity_1_info;

        let (identity_2, signer_2, identity_key_2) = identity_2_info;

        (
            (
                identity_1,
                signer_1,
                identity_key_1,
                preorder_document_1,
                document_1,
            ),
            (
                identity_2,
                signer_2,
                identity_key_2,
                preorder_document_2,
                document_2,
            ),
            dpns_contract,
        )
    }

    pub(in crate::execution) async fn create_dpns_identity_name_contest(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        seed: u64,
        name: &str,
        platform_version: &PlatformVersion,
    ) -> (Identity, Identity, Arc<DataContract>) {
        let mut rng = StdRng::seed_from_u64(seed);

        let identity_1_info = setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

        let identity_2_info = setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

        // Flip them if needed so identity 1 id is always smaller than identity 2 id
        let (identity_1_info, identity_2_info) = if identity_1_info.0.id() < identity_2_info.0.id()
        {
            (identity_1_info, identity_2_info)
        } else {
            (identity_2_info, identity_1_info)
        };

        let (_, _, dpns_contract) = create_dpns_name_contest_on_identities(
            platform,
            &identity_1_info,
            &identity_2_info,
            platform_state,
            rng,
            name,
            None,
            false,
            platform_version,
        )
        .await;
        (identity_1_info.0, identity_2_info.0, dpns_contract)
    }

    /// This can be useful if we already created the identities and we reuse the seed
    pub(in crate::execution) async fn create_dpns_identity_name_contest_skip_creating_identities(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        seed: u64,
        name: &str,
        nonce_offset: Option<IdentityNonce>,
        platform_version: &PlatformVersion,
    ) -> (Identity, Identity, Arc<DataContract>) {
        let mut rng = StdRng::seed_from_u64(seed);

        let identity_1_info = setup_identity_without_adding_it(rng.gen(), dash_to_credits!(0.5));

        let identity_2_info = setup_identity_without_adding_it(rng.gen(), dash_to_credits!(0.5));

        // Flip them if needed so identity 1 id is always smaller than identity 2 id
        let (identity_1_info, identity_2_info) = if identity_1_info.0.id() < identity_2_info.0.id()
        {
            (identity_1_info, identity_2_info)
        } else {
            (identity_2_info, identity_1_info)
        };

        let (_, _, dpns_contract) = create_dpns_name_contest_on_identities(
            platform,
            &identity_1_info,
            &identity_2_info,
            platform_state,
            rng,
            name,
            nonce_offset,
            true, //we should also skip preorder
            platform_version,
        )
        .await;
        (identity_1_info.0, identity_2_info.0, dpns_contract)
    }

    pub(in crate::execution) async fn create_dpns_contract_name_contest(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        seed: u64,
        name: &str,
        platform_version: &PlatformVersion,
    ) -> (Identity, Identity, DataContract) {
        let mut rng = StdRng::seed_from_u64(seed);

        let identity_1_info = setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

        let identity_2_info = setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

        // Flip them if needed so identity 1 id is always smaller than identity 2 id
        let (identity_1_info, identity_2_info) = if identity_1_info.0.id() < identity_2_info.0.id()
        {
            (identity_1_info, identity_2_info)
        } else {
            (identity_2_info, identity_1_info)
        };

        let dashpay_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let card_game = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let (_, _, dpns_contract) = create_dpns_name_contest_on_identities_for_contract_records(
            platform,
            &identity_1_info,
            &identity_2_info,
            &dashpay_contract,
            &card_game,
            platform_state,
            rng,
            name,
            platform_version,
        )
        .await;
        (identity_1_info.0, identity_2_info.0, dpns_contract)
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_dpns_name_contest_on_identities(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        identity_1: &(Identity, SimpleSigner, IdentityPublicKey),
        identity_2: &(Identity, SimpleSigner, IdentityPublicKey),
        platform_state: &PlatformState,
        mut rng: StdRng,
        name: &str,
        nonce_offset: Option<IdentityNonce>,
        skip_preorder: bool,
        platform_version: &PlatformVersion,
    ) -> (
        ((Document, Bytes32), (Document, Bytes32)),
        ((Document, Bytes32), (Document, Bytes32)),
        Arc<DataContract>,
    ) {
        let (identity_1, signer_1, key_1) = identity_1;

        let (identity_2, signer_2, key_2) = identity_2;

        let dpns = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract");
        let dpns_contract = dpns.clone();

        let preorder = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected a profile document type");

        assert!(!preorder.documents_mutable());
        assert!(preorder.documents_can_be_deleted());
        assert!(!preorder.documents_transferable().is_transferable());

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        assert!(!domain.documents_mutable());
        // Deletion is disabled with data trigger
        assert!(domain.documents_can_be_deleted());
        assert!(domain.documents_transferable().is_transferable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document_1 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut preorder_document_2 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_2.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_1 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_2 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_2.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", name.into());
        document_1.set(
            "normalizedLabel",
            convert_to_homograph_safe_chars(name).into(),
        );
        document_1.set("records.identity", document_1.owner_id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        document_2.set("parentDomainName", "dash".into());
        document_2.set("normalizedParentDomainName", "dash".into());
        document_2.set("label", name.into());
        document_2.set(
            "normalizedLabel",
            convert_to_homograph_safe_chars(name).into(),
        );
        document_2.set("records.identity", document_2.owner_id().into());
        document_2.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();
        let salt_2: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        let mut salted_domain_buffer_2: Vec<u8> = vec![];
        salted_domain_buffer_2.extend(salt_2);
        salted_domain_buffer_2.extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());

        let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
        preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());

        document_1.set("preorderSalt", salt_1.into());
        document_2.set("preorderSalt", salt_2.into());

        let documents_batch_create_preorder_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_1.clone(),
                preorder,
                entropy.0,
                key_1,
                2 + nonce_offset.unwrap_or_default(),
                0,
                None,
                signer_1,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_1 =
            documents_batch_create_preorder_transition_1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_preorder_transition_2 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_2.clone(),
                preorder,
                entropy.0,
                key_2,
                2 + nonce_offset.unwrap_or_default(),
                0,
                None,
                signer_2,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_2 =
            documents_batch_create_preorder_transition_2
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                document_1.clone(),
                domain,
                entropy.0,
                key_1,
                3 + nonce_offset.unwrap_or_default(),
                0,
                None,
                signer_1,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_1 = documents_batch_create_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_2 =
            BatchTransition::new_document_creation_transition_from_document(
                document_2.clone(),
                domain,
                entropy.0,
                key_2,
                3 + nonce_offset.unwrap_or_default(),
                0,
                None,
                signer_2,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_2 = documents_batch_create_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        if !skip_preorder {
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[
                        documents_batch_create_serialized_preorder_transition_1.clone(),
                        documents_batch_create_serialized_preorder_transition_2.clone(),
                    ],
                    platform_state,
                    &BlockInfo::default_with_time(
                        platform_state
                            .last_committed_block_time_ms()
                            .unwrap_or_default()
                            + 3000,
                    ),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let successful_count = processing_result
                .execution_results()
                .iter()
                .filter(|result| {
                    assert_matches!(
                        result,
                        StateTransitionExecutionResult::SuccessfulExecution { .. }
                    );
                    true
                })
                .count();

            assert_eq!(successful_count, 2);
        }

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[
                    documents_batch_create_serialized_transition_1.clone(),
                    documents_batch_create_serialized_transition_2.clone(),
                ],
                platform_state,
                &BlockInfo::default_with_time(
                    platform_state
                        .last_committed_block_time_ms()
                        .unwrap_or_default()
                        + 3000,
                ),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let successful_count = processing_result
            .execution_results()
            .iter()
            .filter(|result| {
                assert_matches!(
                    result,
                    StateTransitionExecutionResult::SuccessfulExecution { .. }
                );
                true
            })
            .count();

        assert_eq!(successful_count, 2);
        (
            ((preorder_document_1, entropy), (document_1, entropy)),
            ((preorder_document_2, entropy), (document_2, entropy)),
            dpns_contract,
        )
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_dpns_name_contest_on_identities_for_contract_records(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        identity_1: &(Identity, SimpleSigner, IdentityPublicKey),
        identity_2: &(Identity, SimpleSigner, IdentityPublicKey),
        contract_1: &DataContract,
        contract_2: &DataContract,
        platform_state: &PlatformState,
        mut rng: StdRng,
        name: &str,
        platform_version: &PlatformVersion,
    ) -> (
        ((Document, Bytes32), (Document, Bytes32)),
        ((Document, Bytes32), (Document, Bytes32)),
        DataContract,
    ) {
        let (identity_1, signer_1, key_1) = identity_1;

        let (identity_2, signer_2, key_2) = identity_2;

        let dpns_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-contract-id.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let preorder = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected a profile document type");

        assert!(!preorder.documents_mutable());
        assert!(preorder.documents_can_be_deleted());
        assert!(!preorder.documents_transferable().is_transferable());

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        assert!(!domain.documents_mutable());
        // Deletion is disabled with data trigger
        assert!(domain.documents_can_be_deleted());
        assert!(domain.documents_transferable().is_transferable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document_1 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut preorder_document_2 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_2.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_1 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_2 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_2.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", name.into());
        document_1.set(
            "normalizedLabel",
            convert_to_homograph_safe_chars(name).into(),
        );
        document_1.remove("records.identity");
        document_1.set("records.contract", contract_1.id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        document_2.set("parentDomainName", "dash".into());
        document_2.set("normalizedParentDomainName", "dash".into());
        document_2.set("label", name.into());
        document_2.set(
            "normalizedLabel",
            convert_to_homograph_safe_chars(name).into(),
        );
        document_2.remove("records.identity");
        document_2.set("records.contract", contract_2.id().into());
        document_2.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();
        let salt_2: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        let mut salted_domain_buffer_2: Vec<u8> = vec![];
        salted_domain_buffer_2.extend(salt_2);
        salted_domain_buffer_2.extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());

        let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
        preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());

        document_1.set("preorderSalt", salt_1.into());
        document_2.set("preorderSalt", salt_2.into());

        let documents_batch_create_preorder_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_1.clone(),
                preorder,
                entropy.0,
                key_1,
                2,
                0,
                None,
                signer_1,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_1 =
            documents_batch_create_preorder_transition_1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_preorder_transition_2 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_2.clone(),
                preorder,
                entropy.0,
                key_2,
                2,
                0,
                None,
                signer_2,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_2 =
            documents_batch_create_preorder_transition_2
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                document_1.clone(),
                domain,
                entropy.0,
                key_1,
                3,
                0,
                None,
                signer_1,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_1 = documents_batch_create_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_2 =
            BatchTransition::new_document_creation_transition_from_document(
                document_2.clone(),
                domain,
                entropy.0,
                key_2,
                3,
                0,
                None,
                signer_2,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_2 = documents_batch_create_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[
                    documents_batch_create_serialized_preorder_transition_1.clone(),
                    documents_batch_create_serialized_preorder_transition_2.clone(),
                ],
                platform_state,
                &BlockInfo::default_with_time(
                    platform_state
                        .last_committed_block_time_ms()
                        .unwrap_or_default()
                        + 3000,
                ),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 2);

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[
                    documents_batch_create_serialized_transition_1.clone(),
                    documents_batch_create_serialized_transition_2.clone(),
                ],
                platform_state,
                &BlockInfo::default_with_time(
                    platform_state
                        .last_committed_block_time_ms()
                        .unwrap_or_default()
                        + 3000,
                ),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 2);
        (
            ((preorder_document_1, entropy), (document_1, entropy)),
            ((preorder_document_2, entropy), (document_2, entropy)),
            dpns_contract,
        )
    }

    pub(in crate::execution) async fn add_contender_to_dpns_name_contest(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        seed: u64,
        name: &str,
        expect_err: Option<&str>,
        platform_version: &PlatformVersion,
    ) -> Identity {
        let mut rng = StdRng::seed_from_u64(seed);

        let (identity_1, signer_1, key_1) =
            setup_identity(platform, rng.gen(), dash_to_credits!(0.5));

        let dpns = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract");
        let dpns_contract = dpns.clone();

        let preorder = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected a profile document type");

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document_1 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_1 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", name.into());
        document_1.set(
            "normalizedLabel",
            convert_to_homograph_safe_chars(name).into(),
        );
        document_1.set("records.identity", document_1.owner_id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());

        document_1.set("preorderSalt", salt_1.into());

        let documents_batch_create_preorder_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_1,
                preorder,
                entropy.0,
                &key_1,
                2,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_1 =
            documents_batch_create_preorder_transition_1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                document_1,
                domain,
                entropy.0,
                &key_1,
                3,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_1 = documents_batch_create_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_create_serialized_preorder_transition_1.clone()],
                platform_state,
                &BlockInfo::default_with_time(
                    platform_state
                        .last_committed_block_time_ms()
                        .unwrap_or_default()
                        + 3000,
                ),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 1);

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_create_serialized_transition_1.clone()],
                platform_state,
                &BlockInfo::default_with_time(
                    platform_state
                        .last_committed_block_time_ms()
                        .unwrap_or_default()
                        + 3000,
                ),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        if let Some(expected_err) = expect_err {
            let result = processing_result.into_execution_results().remove(0);

            let StateTransitionExecutionResult::PaidConsensusError {
                error: consensus_error,
                ..
            } = result
            else {
                panic!("expected a paid consensus error");
            };
            assert_eq!(consensus_error.to_string(), expected_err);
        } else {
            assert_eq!(processing_result.valid_count(), 1);
        }
        identity_1
    }

    pub(in crate::execution) fn verify_dpns_name_contest(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &Guard<Arc<PlatformState>>,
        dpns_contract: &DataContract,
        identity_1: &Identity,
        identity_2: &Identity,
        name: &str,
        platform_version: &PlatformVersion,
    ) {
        // Now let's run a query for the vote totals

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
            .expect("expected to encode the word dash");

        let quantum_encoded =
            bincode::encode_to_vec(Value::Text(convert_to_homograph_safe_chars(name)), config)
                .expect("expected to encode the word quantum");

        let index_name = "parentNameAndLabel".to_string();

        let query_validation_result = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: domain.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: true,
                            start_at_identifier_info: None,
                            count: None,
                            prove: false,
                        },
                    )),
                },
                platform_state,
                platform_version,
            )
            .expect("expected to execute query")
            .into_data()
            .expect("expected query to be valid");

        let get_contested_resource_vote_state_response::Version::V0(
            GetContestedResourceVoteStateResponseV0 {
                metadata: _,
                result,
            },
        ) = query_validation_result.version.expect("expected a version");

        let Some(
            get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                    contenders,
                    ..
                },
            ),
        ) = result
        else {
            panic!("expected contenders")
        };

        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        let first_contender_document = Document::from_bytes(
            first_contender
                .document
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        let second_contender_document = Document::from_bytes(
            second_contender
                .document
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        assert_ne!(first_contender_document, second_contender_document);

        assert_eq!(first_contender.identifier, identity_1.id().to_vec());

        assert_eq!(second_contender.identifier, identity_2.id().to_vec());

        assert_eq!(first_contender.vote_count, Some(0));

        assert_eq!(second_contender.vote_count, Some(0));

        let GetContestedResourceVoteStateResponse { version } = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: domain.name().clone(),
                            index_name: "parentNameAndLabel".to_string(),
                            index_values: vec![dash_encoded, quantum_encoded],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: true,
                            start_at_identifier_info: None,
                            count: None,
                            prove: true,
                        },
                    )),
                },
                platform_state,
                platform_version,
            )
            .expect("expected to execute query")
            .into_data()
            .expect("expected query to be valid");

        let get_contested_resource_vote_state_response::Version::V0(
            GetContestedResourceVoteStateResponseV0 {
                metadata: _,
                result,
            },
        ) = version.expect("expected a version");

        let Some(get_contested_resource_vote_state_response_v0::Result::Proof(proof)) = result
        else {
            panic!("expected contenders")
        };

        let resolved_contested_document_vote_poll_drive_query =
            ResolvedContestedDocumentVotePollDriveQuery {
                vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                    contract: DataContractResolvedInfo::BorrowedDataContract(dpns_contract),
                    document_type_name: domain.name().clone(),
                    index_name: index_name.clone(),
                    index_values: vec![
                        Value::Text("dash".to_string()),
                        Value::Text(convert_to_homograph_safe_chars(name)),
                    ],
                },
                result_type: DocumentsAndVoteTally,
                offset: None,
                limit: None,
                start_at: None,
                allow_include_locked_and_abstaining_vote_tally: true,
            };

        let (_, result) = resolved_contested_document_vote_poll_drive_query
            .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
            .expect("expected to verify proof");

        let contenders = result.contenders;

        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        let first_contender_document = Document::from_bytes(
            first_contender
                .serialized_document()
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        let second_contender_document = Document::from_bytes(
            second_contender
                .serialized_document()
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        assert_ne!(first_contender_document, second_contender_document);

        assert_eq!(first_contender.identity_id(), identity_1.id());

        assert_eq!(second_contender.identity_id(), identity_2.id());

        assert_eq!(first_contender.vote_tally(), Some(0));

        assert_eq!(second_contender.vote_tally(), Some(0));
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::execution) async fn perform_vote(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &Guard<Arc<PlatformState>>,
        dpns_contract: &DataContract,
        resource_vote_choice: ResourceVoteChoice,
        name: &str,
        signer: &SimpleSigner,
        pro_tx_hash: Identifier,
        voting_key: &IdentityPublicKey,
        nonce: IdentityNonce,
        expect_error: Option<&str>,
        platform_version: &PlatformVersion,
    ) {
        // Let's vote for contender 1

        let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    contract_id: dpns_contract.id(),
                    document_type_name: "domain".to_string(),
                    index_name: "parentNameAndLabel".to_string(),
                    index_values: vec![
                        Value::Text("dash".to_string()),
                        Value::Text(convert_to_homograph_safe_chars(name)),
                    ],
                },
            ),
            resource_vote_choice,
        }));

        let masternode_vote_transition = MasternodeVoteTransition::try_from_vote_with_signer(
            vote,
            signer,
            pro_tx_hash,
            voting_key,
            nonce,
            platform_version,
            None,
        )
        .await
        .expect("expected to make transition vote");

        let masternode_vote_serialized_transition = masternode_vote_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        // CheckTx root-invariance guard (devnet paloma h788): `check_tx` asserts under
        // cfg(test) that it never mutates committed grovedb state, so every valid vote
        // fixture going through this shared helper pins the invariant for masternode votes.
        if expect_error.is_none() {
            crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
                platform,
                &masternode_vote_serialized_transition,
                "masternode vote",
            );
        }

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[masternode_vote_serialized_transition.clone()],
                platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let execution_result = processing_result.into_execution_results().remove(0);
        if let Some(error_msg) = expect_error {
            assert_matches!(execution_result, UnpaidConsensusError(..));
            let UnpaidConsensusError(consensus_error) = execution_result else {
                panic!()
            };
            assert_eq!(consensus_error.to_string(), error_msg)
        } else {
            assert_matches!(execution_result, SuccessfulExecution { .. });
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::execution) async fn perform_votes(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        dpns_contract: &DataContract,
        resource_vote_choice: ResourceVoteChoice,
        name: &str,
        count: u64,
        start_seed: u64,
        nonce_offset: Option<IdentityNonce>,
        platform_version: &PlatformVersion,
    ) -> Vec<(Identifier, Identity, SimpleSigner, IdentityPublicKey)> {
        let mut masternode_infos = vec![];
        for i in 0..count {
            let (pro_tx_hash_bytes, voting_identity, signer, voting_key) =
                setup_masternode_voting_identity(platform, start_seed + i, platform_version);

            let platform_state = platform.state.load();

            perform_vote(
                platform,
                &platform_state,
                dpns_contract,
                resource_vote_choice,
                name,
                &signer,
                pro_tx_hash_bytes,
                &voting_key,
                1 + nonce_offset.unwrap_or_default(),
                None,
                platform_version,
            )
            .await;

            masternode_infos.push((pro_tx_hash_bytes, voting_identity, signer, voting_key));
        }
        masternode_infos
    }

    pub(in crate::execution) async fn perform_votes_multi(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        dpns_contract: &DataContract,
        resource_vote_choices: Vec<(ResourceVoteChoice, u64)>,
        name: &str,
        start_seed: u64,
        nonce_offset: Option<IdentityNonce>,
        platform_version: &PlatformVersion,
    ) -> BTreeMap<ResourceVoteChoice, Vec<(Identifier, Identity, SimpleSigner, IdentityPublicKey)>>
    {
        let mut count_aggregate = start_seed;
        let mut masternodes_by_vote_choice = BTreeMap::new();
        for (resource_vote_choice, count) in resource_vote_choices.into_iter() {
            let masternode_infos = perform_votes(
                platform,
                dpns_contract,
                resource_vote_choice,
                name,
                count,
                count_aggregate,
                nonce_offset,
                platform_version,
            )
            .await;
            masternodes_by_vote_choice.insert(resource_vote_choice, masternode_infos);
            count_aggregate += count;
        }
        masternodes_by_vote_choice
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::execution) fn get_vote_states(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        dpns_contract: &DataContract,
        name: &str,
        count: Option<u32>,
        allow_include_locked_and_abstaining_vote_tally: bool,
        start_at_identifier_info: Option<
            get_contested_resource_vote_state_request_v0::StartAtIdentifierInfo,
        >,
        result_type: ResultType,
        platform_version: &PlatformVersion,
    ) -> (
        Vec<Contender>,
        Option<u32>,
        Option<u32>,
        Option<FinishedVoteInfo>,
    ) {
        // Now let's run a query for the vote totals

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
            .expect("expected to encode the word dash");

        let name_encoded =
            bincode::encode_to_vec(Value::Text(convert_to_homograph_safe_chars(name)), config)
                .expect("expected to encode the word quantum");

        let index_name = "parentNameAndLabel".to_string();

        let query_validation_result = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: domain.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), name_encoded.clone()],
                            result_type: result_type as i32,
                            allow_include_locked_and_abstaining_vote_tally,
                            start_at_identifier_info,
                            count,
                            prove: false,
                        },
                    )),
                },
                platform_state,
                platform_version,
            )
            .expect("expected to execute query")
            .into_data()
            .expect("expected query to be valid");

        let get_contested_resource_vote_state_response::Version::V0(
            GetContestedResourceVoteStateResponseV0 {
                metadata: _,
                result,
            },
        ) = query_validation_result.version.expect("expected a version");

        let Some(
            get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                    contenders,
                    abstain_vote_tally,
                    lock_vote_tally,
                    finished_vote_info,
                },
            ),
        ) = result
        else {
            panic!("expected contenders")
        };
        (
            contenders
                .into_iter()
                .map(|contender| {
                    ContenderV0 {
                        identity_id: contender.identifier.try_into().expect("expected 32 bytes"),
                        document: contender.document.map(|document_bytes| {
                            Document::from_bytes(
                                document_bytes.as_slice(),
                                domain,
                                platform_version,
                            )
                            .expect("expected to deserialize document")
                        }),
                        vote_tally: contender.vote_count,
                    }
                    .into()
                })
                .collect(),
            abstain_vote_tally,
            lock_vote_tally,
            finished_vote_info,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::execution) fn get_proved_vote_states(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        dpns_contract: &DataContract,
        name: &str,
        count: Option<u32>,
        allow_include_locked_and_abstaining_vote_tally: bool,
        start_at_identifier_info: Option<
            get_contested_resource_vote_state_request_v0::StartAtIdentifierInfo,
        >,
        result_type: ResultType,
        platform_version: &PlatformVersion,
    ) -> (
        Vec<Contender>,
        Option<u32>,
        Option<u32>,
        Option<(ContestedDocumentVotePollWinnerInfo, BlockInfo)>,
    ) {
        // Now let's run a query for the vote totals

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
            .expect("expected to encode the word dash");

        let name_encoded =
            bincode::encode_to_vec(Value::Text(convert_to_homograph_safe_chars(name)), config)
                .expect("expected to encode the word quantum");

        let index_name = "parentNameAndLabel".to_string();

        let query_validation_result = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: domain.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), name_encoded.clone()],
                            result_type: result_type as i32,
                            allow_include_locked_and_abstaining_vote_tally,
                            start_at_identifier_info,
                            count,
                            prove: true,
                        },
                    )),
                },
                platform_state,
                platform_version,
            )
            .expect("expected to execute query")
            .into_data()
            .expect("expected query to be valid");

        let get_contested_resource_vote_state_response::Version::V0(
            GetContestedResourceVoteStateResponseV0 {
                metadata: _,
                result,
            },
        ) = query_validation_result.version.expect("expected a version");

        let Some(get_contested_resource_vote_state_response_v0::Result::Proof(proof)) = result
        else {
            panic!("expected contenders")
        };

        let resolved_contested_document_vote_poll_drive_query =
            ResolvedContestedDocumentVotePollDriveQuery {
                vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                    contract: DataContractResolvedInfo::BorrowedDataContract(dpns_contract),
                    document_type_name: domain.name().clone(),
                    index_name: index_name.clone(),
                    index_values: vec![
                        Value::Text("dash".to_string()),
                        Value::Text(convert_to_homograph_safe_chars(name)),
                    ],
                },
                result_type: ContestedDocumentVotePollDriveQueryResultType::try_from(
                    result_type as i32,
                )
                .expect("expected valid result type"),
                offset: None,
                limit: count.map(|a| a as u16),
                start_at: None,
                allow_include_locked_and_abstaining_vote_tally,
            };

        let (_, result) = resolved_contested_document_vote_poll_drive_query
            .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
            .expect("expected to verify proof");

        let abstaining_vote_tally = result.abstaining_vote_tally;
        let lock_vote_tally = result.locked_vote_tally;

        let contenders = result.contenders;
        let finished_vote_info = result.winner;
        (
            contenders
                .into_iter()
                .map(|contender| {
                    ContenderV0 {
                        identity_id: contender.identity_id(),
                        document: contender
                            .serialized_document()
                            .as_ref()
                            .map(|document_bytes| {
                                Document::from_bytes(
                                    document_bytes.as_slice(),
                                    domain,
                                    platform_version,
                                )
                                .expect("expected to deserialize document")
                            }),
                        vote_tally: contender.vote_tally(),
                    }
                    .into()
                })
                .collect(),
            abstaining_vote_tally,
            lock_vote_tally,
            finished_vote_info,
        )
    }

    pub(in crate::execution) fn create_token_contract_with_owner_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        identity_id: Identifier,
        token_configuration_modification: Option<impl FnOnce(&mut TokenConfiguration)>,
        contract_start_time: Option<TimestampMillis>,
        add_groups: Option<BTreeMap<GroupContractPosition, Group>>,
        contract_start_block: Option<BlockHeight>,
        platform_version: &PlatformVersion,
    ) -> (DataContract, Identifier) {
        let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

        let basic_token_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/basic-token/basic-token.json",
            Some(data_contract_id.to_buffer()),
            Some(identity_id.to_buffer()),
            Some(|data_contract: &mut DataContract| {
                data_contract.set_created_at_epoch(Some(0));
                data_contract.set_created_at(Some(contract_start_time.unwrap_or_default()));
                data_contract
                    .set_created_at_block_height(Some(contract_start_block.unwrap_or_default()));
                if let Some(token_configuration_modification) = token_configuration_modification {
                    let token_configuration = data_contract
                        .token_configuration_mut(0)
                        .expect("expected token configuration");
                    token_configuration_modification(token_configuration);
                }
                if let Some(add_groups) = add_groups {
                    data_contract.set_groups(add_groups);
                }
            }),
            None,
            Some(platform_version),
        );

        let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

        (basic_token_contract, token_id.into())
    }

    pub(in crate::execution) fn create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        identity_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> (DataContract, Identifier, Identifier) {
        let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

        let basic_token_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json",
            Some(data_contract_id.to_buffer()),
            Some(identity_id.to_buffer()),
            Some(|data_contract: &mut DataContract| {
                data_contract.set_created_at_epoch(Some(0));
                data_contract.set_created_at(Some(0));
                data_contract.set_created_at_block_height(Some(0));
            }),
            None,
            Some(platform_version),
        );

        let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);
        let token_id_2 = calculate_token_id(data_contract_id.as_bytes(), 1);

        (basic_token_contract, token_id.into(), token_id_2.into())
    }

    pub(in crate::execution) fn create_card_game_internal_token_contract_with_owner_identity_transfer_tokens(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        identity_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> (DataContract, Identifier, Identifier) {
        let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

        let basic_token_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json",
            Some(data_contract_id.to_buffer()),
            Some(identity_id.to_buffer()),
            Some(|data_contract: &mut DataContract| {
                data_contract.set_created_at_epoch(Some(0));
                data_contract.set_created_at(Some(0));
                data_contract.set_created_at_block_height(Some(0));
            }),
            None,
            Some(platform_version),
        );

        let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);
        let token_id_2 = calculate_token_id(data_contract_id.as_bytes(), 1);

        (basic_token_contract, token_id.into(), token_id_2.into())
    }

    pub(in crate::execution) fn create_card_game_external_token_contract_with_owner_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        token_contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_cost_amount: TokenAmount,
        gas_fees_paid_by: GasFeesPaidBy,
        identity_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

        let basic_token_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-use-external-currency.json",
            Some(data_contract_id.to_buffer()),
            Some(identity_id.to_buffer()),
            Some(|data_contract: &mut DataContract| {
                data_contract.set_created_at_epoch(Some(0));
                data_contract.set_created_at(Some(0));
                data_contract.set_created_at_block_height(Some(0));
                let document_type = data_contract.document_types_mut().get_mut("card").expect("expected a document type with name card");
                document_type.set_document_creation_token_cost(Some(DocumentActionTokenCost {
                    contract_id: Some(token_contract_id),
                    token_contract_position,
                    token_amount: token_cost_amount,
                    effect: DocumentActionTokenEffect::TransferTokenToContractOwner,
                    gas_fees_paid_by,
                }));
                let gas_fees_paid_by_int: u8 = gas_fees_paid_by.into();
                let schema = document_type.schema_mut();
                let token_cost = schema.get_mut("tokenCost").expect("expected to get token cost").expect("expected token cost to be set");
                let creation_token_cost = token_cost.get_mut("create").expect("expected to get creation token cost").expect("expected creation token cost to be set");
                creation_token_cost.set_value("contractId", token_contract_id.into()).expect("expected to set token contract id");
                creation_token_cost.set_value("tokenPosition", token_contract_position.into()).expect("expected to set token position");
                creation_token_cost.set_value("amount", token_cost_amount.into()).expect("expected to set token amount");
                creation_token_cost.set_value("gasFeesPaidBy", gas_fees_paid_by_int.into()).expect("expected to set token amount");
            }),
            None,
            Some(platform_version),
        );

        basic_token_contract
    }

    pub(in crate::execution) fn process_test_state_transition<S: PlatformSerializable>(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        state_transition: S,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let Ok(serialized_state_transition) = state_transition.serialize_to_bytes() else {
            panic!("expected documents batch serialized state transition")
        };

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized_state_transition],
                platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        processing_result
    }

    mod keyword_search_contract {
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::ConsensusError;
        use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::PaidConsensusError;
        use super::*;
        //
        // ──────────────────────────────────────────────────────────────────────────
        //  Keyword‑Search contract – creation is forbidden
        // ──────────────────────────────────────────────────────────────────────────
        //

        /// Return `(document, entropy)` so we can reuse the exact entropy when
        /// we build the transition.  **No rng.clone()** (that caused the ID mismatch).
        fn build_random_doc_of_type(
            rng: &mut StdRng,
            doc_type_name: &str,
            identity_id: Identifier,
            contract: &DataContract,
            platform_version: &PlatformVersion,
        ) -> (Document, Bytes32) {
            let doc_type = contract
                .document_type_for_name(doc_type_name)
                .expect("doc type exists");

            let entropy = Bytes32::random_with_rng(rng);

            let doc = doc_type
                .random_document_with_identifier_and_entropy(
                    rng,
                    identity_id,
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("random doc");

            (doc, entropy)
        }

        #[tokio::test]
        async fn should_err_when_creating_contract_keywords_document() {
            let platform_version = PlatformVersion::latest();

            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (identity, signer, key) = setup_identity(&mut platform, 42, dash_to_credits!(1.0));

            let contract = load_system_data_contract(
                SystemDataContract::KeywordSearch,
                PlatformVersion::latest(),
            )
            .expect("expected to load search contract");

            let mut rng = StdRng::seed_from_u64(1);
            let (doc, entropy) = build_random_doc_of_type(
                &mut rng,
                "contractKeywords",
                identity.id(),
                &contract,
                platform_version,
            );

            let transition = BatchTransition::new_document_creation_transition_from_document(
                doc,
                contract.document_type_for_name("contractKeywords").unwrap(),
                entropy.0, // same entropy → no ID mismatch
                &key,
                1,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("batch transition");

            let serialized = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &platform.drive.grove.start_transaction(),
                    platform_version,
                    false,
                    None,
                )
                .expect("processing failed");

            let execution_result = processing_result.into_execution_results().remove(0);
            assert_matches!(
                execution_result,
                StateTransitionExecutionResult::PaidConsensusError{ error, .. }
                    if error.to_string().contains("not allowed because of the document type's creation restriction mode")
            );
        }

        #[tokio::test]
        async fn should_err_when_creating_short_description_document() {
            let platform_version = PlatformVersion::latest();

            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (identity, signer, key) = setup_identity(&mut platform, 43, dash_to_credits!(1.0));

            let contract = load_system_data_contract(
                SystemDataContract::KeywordSearch,
                PlatformVersion::latest(),
            )
            .expect("expected to load search contract");

            let mut rng = StdRng::seed_from_u64(2);
            let (doc, entropy) = build_random_doc_of_type(
                &mut rng,
                "shortDescription",
                identity.id(),
                &contract,
                platform_version,
            );

            let transition = BatchTransition::new_document_creation_transition_from_document(
                doc,
                contract.document_type_for_name("shortDescription").unwrap(),
                entropy.0,
                &key,
                1,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("batch transition");

            let serialized = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &platform.drive.grove.start_transaction(),
                    platform_version,
                    false,
                    None,
                )
                .expect("processing failed");

            let execution_result = processing_result.into_execution_results().remove(0);
            assert_matches!(
                execution_result,
                StateTransitionExecutionResult::PaidConsensusError{ error, .. }
                    if error.to_string().contains("not allowed because of the document type's creation restriction mode")
            );
        }

        #[tokio::test]
        async fn should_err_when_creating_full_description_document() {
            let platform_version = PlatformVersion::latest();

            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (identity, signer, key) = setup_identity(&mut platform, 44, dash_to_credits!(1.0));

            let contract = load_system_data_contract(
                SystemDataContract::KeywordSearch,
                PlatformVersion::latest(),
            )
            .expect("expected to load search contract");

            let mut rng = StdRng::seed_from_u64(3);
            let (doc, entropy) = build_random_doc_of_type(
                &mut rng,
                "fullDescription",
                identity.id(),
                &contract,
                platform_version,
            );

            let transition = BatchTransition::new_document_creation_transition_from_document(
                doc,
                contract.document_type_for_name("fullDescription").unwrap(),
                entropy.0,
                &key,
                1,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("batch transition");

            let serialized = transition.serialize_to_bytes().unwrap();

            let platform_state = platform.state.load();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &platform.drive.grove.start_transaction(),
                    platform_version,
                    false,
                    None,
                )
                .expect("processing failed");

            let execution_result = processing_result.into_execution_results().remove(0);
            assert_matches!(
                execution_result,
                StateTransitionExecutionResult::PaidConsensusError{ error, .. }
                    if error.to_string().contains("not allowed because of the document type's creation restriction mode")
            );
        }

        //
        // ──────────────────────────────────────────────────────────────────────────
        //  Keyword‑Search contract – owner can update / delete
        // ──────────────────────────────────────────────────────────────────────────
        //

        async fn create_contract_with_keywords_and_description(
            platform: &mut TempPlatform<MockCoreRPCLike>,
        ) -> (Identity, SimpleSigner, IdentityPublicKey) {
            let platform_version = PlatformVersion::latest();

            // Owner identity
            let (owner_identity, signer, key) =
                setup_identity(platform, 777, dash_to_credits!(1.0));

            // Load the keyword‑test fixture
            let mut contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                Some(owner_identity.id()),
                None,
                false,
                platform_version,
            )
            .expect("load contract");

            // Inject description + keywords
            contract.set_description(Some("A short description".to_string()));
            contract.set_keywords(vec!["graph".into(), "indexing".into()]);

            // Create transition inside GroveDB tx
            let create_transition = DataContractCreateTransition::new_from_data_contract(
                contract,
                1,
                &owner_identity.clone().into_partial_identity_info(),
                key.id(),
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("build transition");

            let serialized = create_transition.serialize_to_bytes().unwrap();
            let platform_state = platform.state.load();
            let tx = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("process");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .expect("commit");

            (owner_identity, signer, key)
        }

        #[tokio::test]
        async fn owner_can_update_short_description_document() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (_owner, signer, key) =
                create_contract_with_keywords_and_description(&mut platform).await;

            // 🔎 fetch shortDescription doc through query
            let search_contract =
                load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                    .expect("load search contract");

            let doc_type = search_contract
                .document_type_for_name("shortDescription")
                .unwrap();

            let query = DriveDocumentQuery::all_items_query(&search_contract, doc_type, None);
            let existing_docs = platform
                .drive
                .query_documents(
                    query,
                    None,
                    false,
                    None,
                    Some(platform_version.protocol_version),
                )
                .expect("query failed");

            let mut doc = existing_docs.documents().first().expect("doc").clone();
            doc.set_revision(Some(doc.revision().unwrap_or_default() + 1));
            doc.set("description", "updated description".into());

            let transition = BatchTransition::new_document_replacement_transition_from_document(
                doc,
                doc_type,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("replace");

            let serialized = transition.serialize_to_bytes().unwrap();
            let platform_state = platform.state.load();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &platform.drive.grove.start_transaction(),
                    platform_version,
                    false,
                    None,
                )
                .expect("process");

            assert_matches!(
                processing_result.into_execution_results().remove(0),
                SuccessfulExecution { .. }
            );
        }

        #[tokio::test]
        async fn owner_can_not_delete_keyword_document() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (_owner, signer, key) =
                create_contract_with_keywords_and_description(&mut platform).await;

            let search_contract =
                load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                    .expect("load search contract");
            let doc_type = search_contract
                .document_type_for_name("contractKeywords")
                .unwrap();

            let query = DriveDocumentQuery::all_items_query(&search_contract, doc_type, None);
            let existing_docs = platform
                .drive
                .query_documents(
                    query,
                    None,
                    false,
                    None,
                    Some(platform_version.protocol_version),
                )
                .expect("query failed");

            let doc = existing_docs.documents().first().unwrap().clone();

            let transition = BatchTransition::new_document_deletion_transition_from_document(
                doc,
                doc_type,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("delete");

            let serialized = transition.serialize_to_bytes().unwrap();
            let platform_state = platform.state.load();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &platform.drive.grove.start_transaction(),
                    platform_version,
                    false,
                    None,
                )
                .expect("process");
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [PaidConsensusError {
                    error: ConsensusError::BasicError(
                        BasicError::InvalidDocumentTransitionActionError { .. }
                    ),
                    ..
                }]
            );
        }
    }

    /// End-to-end regression test for the byteArray on-disk encoding-flip
    /// chain-halt vulnerability (Dash Platform v4.0.0-rc.1).
    ///
    /// This drives the full per-block DAO vote-poll resolver path so that
    /// `Platform::check_for_ended_vote_polls` returns `Err` today (pre-fix). On
    /// chain that `Err` propagates through `run_dao_platform_events` ->
    /// `run_block_proposal` (a per-block event handler with a bare `?`), so it is
    /// NOT caught into a per-state-transition result -- it halts the chain on
    /// every validator at the block where the contested vote poll ends.
    mod byte_array_encoding_flip_chain_halt {
        use super::*;
        use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
        use dpp::data_contract::document_type::schema::validate_schema_compatibility;

        const CUSTOM_CONTESTED_CONTRACT: &str =
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json";
        // Identical to the contract above except `domain.preorderSalt` byteArray
        // has its `maxItems` widened from 32 to 64 -- the malicious update.
        const CUSTOM_CONTESTED_CONTRACT_WIDENED: &str =
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-byte-array-widened.json";

        /// Builds one preorder + one domain (contender) document for `identity`,
        /// with `preorderSalt` forced to `[0xFF; 32]` so that, after the
        /// byteArray encoding flips to varint-length-prefixed, the first stored
        /// byte (0xFF, a varint continuation byte) decodes to an enormous length
        /// and overruns the buffer.
        #[allow(clippy::too_many_arguments)]
        async fn build_preorder_and_domain(
            contract: &DataContract,
            identity: &Identity,
            signer: &SimpleSigner,
            key: &IdentityPublicKey,
            name: &str,
            // A per-contender distinguishing byte placed in the *tail* of the
            // salt so the two contenders' `saltedDomainHash` differ (avoiding a
            // unique-index collision on the preorder), while byte[0] stays 0xFF.
            salt_discriminator: u8,
            rng: &mut StdRng,
            platform_version: &PlatformVersion,
        ) -> (Vec<u8>, Vec<u8>) {
            let preorder = contract
                .document_type_for_name("preorder")
                .expect("expected preorder document type");
            let domain = contract
                .document_type_for_name("domain")
                .expect("expected domain document type");

            let entropy = Bytes32::random_with_rng(rng);

            let mut preorder_document = preorder
                .random_document_with_identifier_and_entropy(
                    rng,
                    identity.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random preorder document");

            let mut domain_document = domain
                .random_document_with_identifier_and_entropy(
                    rng,
                    identity.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random domain document");

            domain_document.set("parentDomainName", "dash".into());
            domain_document.set("normalizedParentDomainName", "dash".into());
            domain_document.set("label", name.into());
            domain_document.set(
                "normalizedLabel",
                convert_to_homograph_safe_chars(name).into(),
            );
            domain_document.set("records.identity", domain_document.owner_id().into());
            domain_document.set("subdomainRules.allowSubdomains", false.into());

            // The crux of the attack: a 32-byte salt whose FIRST byte is 0xFF
            // (a varint continuation byte). The last byte distinguishes the two
            // contenders so their preorder salted hashes do not collide.
            let mut salt: [u8; 32] = [0xFF; 32];
            salt[31] = salt_discriminator;

            let mut salted_domain_buffer: Vec<u8> = vec![];
            salted_domain_buffer.extend(salt);
            salted_domain_buffer
                .extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());
            let salted_domain_hash = hash_double(salted_domain_buffer);

            preorder_document.set("saltedDomainHash", salted_domain_hash.into());
            domain_document.set("preorderSalt", salt.into());

            let preorder_transition =
                BatchTransition::new_document_creation_transition_from_document(
                    preorder_document,
                    preorder,
                    entropy.0,
                    key,
                    2,
                    0,
                    None,
                    signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expect to create preorder batch transition");

            let domain_transition =
                BatchTransition::new_document_creation_transition_from_document(
                    domain_document,
                    domain,
                    entropy.0,
                    key,
                    3,
                    0,
                    None,
                    signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expect to create domain batch transition");

            (
                preorder_transition
                    .serialize_to_bytes()
                    .expect("serialize preorder transition"),
                domain_transition
                    .serialize_to_bytes()
                    .expect("serialize domain transition"),
            )
        }

        /// Runs the full contest setup and returns the result of the per-block
        /// vote-poll resolver. When `widen` is true the byteArray `maxItems` is
        /// flipped (the attack); when false the original contract is left in
        /// place (the control).
        async fn run_contest_then_resolve(widen: bool) -> Result<(), crate::error::Error> {
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(0xB17E_A77A);

            // Two contenders, each prefunded so they can pay the contested-index
            // voting balance.
            let identity_1_info = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let identity_2_info = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let contract_owner = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            // (1) Create the CUSTOM contested contract owned by `contract_owner`.
            // It has a contested unique index on `normalizedLabel` and a fixed
            // byteArray `preorderSalt` {minItems:32, maxItems:32}.
            let contract = setup_contract(
                &platform.drive,
                CUSTOM_CONTESTED_CONTRACT,
                None,
                Some(contract_owner.0.id().to_buffer()),
                None::<fn(&mut DataContract)>,
                None,
                Some(platform_version),
            );

            let name = "quantum";

            let platform_state = platform.state.load();

            // (2) Create TWO contested documents (contenders) with the same
            // contested-index values and a 0xFF-prefixed 32-byte byteArray.
            let (preorder_tx_1, domain_tx_1) = build_preorder_and_domain(
                &contract,
                &identity_1_info.0,
                &identity_1_info.1,
                &identity_1_info.2,
                name,
                0x01,
                &mut rng,
                platform_version,
            )
            .await;

            let (preorder_tx_2, domain_tx_2) = build_preorder_and_domain(
                &contract,
                &identity_2_info.0,
                &identity_2_info.1,
                &identity_2_info.2,
                name,
                0x02,
                &mut rng,
                platform_version,
            )
            .await;

            // Submit the preorders.
            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[preorder_tx_1, preorder_tx_2],
                    &platform_state,
                    &BlockInfo::default_with_time(
                        platform_state
                            .last_committed_block_time_ms()
                            .unwrap_or_default()
                            + 3000,
                    ),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process preorder state transitions");
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
            assert_eq!(
                processing_result.valid_count(),
                2,
                "both preorders should be accepted"
            );

            // Submit the domains -> this opens the contested vote poll.
            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[domain_tx_1, domain_tx_2],
                    &platform_state,
                    &BlockInfo::default_with_time(
                        platform_state
                            .last_committed_block_time_ms()
                            .unwrap_or_default()
                            + 3000,
                    ),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process domain state transitions");
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
            assert_eq!(
                processing_result.valid_count(),
                2,
                "both contenders should be accepted, opening the contest"
            );

            // (3) Simulate the malicious DataContractUpdate that widens the
            // byteArray `maxItems` from 32 to 64 (only in the attack scenario).
            if widen {
                // The JSON-schema compatibility layer still treats widening
                // `maxItems` as a compatible change -- which is exactly why the
                // dedicated byte-array-encoding check added to `validate_update`
                // (the fix) is required to reject it. We pin that compatibility
                // verdict here to document the gap the fix closes.
                let original_domain_schema = serde_json::json!({
                    "type": "object",
                    "properties": {
                        "preorderSalt": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "position": 4
                        }
                    },
                    "additionalProperties": false
                });
                let widened_domain_schema = serde_json::json!({
                    "type": "object",
                    "properties": {
                        "preorderSalt": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 64,
                            "position": 4
                        }
                    },
                    "additionalProperties": false
                });
                let compatibility = validate_schema_compatibility(
                    &original_domain_schema,
                    &widened_domain_schema,
                    platform_version,
                )
                .expect("schema compatibility validation must not error");
                assert!(
                    compatibility.is_valid(),
                    "the maxItems 32 -> 64 widening must be accepted by update \
                     validation for this attack to be reachable on chain; \
                     reported incompatibilities: {:?}",
                    compatibility.errors
                );

                // Apply the widened contract directly to grovedb to simulate the
                // corrupt on-disk state that WOULD result if such an update were
                // committed. On a real chain `validate_update` now rejects this
                // update at the source (see the rs-dpp validate_byte_array_encoding
                // tests), so the resolver is never reached with corrupt bytes;
                // this reproduces the consequence the fix prevents.
                // The derived `properties` for `preorderSalt` flip to the
                // variable-length (varint-prefixed) decode path.
                let widened_contract = setup_contract(
                    &platform.drive,
                    CUSTOM_CONTESTED_CONTRACT_WIDENED,
                    None,
                    Some(contract_owner.0.id().to_buffer()),
                    None::<fn(&mut DataContract)>,
                    None,
                    Some(platform_version),
                );
                assert_eq!(
                    widened_contract.id(),
                    contract.id(),
                    "the widened contract must replace the original at the same id"
                );
            }

            // (4) Advance past the vote-poll end date and invoke the resolver.
            let time_after_distribution_limit = platform_version
                .dpp
                .voting_versions
                .default_vote_poll_time_duration_test_network_ms
                + 10_000;

            fast_forward_to_block(&platform, time_after_distribution_limit, 900, 42, 0, false);

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // This is the exact per-block call. In the attack scenario it returns
            // Err because the resolver loads the OLD contender bytes and decodes
            // them against the WIDENED `preorderSalt` type, misreading the 0xFF
            // first byte as a varint length -> CorruptedSerialization.
            platform.check_for_ended_vote_polls(
                &platform_state,
                &platform_state,
                &BlockInfo {
                    time_ms: time_after_distribution_limit,
                    height: 900,
                    core_height: 42,
                    epoch: Default::default(),
                },
                Some(&transaction),
                platform_version,
            )
        }

        /// THE CHAIN-HALT CONSEQUENCE. If a byteArray `maxItems` widening were
        /// ever committed, the stored contender documents become undecodable and
        /// the per-block vote-poll resolver returns `Err`, which propagates out of
        /// the bare-`?` per-block event handler and halts every validator.
        ///
        /// The fix prevents that state at the source: `validate_update` now
        /// rejects the widening (see the rs-dpp validate_byte_array_encoding
        /// tests). This test deliberately writes the corrupt state directly to
        /// grovedb (bypassing validation) to reproduce the consequence the
        /// validation fix prevents.
        #[tokio::test]
        async fn widening_byte_array_max_items_halts_vote_poll_resolver() {
            let result = run_contest_then_resolve(true).await;

            assert!(
                result.is_err(),
                "check_for_ended_vote_polls must return Err when stored contender \
                 documents are decoded against a widened byteArray encoding -- the \
                 chain-halt consequence the `validate_update` fix prevents by \
                 rejecting the update at the source. Got Ok instead."
            );

            // Confirm it is the expected decode-failure variant from the encoding
            // flip, not some unrelated error. Matching the variant (rather than the
            // Debug string) keeps the test robust to message/format changes.
            assert_matches!(
                result.unwrap_err(),
                crate::error::Error::Protocol(dpp::ProtocolError::DataContractError(
                    dpp::data_contract::errors::DataContractError::CorruptedSerialization(_)
                        | dpp::data_contract::errors::DataContractError::DecodingContractError(_)
                ))
            );
        }

        /// CONTROL (causation proof). The exact same contest, but WITHOUT the
        /// byteArray widening, resolves successfully (`Ok`). Together with the
        /// test above this proves the widening is what causes the halt -- the
        /// only difference between the two runs is the `maxItems` flip.
        #[tokio::test]
        async fn contest_without_widening_resolves_successfully() {
            let result = run_contest_then_resolve(false).await;

            assert!(
                result.is_ok(),
                "control: without the byteArray widening the vote-poll resolver \
                 must succeed; got {:?}",
                result
            );
        }
    }
}
