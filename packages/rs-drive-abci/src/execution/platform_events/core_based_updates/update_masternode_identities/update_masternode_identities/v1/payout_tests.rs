use crate::config::{PlatformConfig, PlatformTestConfig};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::signature::SignatureError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{ProTxHash, PubkeyHash, ScriptBuf, ScriptHash, Txid};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    DMNPayout, DMNState, DMNStateDiff, MasternodeListDiff, MasternodeListItem, MasternodeType,
};
use dpp::document::DocumentV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::core_script::CoreScript;
use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::identity_credit_withdrawal_transition::methods::{
    IdentityCreditWithdrawalTransitionMethodsV0, PreferredKeyPurposeForSigningWithdrawal,
};
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::grovedb::Transaction;
use drive::util::batch::{DriveOperation, IdentityOperationType};
use rand::prelude::StdRng;
use rand::SeedableRng;
use simple_signer::signer::SimpleSigner;
use std::collections::BTreeMap;
use std::sync::Arc;

const PRO_TX_HASH: [u8; 32] = [0x71; 32];
const UPDATE_TIME: u64 = 1_200_002_000;

fn payout(address: [u8; 20], reward: u16) -> DMNPayout {
    DMNPayout {
        address,
        script: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(address)),
        reward,
    }
}

fn empty_diff() -> DMNStateDiff {
    serde_json::from_str("{}").expect("empty Core state diff")
}

struct PayoutFixture {
    platform: TempPlatform<MockCoreRPCLike>,
    signer: SimpleSigner,
    original_identity: Identity,
    original_key: IdentityPublicKey,
}

impl PayoutFixture {
    fn new() -> Self {
        Self::with_split_payout(false)
    }

    fn with_split_payout(initial_split: bool) -> Self {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false);

