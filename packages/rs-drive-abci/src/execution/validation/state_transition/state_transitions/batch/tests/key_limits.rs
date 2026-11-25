use super::*;
use crate::execution::validation::state_transition::tests::setup_identity_without_adding_it;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::consensus::codes::ErrorWithCode;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use dpp::state_transition::StateTransition;
use simple_signer::signer::SimpleSigner;

const PUBLIC_KEY_BUDGET_EXHAUSTED: u32 = 20015;
const PUBLIC_KEY_EXPIRED: u32 = 20016;
const IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED: u32 = 40218;

const BLOCK_TIME_MS: TimestampMillis = 1_000_000;

/// An identity whose signing key is stored with the given limits.
///
/// The transitions are signed with the key as the signer knows it, without limits: limits are
/// not part of what a key signs with, and validators must enforce the ones of the key in Drive.
struct LimitedKeySetup {
    platform: TempPlatform<MockCoreRPCLike>,
    identity: Identity,
    signer: SimpleSigner,
    signing_key: IdentityPublicKey,
}

impl LimitedKeySetup {
    fn new(budget: Option<Credits>, expires_at: Option<TimestampMillis>) -> Self {
        let version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (mut identity, signer, signing_key) =
            setup_identity_without_adding_it(958, dash_to_credits!(1));
        identity.add_public_key(signing_key.clone().with_limits(budget, expires_at));
        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                version,
            )
            .expect("expected to add the identity");
        Self {
            platform,
            identity,
            signer,
            signing_key,
        }
    }

    fn key_id(&self) -> KeyID {
        self.signing_key.id()
    }

    /// A dashpay profile creation signed by the limited key, with the first identity contract
    /// nonce
    async fn profile_creation(&self, seed: u64, user_fee_increase: u16) -> StateTransition {
        self.profile_creation_with_nonce(seed, user_fee_increase, 2)
            .await
    }

    /// The dashpay profile document of the identity for a seed, and the entropy its id comes from
    fn profile_document(&self, seed: u64) -> (dpp::document::Document, Bytes32) {
        let version = PlatformVersion::latest();
        let dashpay = self
            .platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(version)
            .expect("expected dashpay");
        let profile = dashpay
            .document_type_for_name("profile")
            .expect("expected the profile type");
        let mut rng = StdRng::seed_from_u64(seed);
        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                self.identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                version,
            )
            .expect("expected a document");
        set_valid_profile_payment_addresses(&mut document, profile);
        document.set("avatarUrl", "http://test.com/bob.jpg".into());
        // the id commits to the nonce of the creation, which `profile_creation` sends with
        // the first identity contract nonce
        document
            .set_id_for_creation(profile, &entropy.0, 2, version)
            .expect("expected to set the document id");
        (document, entropy)
    }

    async fn profile_creation_with_nonce(
        &self,
        seed: u64,
        user_fee_increase: u16,
        identity_contract_nonce: u64,
    ) -> StateTransition {
        let version = PlatformVersion::latest();
        let dashpay = self
            .platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(version)
            .expect("expected dashpay");
        let profile = dashpay
            .document_type_for_name("profile")
            .expect("expected the profile type");
        let (document, entropy) = self.profile_document(seed);
        BatchTransition::new_document_creation_transition_from_document(
            document,
            profile,
            entropy.0,
            &self.signing_key,
            identity_contract_nonce,
            user_fee_increase,
            None,
            &self.signer,
            version,
            None,
        )
        .await
        .expect("expected a batch transition")
    }

    /// The deletion of the profile created for the same seed, signed by the limited key
    async fn profile_deletion(&self, seed: u64, identity_contract_nonce: u64) -> StateTransition {
        let version = PlatformVersion::latest();
        let dashpay = self
            .platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(version)
            .expect("expected dashpay");
        let profile = dashpay
            .document_type_for_name("profile")
            .expect("expected the profile type");
        let (document, _) = self.profile_document(seed);
        BatchTransition::new_document_deletion_transition_from_document(
            document,
            profile,
            &self.signing_key,
            identity_contract_nonce,
            0,
            None,
            &self.signer,
            version,
            None,
        )
        .await
        .expect("expected a batch transition")
    }

    fn balance(&self, tx: &drive::grovedb::Transaction) -> Credits {
        self.platform
            .drive
            .fetch_identity_balance(
                self.identity.id().to_buffer(),
                Some(tx),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the balance")
            .expect("expected a balance")
    }

    fn remaining_budget(&self, tx: &drive::grovedb::Transaction) -> Option<Credits> {
        self.platform
            .drive
            .fetch_identity_key_remaining_budget(
                self.identity.id().to_buffer(),
                self.key_id(),
                Some(tx),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the remaining budget")
    }

    fn process(
        &self,
        transition: &StateTransition,
        time_ms: TimestampMillis,
        tx: &drive::grovedb::Transaction,
    ) -> StateTransitionExecutionResult {
        let version = PlatformVersion::latest();
        let state = self.platform.state.load();
        let result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &vec![transition
                    .serialize_to_bytes()
                    .expect("expected to serialize")],
                &state,
                &BlockInfo {
                    time_ms,
                    ..Default::default()
                },
                tx,
                version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        result.execution_results()[0].clone()
    }
}

fn assert_unpaid_with_code(execution: &StateTransitionExecutionResult, code: u32) {
    assert!(
        matches!(execution, StateTransitionExecutionResult::UnpaidConsensusError(error) if error.code() == code),
        "expected an unpaid error {code}, got {execution:?}"
    );
}

#[tokio::test]
async fn should_deduct_what_a_transition_took_from_the_identity_from_the_key_budget() {
    let budget = dash_to_credits!(0.5);
    let setup = LimitedKeySetup::new(Some(budget), None);
    let tx = setup.platform.drive.grove.start_transaction();
    assert_eq!(setup.remaining_budget(&tx), Some(budget));

    let execution = setup.process(&setup.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx);
    assert_matches!(
        execution,
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    let paid_by_identity = setup.identity.balance() - setup.balance(&tx);
    assert!(paid_by_identity > 0);
    assert_eq!(
        setup.remaining_budget(&tx),
        Some(budget - paid_by_identity),
        "the key spent exactly what the identity paid"
    );
}

#[tokio::test]
async fn should_refuse_unpaid_a_transition_whose_storage_does_not_fit_the_remaining_budget() {
    // A profile costs far more storage than this.
    let setup = LimitedKeySetup::new(Some(10_000), None);
    let tx = setup.platform.drive.grove.start_transaction();

    let execution = setup.process(&setup.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx);
    assert_unpaid_with_code(&execution, IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED);

    // Nothing was charged through a key that was not allowed to spend it.
    assert_eq!(setup.balance(&tx), setup.identity.balance());
    assert_eq!(setup.remaining_budget(&tx), Some(10_000));
}

#[tokio::test]
async fn should_refuse_a_key_whose_budget_is_spent_at_signature_validation() {
    let setup = LimitedKeySetup::new(Some(dash_to_credits!(0.5)), None);
    setup
        .platform
        .drive
        .deduct_from_identity_key_budget(
            setup.identity.id().to_buffer(),
            setup.key_id(),
            u64::MAX,
            None,
            PlatformVersion::latest(),
        )
        .expect("expected to spend the budget");
    let tx = setup.platform.drive.grove.start_transaction();

    let execution = setup.process(&setup.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx);
    assert_unpaid_with_code(&execution, PUBLIC_KEY_BUDGET_EXHAUSTED);
    assert_eq!(setup.balance(&tx), setup.identity.balance());
}

#[tokio::test]
async fn should_count_the_user_fee_increase_against_the_budget_before_running() {
    // The budget covers the transition itself with room to spare, but not once the signer
    // multiplies the processing fee: the increase is the signer's choice, not metered work, so
    // it has to fit like storage does.
    let plain = LimitedKeySetup::new(Some(dash_to_credits!(0.5)), None);
    let tx = plain.platform.drive.grove.start_transaction();
    assert_matches!(
        plain.process(&plain.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    let plain_cost = plain.identity.balance() - plain.balance(&tx);

    let budget = plain_cost * 3;
    let setup = LimitedKeySetup::new(Some(budget), None);
    let tx = setup.platform.drive.grove.start_transaction();
    let execution = setup.process(
        &setup.profile_creation(433, u16::MAX).await,
        BLOCK_TIME_MS,
        &tx,
    );
    assert_unpaid_with_code(&execution, IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED);
    assert_eq!(setup.remaining_budget(&tx), Some(budget));

    // The same budget does cover the transition without the increase.
    assert_matches!(
        setup.process(&setup.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
}

#[tokio::test]
async fn should_refuse_an_expired_key_from_the_expiry_instant_on() {
    for (expires_at, expired) in [
        (BLOCK_TIME_MS - 1, true),
        (BLOCK_TIME_MS, true),
        (BLOCK_TIME_MS + 1, false),
    ] {
        let setup = LimitedKeySetup::new(None, Some(expires_at));
        let tx = setup.platform.drive.grove.start_transaction();
        let execution = setup.process(&setup.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx);
        if expired {
            assert_unpaid_with_code(&execution, PUBLIC_KEY_EXPIRED);
            assert_eq!(setup.balance(&tx), setup.identity.balance());
        } else {
            assert_matches!(
                execution,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "a key expiring at {expires_at} signs at {BLOCK_TIME_MS}"
            );
            // A key that only expires has no budget entry to keep.
            assert_eq!(setup.remaining_budget(&tx), None);
        }
    }
}

#[tokio::test]
async fn should_enforce_both_limits_on_a_key_that_has_both() {
    let setup = LimitedKeySetup::new(Some(dash_to_credits!(0.5)), Some(BLOCK_TIME_MS + 10));
    let tx = setup.platform.drive.grove.start_transaction();

    assert_matches!(
        setup.process(&setup.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    let remaining = setup
        .remaining_budget(&tx)
        .expect("expected a remaining budget");
    assert!(remaining < dash_to_credits!(0.5));

    // Budget left, but the key has expired by the next block.
    let balance = setup.balance(&tx);
    let execution = setup.process(
        &setup.profile_deletion(433, 3).await,
        BLOCK_TIME_MS + 10,
        &tx,
    );
    assert_unpaid_with_code(&execution, PUBLIC_KEY_EXPIRED);
    assert_eq!(setup.remaining_budget(&tx), Some(remaining));
    assert_eq!(setup.balance(&tx), balance);
}

#[tokio::test]
async fn should_not_charge_an_invalid_transition_through_a_key_that_may_not_pay_for_it() {
    // An invalid transition is normally paid for (a penalty and a nonce bump). When the key that
    // signed it has expired there is nobody to charge it to, which is the situation of an
    // identity that cannot afford the penalty: the transition is dropped from the block.
    let setup = LimitedKeySetup::new(None, Some(BLOCK_TIME_MS + 10));
    let tx = setup.platform.drive.grove.start_transaction();
    assert_matches!(
        setup.process(&setup.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    let balance = setup.balance(&tx);

    // A second profile for the same owner breaks the unique index.
    let execution = setup.process(
        &setup.profile_creation_with_nonce(434, 0, 3).await,
        BLOCK_TIME_MS + 10,
        &tx,
    );
    // Not `UnpaidConsensusError(PublicKeyExpired)`: an invalid transition whose penalty cannot
    // be charged is reported the way it always was for an identity that cannot afford it, as an
    // internal error carrying the original failure (the shipped block loop leaves the payment
    // failure out of the message). Require exactly that, so that no other internal failure can
    // pass for it; the balance and nonce checks below show nothing was charged.
    let StateTransitionExecutionResult::InternalError(message) = &execution else {
        panic!("expected the transition to be dropped as unpayable, got {execution:?}");
    };
    assert!(message.contains("duplicate unique properties"), "{message}");
    assert_eq!(setup.balance(&tx), balance);
    let dashpay_id = setup
        .platform
        .drive
        .cache
        .system_data_contracts
        .load_dashpay(PlatformVersion::latest())
        .expect("expected dashpay")
        .id();
    assert_eq!(
        setup
            .platform
            .drive
            .fetch_identity_contract_nonce(
                setup.identity.id().to_buffer(),
                dashpay_id.to_buffer(),
                true,
                Some(&tx),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the nonce")
            .map(|nonce| nonce & dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER),
        Some(2),
        "the nonce of the dropped transition was not bumped"
    );
}

#[tokio::test]
async fn should_spend_from_the_budget_when_a_failed_transition_is_paid_for() {
    // The same profile twice: the second creation fails state validation (the document already
    // exists) and is still paid for, through the same key.
    let budget = dash_to_credits!(0.5);
    let setup = LimitedKeySetup::new(Some(budget), None);
    let tx = setup.platform.drive.grove.start_transaction();

    let creation = setup.profile_creation(433, 0).await;
    assert_matches!(
        setup.process(&creation, BLOCK_TIME_MS, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    let balance_after_creation = setup.balance(&tx);
    let remaining_after_creation = setup
        .remaining_budget(&tx)
        .expect("expected a remaining budget");

    // A fresh identity contract nonce, so that the failure is the duplicate document.
    let second = setup.profile_creation_with_nonce(433, 0, 3).await;

    let execution = setup.process(&second, BLOCK_TIME_MS, &tx);
    assert_matches!(execution, PaidConsensusError { .. });
    let penalty = balance_after_creation - setup.balance(&tx);
    assert!(penalty > 0);
    assert_eq!(
        setup.remaining_budget(&tx),
        Some(remaining_after_creation - penalty)
    );
}

#[tokio::test]
async fn should_ignore_limits_the_key_does_not_have() {
    // The signing key of an ordinary identity: nothing is read or written for it.
    let setup = LimitedKeySetup::new(None, None);
    let tx = setup.platform.drive.grove.start_transaction();
    assert_matches!(
        setup.process(&setup.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    assert_eq!(setup.remaining_budget(&tx), None);
}

#[tokio::test]
async fn should_let_only_metered_processing_take_a_key_over_its_budget() {
    // What the profile costs, from a key that can afford anything.
    let reference = LimitedKeySetup::new(Some(dash_to_credits!(0.5)), None);
    let tx = reference.platform.drive.grove.start_transaction();
    let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = reference.process(
        &reference.profile_creation(433, 0).await,
        BLOCK_TIME_MS,
        &tx,
    ) else {
        panic!("expected the reference profile to be created");
    };
    let (storage_fee, processing_fee) = (fee_result.storage_fee, fee_result.processing_fee);
    assert!(storage_fee > 0 && processing_fee > 1);

    // One credit short of the storage: refused, however little is missing.
    let short = LimitedKeySetup::new(Some(storage_fee - 1), None);
    let tx = short.platform.drive.grove.start_transaction();
    let execution = short.process(&short.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx);
    assert_unpaid_with_code(&execution, IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED);
    assert_eq!(short.remaining_budget(&tx), Some(storage_fee - 1));

    // The storage fits with a credit to spare: the processing fee takes the key over its budget,
    // the identity pays it in full, and nothing is left of the budget.
    let exact = LimitedKeySetup::new(Some(storage_fee + 1), None);
    let tx = exact.platform.drive.grove.start_transaction();
    assert_matches!(
        exact.process(&exact.profile_creation(433, 0).await, BLOCK_TIME_MS, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    assert_eq!(
        exact.identity.balance() - exact.balance(&tx),
        storage_fee + processing_fee
    );
    assert_eq!(exact.remaining_budget(&tx), Some(0));

    // A spent key signs nothing more, not even a deletion that would cost it nothing.
    let execution = exact.process(&exact.profile_deletion(433, 3).await, BLOCK_TIME_MS, &tx);
    assert_unpaid_with_code(&execution, PUBLIC_KEY_BUDGET_EXHAUSTED);
}

#[tokio::test]
async fn should_keep_transitions_of_unusable_keys_out_of_the_mempool() {
    use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
    use crate::platform_types::platform::PlatformRef;

    // Check tx has no block of its own and judges the expiry against the last committed block
    // time, which is 0 on a fresh chain.
    for (budget, expires_at, expected_code) in [
        (None, Some(0), Some(PUBLIC_KEY_EXPIRED)),
        (
            Some(10_000),
            None,
            Some(IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED),
        ),
        (Some(dash_to_credits!(0.5)), Some(1), None),
    ] {
        let setup = LimitedKeySetup::new(budget, expires_at);
        let version = PlatformVersion::latest();
        let state = setup.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &setup.platform.drive,
            state: &state,
            config: &setup.platform.config,
            core_rpc: &setup.platform.core_rpc,
        };
        let raw = setup
            .profile_creation(433, 0)
            .await
            .serialize_to_bytes()
            .expect("expected to serialize");

        let check_result = setup
            .platform
            .check_tx(&raw, FirstTimeCheck, &platform_ref, version)
            .expect("expected to check tx");
        match expected_code {
            Some(code) => assert!(
                matches!(check_result.errors.as_slice(), [error] if error.code() == code),
                "expected {code}, got {:?}",
                check_result.errors
            ),
            None => assert!(check_result.is_valid(), "{:?}", check_result.errors),
        }
    }
}

/// A data contract commits its owner beyond the transition that sets it up or changes it. From
/// protocol version 15 a key with a budget or an expiry signs neither a contract create nor a
/// contract update.
mod data_contracts {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::DataContract;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::tests::json_document::json_document_to_contract_with_ids;

    const PUBLIC_KEY_WITH_LIMITS_CANNOT_SIGN_DATA_CONTRACT_TRANSITION: u32 = 20018;

    const CONTRACT_PATH: &str =
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json";

    /// An identity whose signing key is stored with the given limits, on a chain at the given
    /// protocol version.
    struct ContractSigner {
        platform: TempPlatform<MockCoreRPCLike>,
        /// The identity as the signer knows it: its signing key carries no limits there.
        identity: Identity,
        signer: SimpleSigner,
        signing_key: IdentityPublicKey,
        version: &'static PlatformVersion,
    }

    impl ContractSigner {
        fn new(
            version: &'static PlatformVersion,
            budget: Option<Credits>,
            expires_at: Option<TimestampMillis>,
        ) -> Self {
            let platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(version.protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();
            let (identity, signer, signing_key) =
                setup_identity_without_adding_it(958, dash_to_credits!(2));
            let mut stored_identity = identity.clone();
            if budget.is_some() || expires_at.is_some() {
                stored_identity.add_public_key(signing_key.clone().with_limits(budget, expires_at));
            }
            platform
                .drive
                .add_new_identity(
                    stored_identity,
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("expected to add the identity");
            Self {
                platform,
                identity,
                signer,
                signing_key,
                version,
            }
        }

        /// A contract of the identity, as a create with the first identity nonce names it
        fn contract(&self) -> DataContract {
            let mut contract = json_document_to_contract_with_ids(
                CONTRACT_PATH,
                Some(DataContract::generate_data_contract_id_v0(
                    self.identity.id(),
                    1,
                )),
                Some(self.identity.id()),
                false,
                self.version,
            )
            .expect("expected the contract");
            contract.set_config(
                DataContractConfig::default_for_version(self.version)
                    .expect("expected the default config"),
            );
            contract
        }

        async fn contract_creation(&self) -> StateTransition {
            DataContractCreateTransition::new_from_data_contract(
                self.contract(),
                1,
                &self.identity.clone().into_partial_identity_info(),
                self.signing_key.id(),
                &self.signer,
                self.version,
                None,
            )
            .await
            .expect("expected a contract create transition")
        }

        /// Stores the contract, as a key without limits would have registered it, and returns
        /// its update to version 2 signed by the signing key
        async fn contract_update_of_a_stored_contract(&self) -> StateTransition {
            let mut contract = self.contract();
            self.platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    self.version,
                )
                .expect("expected to store the contract");
            contract.set_version(2);
            DataContractUpdateTransition::new_from_data_contract(
                contract,
                &self.identity.clone().into_partial_identity_info(),
                self.signing_key.id(),
                1,
                0,
                &self.signer,
                self.version,
                None,
            )
            .await
            .expect("expected a contract update transition")
        }

        fn process(
            &self,
            transition: &StateTransition,
            tx: &drive::grovedb::Transaction,
        ) -> StateTransitionExecutionResult {
            let state = self.platform.state.load();
            let result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition
                        .serialize_to_bytes()
                        .expect("expected to serialize")],
                    &state,
                    &BlockInfo {
                        time_ms: BLOCK_TIME_MS,
                        ..Default::default()
                    },
                    tx,
                    self.version,
                    false,
                    None,
                )
                .expect("expected to process the state transition");
            result.execution_results()[0].clone()
        }

        fn balance(&self, tx: &drive::grovedb::Transaction) -> Credits {
            self.platform
                .drive
                .fetch_identity_balance(self.identity.id().to_buffer(), Some(tx), self.version)
                .expect("expected to fetch the balance")
                .expect("expected a balance")
        }

        fn remaining_budget(&self, tx: &drive::grovedb::Transaction) -> Option<Credits> {
            self.platform
                .drive
                .fetch_identity_key_remaining_budget(
                    self.identity.id().to_buffer(),
                    self.signing_key.id(),
                    Some(tx),
                    self.version,
                )
                .expect("expected to fetch the remaining budget")
        }

        fn identity_nonce(&self, tx: &drive::grovedb::Transaction) -> Option<IdentityNonce> {
            self.platform
                .drive
                .fetch_identity_nonce(self.identity.id().to_buffer(), true, Some(tx), self.version)
                .expect("expected to fetch the identity nonce")
        }

        /// Nothing was taken from the identity, from the key or from the nonce
        fn assert_untouched(&self, budget: Option<Credits>, tx: &drive::grovedb::Transaction) {
            assert_eq!(self.balance(tx), self.identity.balance());
            assert_eq!(self.remaining_budget(tx), budget);
            assert_eq!(self.identity_nonce(tx).unwrap_or_default(), 0);
        }
    }

    fn protocol_version_14() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("expected protocol version 14")
    }

    #[tokio::test]
    async fn should_refuse_unpaid_a_contract_create_signed_by_a_key_with_a_budget() {
        // The budget covers the registration many times over: what is refused is the kind of
        // transition, not its price.
        let budget = Some(dash_to_credits!(1.5));
        let setup = ContractSigner::new(PlatformVersion::latest(), budget, None);
        let tx = setup.platform.drive.grove.start_transaction();

        let execution = setup.process(&setup.contract_creation().await, &tx);
        assert_unpaid_with_code(
            &execution,
            PUBLIC_KEY_WITH_LIMITS_CANNOT_SIGN_DATA_CONTRACT_TRANSITION,
        );
        setup.assert_untouched(budget, &tx);
        assert!(
            setup
                .platform
                .drive
                .fetch_contract(
                    setup.contract().id().to_buffer(),
                    None,
                    None,
                    Some(&tx),
                    setup.version
                )
                .unwrap()
                .expect("expected to look the contract up")
                .is_none(),
            "the contract was not registered"
        );
    }

    #[tokio::test]
    async fn should_refuse_unpaid_a_contract_create_signed_by_a_key_with_an_expiry() {
        // An expiry alone is refused too, long before it is reached.
        let setup =
            ContractSigner::new(PlatformVersion::latest(), None, Some(BLOCK_TIME_MS * 1_000));
        let tx = setup.platform.drive.grove.start_transaction();

        let execution = setup.process(&setup.contract_creation().await, &tx);
        assert_unpaid_with_code(
            &execution,
            PUBLIC_KEY_WITH_LIMITS_CANNOT_SIGN_DATA_CONTRACT_TRANSITION,
        );
        setup.assert_untouched(None, &tx);
    }

    #[tokio::test]
    async fn should_refuse_unpaid_a_contract_update_signed_by_a_key_with_limits() {
        for (budget, expires_at) in [
            (Some(dash_to_credits!(1.5)), None),
            (None, Some(BLOCK_TIME_MS * 1_000)),
        ] {
            let setup = ContractSigner::new(PlatformVersion::latest(), budget, expires_at);
            let update = setup.contract_update_of_a_stored_contract().await;
            let tx = setup.platform.drive.grove.start_transaction();

            let execution = setup.process(&update, &tx);
            assert_unpaid_with_code(
                &execution,
                PUBLIC_KEY_WITH_LIMITS_CANNOT_SIGN_DATA_CONTRACT_TRANSITION,
            );
            setup.assert_untouched(budget, &tx);
        }
    }

    #[tokio::test]
    async fn should_let_a_key_without_limits_create_and_update_a_contract() {
        let setup = ContractSigner::new(PlatformVersion::latest(), None, None);
        let tx = setup.platform.drive.grove.start_transaction();
        assert_matches!(
            setup.process(&setup.contract_creation().await, &tx),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let setup = ContractSigner::new(PlatformVersion::latest(), None, None);
        let update = setup.contract_update_of_a_stored_contract().await;
        let tx = setup.platform.drive.grove.start_transaction();
        assert_matches!(
            setup.process(&update, &tx),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_keep_a_contract_create_signed_by_a_key_with_limits_out_of_the_mempool() {
        use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
        use crate::platform_types::platform::PlatformRef;

        let setup =
            ContractSigner::new(PlatformVersion::latest(), Some(dash_to_credits!(1.5)), None);
        let state = setup.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &setup.platform.drive,
            state: &state,
            config: &setup.platform.config,
            core_rpc: &setup.platform.core_rpc,
        };
        let raw = setup
            .contract_creation()
            .await
            .serialize_to_bytes()
            .expect("expected to serialize");

        let check_result = setup
            .platform
            .check_tx(&raw, FirstTimeCheck, &platform_ref, setup.version)
            .expect("expected to check tx");
        assert!(
            matches!(
                check_result.errors.as_slice(),
                [error] if error.code() == PUBLIC_KEY_WITH_LIMITS_CANNOT_SIGN_DATA_CONTRACT_TRANSITION
            ),
            "{:?}",
            check_result.errors
        );
    }

    /// Protocol version 14 keeps `validate_state_transition_identity_signed` 1, so the blocks
    /// committed under it replay with the same results.
    #[tokio::test]
    async fn should_let_a_key_with_limits_create_and_update_a_contract_at_protocol_version_14() {
        let budget = dash_to_credits!(1.5);
        let setup = ContractSigner::new(protocol_version_14(), Some(budget), None);
        let tx = setup.platform.drive.grove.start_transaction();
        assert_matches!(
            setup.process(&setup.contract_creation().await, &tx),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let paid_by_identity = setup.identity.balance() - setup.balance(&tx);
        assert!(paid_by_identity > 0);
        assert_eq!(setup.remaining_budget(&tx), Some(budget - paid_by_identity));

        let setup = ContractSigner::new(protocol_version_14(), Some(budget), None);
        let update = setup.contract_update_of_a_stored_contract().await;
        let tx = setup.platform.drive.grove.start_transaction();
        assert_matches!(
            setup.process(&update, &tx),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }
}