        let mut rng = StdRng::seed_from_u64(713);
        let (original_key, original_secret) =
            IdentityPublicKey::random_masternode_transfer_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("payout key pair");
        let (owner_key, owner_secret) =
            IdentityPublicKey::random_masternode_owner_key_with_rng(1, &mut rng, platform_version)
                .expect("owner key pair");
        let mut signer = SimpleSigner::default();
        signer.add_identity_public_key(original_key.clone(), original_secret);
        signer.add_identity_public_key(owner_key.clone(), owner_secret);
        let masternode = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash: ProTxHash::from_byte_array(PRO_TX_HASH),
            collateral_hash: Txid::from_byte_array([0x72; 32]),
            collateral_index: 0,
            collateral_address: Some([0x73; 20]),
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:9999".parse().expect("service address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: Some(owner_key.public_key_hash().expect("owner hash")),
                voting_address: [0x74; 20],
                payout_address: None,
                payouts: Some(if initial_split {
                    vec![
                        payout(original_key.public_key_hash().expect("payout hash"), 5000),
                        payout([0x76; 20], 5000),
                    ]
                } else {
                    vec![payout(
                        original_key.public_key_hash().expect("payout hash"),
                        10000,
                    )]
                }),
                pub_key_operator: vec![0x75; 48],
                operator_payout_address: None,
                platform_node_id: None,
                #[allow(deprecated)]
                legacy_platform_p2p_port: None,
                #[allow(deprecated)]
                legacy_platform_http_port: None,
                addresses: None,
            },
        };
        let transaction = platform.drive.grove.start_transaction();
        platform
            .update_masternode_identities(
                MasternodeListDiff {
                    base_height: 0,
                    block_height: 1,
                    added_mns: vec![masternode.clone()],
                    removed_mns: vec![],
                    updated_mns: vec![],
                },
                &BTreeMap::new(),
                &BlockInfo::default_with_time(1_200_001_000),
                None,
                &transaction,
                platform_version,
            )
            .expect("register sole-payout masternode");
        let credits = dash_to_credits!(0.5);
        platform
            .drive
            .add_to_system_credits(credits, Some(&transaction), platform_version)
            .expect("system credits");
        platform
            .drive
            .add_to_identity_balance(
                PRO_TX_HASH,
                credits,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("fund payout identity");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit registration");
        let mut state = platform.state.load().as_ref().clone();
        state.insert_masternode(masternode);
        platform.state.store(Arc::new(state));
        let original_identity = platform
            .drive
            .fetch_full_identity(PRO_TX_HASH, None, platform_version)
            .expect("fetch registered identity")
            .expect("registered owner identity");
        Self {
            platform,
            signer,
            original_identity,
            original_key,
        }
    }

    fn identity(&self, transaction: &Transaction) -> Identity {
        self.platform
            .drive
            .fetch_full_identity(PRO_TX_HASH, Some(transaction), PlatformVersion::latest())
            .expect("fetch owner identity")
            .expect("owner identity")
    }

    fn update(&self, payouts: Vec<DMNPayout>, transaction: &Transaction) {
        self.apply_diff(
            DMNStateDiff {
                payouts: Some(payouts),
                ..empty_diff()
            },
            transaction,
        )
        .expect("apply payout list update");
    }

    fn apply_diff(&self, diff: DMNStateDiff, transaction: &Transaction) -> Result<(), Error> {
        self.platform.update_masternode_identities(
            MasternodeListDiff {
                base_height: 1,
                block_height: 2,
                added_mns: vec![],
                removed_mns: vec![],
                updated_mns: vec![(ProTxHash::from_byte_array(PRO_TX_HASH), diff)],
            },
            &BTreeMap::new(),
            &BlockInfo::default_with_time(UPDATE_TIME),
            Some(&self.platform.state.load()),
            transaction,
            PlatformVersion::latest(),
        )
    }

    fn add_later_credits(&self, transaction: &Transaction) {
        let credits = dash_to_credits!(0.2);
        let version = PlatformVersion::latest();
        self.platform
            .drive
            .add_to_system_credits(credits, Some(transaction), version)
            .expect("later system credits");
        self.platform
            .drive
            .add_to_identity_balance(
                PRO_TX_HASH,
                credits,
                &BlockInfo::default_with_time(UPDATE_TIME),
                true,
                Some(transaction),
                version,
            )
            .expect("later payout credits");
    }

    async fn withdraw(
        &self,
        identity: &Identity,
        key: &IdentityPublicKey,
        nonce: u64,
        explicit_destination: bool,
        transaction: &Transaction<'_>,
    ) -> Vec<StateTransitionExecutionResult> {
        let transition = IdentityCreditWithdrawalTransition::try_from_identity(
            identity,
            explicit_destination.then(|| CoreScript::random_p2pkh(&mut StdRng::seed_from_u64(714))),
            dash_to_credits!(0.1),
            Pooling::Never,
            1,
            0,
            self.signer.clone(),
            Some(key),
            PreferredKeyPurposeForSigningWithdrawal::Any,
            nonce,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("sign withdrawal with the supplied enabled-key snapshot");
        self.platform
            .process_raw_state_transitions(
                &vec![transition
                    .serialize_to_bytes()
                    .expect("serialize withdrawal")],
                &self.platform.state.load(),
                &BlockInfo::default_with_time(UPDATE_TIME + 1),
                transaction,
                PlatformVersion::latest(),
                false,
                None,
            )
            .expect("process signed withdrawal")
            .into_execution_results()
    }
}

#[tokio::test]
async fn should_create_first_transfer_key_and_withdraw_from_an_initially_owner_only_identity() {
    let mut fixture = PayoutFixture::with_split_payout(true);
    let transaction = fixture.platform.drive.grove.start_transaction();
    let initial = fixture.identity(&transaction);
    assert_eq!(
        initial.public_keys().keys().copied().collect::<Vec<_>>(),
        vec![1]
    );
    assert!(recent_keys(&fixture, &transaction).is_empty());
    fixture.update(
        vec![payout(
            fixture
                .original_key
                .public_key_hash()
                .expect("sole payout hash"),
            10000,
        )],
        &transaction,
    );
    let identity = fixture.identity(&transaction);
    assert_eq!(
        identity.public_keys().keys().copied().collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        identity.public_keys().get(&1),
        initial.public_keys().get(&1)
    );
    assert_eq!(identity.balance(), initial.balance());
    let transfer = identity
        .public_keys()
        .get(&2)
        .expect("first transfer key")
        .clone();
    assert_eq!(transfer.purpose(), Purpose::TRANSFER);
    assert!(!transfer.is_disabled());
    assert_eq!(
        recent_keys(&fixture, &transaction),
        BTreeMap::from([(2, transfer.clone())])
    );
    let secret = *fixture
        .signer
        .private_keys
        .get(&fixture.original_key)
        .expect("payout secret");
    fixture
        .signer
        .add_identity_public_key(transfer.clone(), secret);
    assert_matches!(
        fixture
            .withdraw(&identity, &transfer, 1, true, &transaction)
            .await
            .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
}

#[tokio::test]
async fn should_allow_replacement_payout_and_preserve_owner_default_withdrawals() {
    let mut fixture = PayoutFixture::new();
    let transaction = fixture.platform.drive.grove.start_transaction();
    let (key, secret) = IdentityPublicKey::random_masternode_transfer_key_with_rng(
        2,
        &mut StdRng::seed_from_u64(715),
        PlatformVersion::latest(),
    )
    .expect("replacement key pair");
    let address = key.public_key_hash().expect("replacement hash");
    fixture.update(vec![payout(address, 10000)], &transaction);
    fixture.add_later_credits(&transaction);
    let identity = fixture.identity(&transaction);
    let replacement = identity
        .public_keys()
        .get(&2)
        .expect("replacement transfer key");
    fixture
        .signer
        .add_identity_public_key(replacement.clone(), secret);
    assert_matches!(
        fixture
            .withdraw(&identity, replacement, 1, true, &transaction)
            .await
            .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    let owner = identity.public_keys().get(&1).expect("owner key");
    assert_matches!(
        fixture
            .withdraw(&identity, owner, 2, false, &transaction)
            .await
            .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    let documents = fixture
        .platform
        .drive
        .fetch_oldest_withdrawal_documents(Some(&transaction), PlatformVersion::latest())
        .expect("withdrawal documents");
    assert!(documents.values().flatten().any(|document| document
        .properties()
        .get_bytes("outputScript")
        .expect("output script")
        == CoreScript::new_p2pkh(address).to_bytes()));
    let before_repeat = fixture.identity(&transaction);
    fixture.update(vec![payout(address, 10000)], &transaction);
    assert_eq!(fixture.identity(&transaction), before_repeat);
}

#[tokio::test]
async fn should_restore_single_payout_authority_without_duplicate_keys() {
    let fixture = PayoutFixture::new();
    let transaction = fixture.platform.drive.grove.start_transaction();
    let original_address = fixture
        .original_key
        .public_key_hash()
        .expect("original hash");
    fixture.update(vec![payout([0x76; 20], 10000)], &transaction);
    let replacement_identity = fixture.identity(&transaction);
    fixture.update(vec![], &transaction);
    fixture.update(vec![payout(original_address, 10000)], &transaction);
    let restored = fixture.identity(&transaction);
    assert_eq!(restored.public_keys().len(), 3);
    assert_eq!(restored.public_keys().get(&0), Some(&fixture.original_key));
    assert!(restored
        .public_keys()
        .get(&2)
        .expect("replacement key")
        .is_disabled());
    assert_matches!(
        fixture
            .withdraw(&restored, &fixture.original_key, 1, true, &transaction)
            .await
            .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(
        restored.public_keys().get(&1),
        replacement_identity.public_keys().get(&1)
    );
}

#[tokio::test]
async fn should_revoke_p2pkh_authority_for_unsupported_or_inconsistent_payout_scripts() {
    let fixture = PayoutFixture::new();
    let transaction = fixture.platform.drive.grove.start_transaction();
    let address = fixture.original_key.public_key_hash().expect("payout hash");
    for script in [
        ScriptBuf::new_p2sh(&ScriptHash::from_byte_array(address)),
        ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x76; 20])),
    ] {
        fixture.update(
            vec![DMNPayout {
                address,
                script,
                reward: 10000,
            }],
            &transaction,
        );
        assert_disabled_key(
            &fixture
                .withdraw(
                    &fixture.original_identity,
                    &fixture.original_key,
                    1,
                    true,
                    &transaction,
                )
                .await,
            0,
        );
        fixture.update(vec![payout(address, 10000)], &transaction);
        assert_eq!(
            fixture.identity(&transaction).public_keys(),
            fixture.original_identity.public_keys()
        );
    }
}

#[tokio::test]
async fn should_reuse_old_payout_keys_after_more_than_six_rotations_with_an_explicit_destination() {
    let fixture = PayoutFixture::new();
    let transaction = fixture.platform.drive.grove.start_transaction();
    for address in 0x80..0x88 {
        fixture.update(vec![payout([address; 20], 10000)], &transaction);
    }
    fixture.update(
        vec![payout(
            fixture.original_key.public_key_hash().expect("old hash"),
            10000,
        )],
        &transaction,
    );
    let restored = fixture.identity(&transaction);
    assert_eq!(restored.public_keys().len(), 10);
    assert_eq!(
        restored
            .public_keys()
            .values()
            .filter(|key| key.purpose() == Purpose::TRANSFER && !key.is_disabled())
            .count(),
        1
    );
    assert_matches!(
        fixture
            .withdraw(&restored, &fixture.original_key, 1, true, &transaction)
            .await
            .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    // The inherited default-destination lookup is bounded; revocation does not widen it.
    assert_matches!(
        fixture
            .withdraw(
                &restored,
                restored.public_keys().get(&1).expect("owner key"),
                2,
                false,
                &transaction
            )
            .await
            .as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(
            ConsensusError::StateError(StateError::IdentityPublicKeyIsDisabledError(error))
        )] if error.public_key_index() == 9
    );
}

fn recent_keys(
    fixture: &PayoutFixture,
    transaction: &Transaction,
) -> KeyIDIdentityPublicKeyPairBTreeMap {
    fixture
        .platform
        .drive
        .fetch_identity_keys(
            IdentityKeysRequest {
                identity_id: PRO_TX_HASH,
                request_type: KeyRequestType::RecentWithdrawalKeys,
                limit: None,
                offset: None,
            },
            Some(transaction),
            PlatformVersion::latest(),
        )
        .expect("withdrawal index keys")
}

#[test]
fn should_rollback_payout_authority_and_retry_without_index_divergence() {
    let fixture = PayoutFixture::new();
    let candidate = fixture.platform.drive.grove.start_transaction();
    let initial_recent = recent_keys(&fixture, &candidate);
    fixture.update(vec![payout([0x76; 20], 10000)], &candidate);
    let accepted_identity = fixture.identity(&candidate);
    let accepted_recent = recent_keys(&fixture, &candidate);
    assert!(accepted_recent
        .get(&0)
        .expect("old indexed key")
        .is_disabled());
    assert!(!accepted_recent
        .get(&2)
        .expect("new indexed key")
        .is_disabled());
    assert_eq!(
        accepted_identity.balance(),
        fixture.original_identity.balance()
    );
    fixture
        .platform
        .drive
        .grove
        .rollback_transaction(&candidate)
        .expect("rollback candidate");
    assert_eq!(fixture.identity(&candidate), fixture.original_identity);
    assert_eq!(recent_keys(&fixture, &candidate), initial_recent);
    drop(candidate);
    let retry = fixture.platform.drive.grove.start_transaction();
    fixture.update(vec![payout([0x76; 20], 10000)], &retry);
    assert_eq!(fixture.identity(&retry), accepted_identity);
    assert_eq!(recent_keys(&fixture, &retry), accepted_recent);
}

#[test]
fn should_reject_mixed_payout_representations_without_changing_identity() {
    let fixture = PayoutFixture::new();
    let transaction = fixture.platform.drive.grove.start_transaction();
    assert_matches!(
        fixture.apply_diff(
            DMNStateDiff {
                payout_address: Some([0x76; 20]),
                payouts: Some(vec![payout([0x77; 20], 10000)]),
                ..empty_diff()
            },
            &transaction
        ),
        Err(Error::Execution(ExecutionError::DashCoreBadResponseError(
            _
        )))
    );
    assert_eq!(fixture.identity(&transaction), fixture.original_identity);
}

#[test]
fn should_not_reuse_owner_keys_as_transfer_keys_even_if_the_hash_matches() {
    let fixture = PayoutFixture::new();
    let transaction = fixture.platform.drive.grove.start_transaction();
    // Core rejects this owner/payout collision; purpose matching is defensive for malformed data.
    let owner_hash = fixture
        .original_identity
        .public_keys()
        .get(&1)
        .expect("owner key")
        .public_key_hash()
        .expect("owner hash");
    fixture.update(vec![payout(owner_hash, 10000)], &transaction);
    let identity = fixture.identity(&transaction);
    assert_eq!(
        identity.public_keys().get(&1),
        fixture.original_identity.public_keys().get(&1)
    );
    assert_eq!(
        identity
            .public_keys()
            .get(&2)
            .expect("new payout key")
            .purpose(),
        Purpose::TRANSFER
    );
}

#[test]
fn should_fail_new_key_allocation_at_exhaustion_but_allow_reuse_and_revocation() {
    let fixture = PayoutFixture::new();
    let transaction = fixture.platform.drive.grove.start_transaction();
    let (maximum_key, _) = IdentityPublicKey::random_masternode_transfer_key_with_rng(
        KeyID::MAX,
        &mut StdRng::seed_from_u64(716),
        PlatformVersion::latest(),
    )
    .expect("maximum-id key");
    fixture
        .platform
        .drive
        .apply_drive_operations(
            vec![DriveOperation::IdentityOperation(
                IdentityOperationType::AddNewKeysToIdentity {
                    identity_id: PRO_TX_HASH,
                    unique_keys_to_add: vec![],
                    non_unique_keys_to_add: vec![maximum_key],
                },
            )],
            true,
            &BlockInfo::default(),
            Some(&transaction),
            PlatformVersion::latest(),
            None,
        )
        .expect("add maximum key id");
    let before = fixture.identity(&transaction);
    let mut operations = vec![];
    assert_matches!(
        fixture.platform.reconcile_owner_payout_keys(
            PRO_TX_HASH,
            &[payout([0x76; 20], 10000)],
            &transaction,
            &mut operations,
            PlatformVersion::latest()
        ),
        Err(Error::Execution(ExecutionError::Overflow(_)))
    );
    assert!(operations.is_empty());
    assert_eq!(fixture.identity(&transaction), before);
    fixture.update(
        vec![payout(
            fixture
                .original_key
                .public_key_hash()
                .expect("original hash"),
            10000,
        )],
        &transaction,
    );
    assert!(!fixture
        .identity(&transaction)
        .public_keys()
        .get(&0)
        .expect("original key")
        .is_disabled());
    fixture.update(vec![], &transaction);
    assert!(fixture
        .identity(&transaction)
        .public_keys()
        .values()
        .filter(|key| key.purpose() == Purpose::TRANSFER)
        .all(|key| key.is_disabled()));
}

fn assert_disabled_key(results: &[StateTransitionExecutionResult], expected_id: u32) {
    assert_matches!(results, [StateTransitionExecutionResult::UnpaidConsensusError(
        ConsensusError::SignatureError(SignatureError::PublicKeyIsDisabledError(error))
    )] if error.public_key_id() == expected_id);
}

async fn rejects_obsolete_payout(split: bool) {
    let fixture = PayoutFixture::new();
    let transaction = fixture.platform.drive.grove.start_transaction();
    assert_matches!(
        fixture
            .withdraw(
                &fixture.original_identity,
                &fixture.original_key,
                1,
                true,
                &transaction
            )
            .await
            .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    let old_address = fixture
        .original_key
        .public_key_hash()
        .expect("old payout hash");
    let payouts = if split {
        vec![payout(old_address, 5000), payout([0x76; 20], 5000)]
    } else {
        vec![payout([0x76; 20], 10000)]
    };
    fixture.update(payouts, &transaction);
    fixture.add_later_credits(&transaction);
    let balance_before = fixture.identity(&transaction).balance();
    let documents_before = fixture
        .platform
        .drive
        .fetch_oldest_withdrawal_documents(Some(&transaction), PlatformVersion::latest())
        .expect("withdrawal documents before rejection");
    // An attacker retains the old enabled key and can still sign. Only node-side validation
    // of the updated stored identity can remove that key's authority over later rewards.
    let results = fixture
        .withdraw(
            &fixture.original_identity,
            &fixture.original_key,
            2,
            true,
            &transaction,
        )
        .await;
    assert_disabled_key(&results, 0);
    if split {
        assert!(fixture
            .identity(&transaction)
            .public_keys()
            .values()
            .filter(|key| key.purpose() == Purpose::TRANSFER)
            .all(|key| key.is_disabled()));
    }
    assert_eq!(fixture.identity(&transaction).balance(), balance_before);
    assert_eq!(
        fixture
            .platform
            .drive
            .fetch_oldest_withdrawal_documents(Some(&transaction), PlatformVersion::latest())
            .expect("withdrawal documents after rejection"),
        documents_before
    );
    assert_eq!(
        fixture.identity(&transaction).public_keys().get(&1),
        fixture.original_identity.public_keys().get(&1)
    );
}

#[tokio::test]
async fn should_reject_obsolete_payout_key_withdrawals_after_recipient_replacement() {
    rejects_obsolete_payout(false).await;
}

#[tokio::test]
async fn should_reject_withdrawals_by_a_recipient_remaining_in_a_split() {
    rejects_obsolete_payout(true).await;
}
