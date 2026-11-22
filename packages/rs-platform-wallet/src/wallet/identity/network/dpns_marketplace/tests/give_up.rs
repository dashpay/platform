//! Giving up a DPNS name: what a holder is offered, what is refused before anything is
//! signed, and that the wallet forgets a name only once the network confirmed it gone.

use super::super::super::signing_key::tests::KeyFilter;
use super::super::give_up::{DpnsGiveUpTarget, DpnsOperationGate, HoldWhenSigned};
use super::*;
use dash_sdk::query_types::Documents;
use dash_sdk::{Sdk, SdkBuilder};
use dpp::data_contracts::SystemDataContract;
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;

use super::super::super::decision_proposal::tests::FixedSignatureSigner;
use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use dash_sdk::query_types::IdentityContractNonceFetcher;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::platform::dpns_usernames::normalize_dpns_label;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::{StateTransition, StateTransitionOwned};

/// A name that is not contested: its normalized form (`a11ce7`) holds a digit the contest
/// pattern does not.
const PLAIN_LABEL: &str = "Alice7";
/// A contested name: its normalized form (`a11ce`) matches the contest pattern.
const CONTESTED_LABEL: &str = "Alice";
/// Another name the owner keeps.
const KEPT_LABEL: &str = "Keeper77";

const OWNER: [u8; 32] = [0xC1; 32];
const DOCUMENT: [u8; 32] = [0xC2; 32];

/// The `domain` document of `label`, held by `owner`.
fn domain_document(label: &str, document_id: Identifier, owner: Identifier) -> Document {
    let properties = BTreeMap::from([
        ("label".to_string(), Value::Text(label.to_string())),
        (
            "normalizedLabel".to_string(),
            Value::Text(convert_to_homograph_safe_chars(label)),
        ),
        (
            "normalizedParentDomainName".to_string(),
            Value::Text(DPNS_PARENT_DOMAIN.to_string()),
        ),
    ]);
    Document::V0(dpp::document::DocumentV0 {
        id: document_id,
        owner_id: owner,
        properties,
        revision: Some(1),
        created_at: Some(1_700_000_000_000),
        ..Default::default()
    })
}

/// A mock SDK at the latest protocol version whose context provider resolves the bundled
/// DPNS contract, as a host's does. It answers the fetch of that contract and the query for
/// the domain of `label` with `domain`; any other request fails.
async fn sdk_holding(label: &str, domain: Option<Document>) -> Sdk {
    let contract = Arc::new(
        load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("bundled DPNS contract"),
    );
    let mut sdk = SdkBuilder::new_mock()
        .with_version(PlatformVersion::latest())
        .with_context_provider(DpnsContractProvider(Arc::clone(&contract)))
        .build()
        .expect("mock sdk");
    sdk.mock()
        .expect_fetch(dpns_contract_id(), Some(contract.as_ref().clone()))
        .await
        .expect("DPNS contract expectation");
    let documents: Documents = domain
        .into_iter()
        .map(|document| (document.id(), Some(document)))
        .collect();
    sdk.mock()
        .expect_fetch_many::<Identifier, Document, _, Documents>(
            domain_by_normalized_label_query(
                contract,
                convert_to_homograph_safe_chars(dpns_label(label)),
            ),
            Some(documents),
        )
        .await
        .expect("domain-document expectation");
    sdk
}

/// A wallet over `sdk` whose identity [`OWNER`] holds `labels`, and its mirror.
async fn wallet_holding(sdk: Sdk, labels: &[&str]) -> (IdentityWallet, Arc<MirrorPersister>) {
    wallet_holding_over(sdk, labels, MirrorPersister::hydrated(Vec::new())).await
}

/// [`wallet_holding`] over `mirror`, the durable storage the wallet loaded from.
async fn wallet_holding_over(
    sdk: Sdk,
    labels: &[&str],
    mirror: MirrorPersister,
) -> (IdentityWallet, Arc<MirrorPersister>) {
    let mirror = Arc::new(mirror);
    let wallet = mirror_backed_identity_wallet_with_sdk(Arc::clone(&mirror), Arc::new(sdk));
    let mut owner = identity(
        Purpose::AUTHENTICATION,
        SecurityLevel::CRITICAL,
        KeyType::ECDSA_SECP256K1,
    );
    owner.set_id(Identifier::from(OWNER));
    {
        let mut wm = wallet.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&wallet.wallet_id)
            .expect("wallet info");
        info.identity_manager
            .add_identity(owner, 0, wallet.wallet_id, &wallet.persister)
            .expect("add identity");
        info.identity_manager
            .managed_identity_mut(&Identifier::from(OWNER))
            .expect("the identity")
            .set_dpns_names(
                labels
                    .iter()
                    .map(|label| DpnsNameInfo {
                        label: label.to_string(),
                        acquired_at: Some(1),
                    })
                    .collect(),
                &wallet.persister,
            );
    }
    (wallet, mirror)
}

/// A wallet whose identity [`OWNER`] holds `label` and [`KEPT_LABEL`], on a network that
/// holds the domain of `label` under `domain_owner`.
async fn wallet_with_name(
    label: &str,
    domain_owner: Identifier,
) -> (IdentityWallet, Arc<MirrorPersister>) {
    let sdk = sdk_holding(
        label,
        Some(domain_document(
            label,
            Identifier::from(DOCUMENT),
            domain_owner,
        )),
    )
    .await;
    wallet_holding(sdk, &[label, KEPT_LABEL]).await
}

#[tokio::test]
async fn should_offer_a_delete_and_a_tombstone_for_a_name_that_is_not_contested() {
    let owner = Identifier::from(OWNER);
    let (wallet, _mirror) = wallet_with_name(PLAIN_LABEL, owner).await;

    let choices = wallet
        .dpns_name_give_up_choices(&owner, PLAIN_LABEL)
        .await
        .expect("the choices are read");
    assert_eq!(
        choices,
        DpnsNameGiveUpChoices {
            document_id: Identifier::from(DOCUMENT),
            label: PLAIN_LABEL.to_string(),
            can_delete: true,
            can_tombstone: true,
        }
    );
}

/// The network refuses the delete of a contested name, so only the tombstone is offered
#[tokio::test]
async fn should_offer_only_a_tombstone_for_a_contested_name() {
    let owner = Identifier::from(OWNER);
    let (wallet, _mirror) = wallet_with_name(CONTESTED_LABEL, owner).await;

    let choices = wallet
        .dpns_name_give_up_choices(&owner, "Alice.dash")
        .await
        .expect("the choices are read");
    assert!(!choices.can_delete);
    assert!(choices.can_tombstone);
}

#[tokio::test]
async fn should_offer_nothing_for_a_name_another_identity_holds() {
    let owner = Identifier::from(OWNER);
    let (wallet, _mirror) = wallet_with_name(PLAIN_LABEL, Identifier::from([0xD1; 32])).await;

    let error = wallet
        .dpns_name_give_up_choices(&owner, PLAIN_LABEL)
        .await
        .expect_err("another identity holds the name");
    assert!(
        matches!(error, PlatformWalletError::InvalidParameter(_)),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_report_a_name_that_is_not_registered_as_not_found() {
    let owner = Identifier::from(OWNER);
    let (wallet, _mirror) = wallet_holding(sdk_holding(PLAIN_LABEL, None).await, &[]).await;

    let error = wallet
        .dpns_name_give_up_choices(&owner, PLAIN_LABEL)
        .await
        .expect_err("nobody holds the name");
    assert!(
        matches!(error, PlatformWalletError::DpnsNameNotFound { .. }),
        "{error:?}"
    );
}

/// Before protocol version 15 no name can be given up. Both the read and the write are
/// refused with the SDK's own protocol version refusal, which the FFI maps to its own
/// result code, and the wallet keeps the name.
#[tokio::test]
async fn should_refuse_to_give_up_a_name_on_a_network_below_protocol_version_15() {
    let owner = Identifier::from(OWNER);
    let sdk = sdk_holding(
        PLAIN_LABEL,
        Some(domain_document(
            PLAIN_LABEL,
            Identifier::from(DOCUMENT),
            owner,
        )),
    )
    .await;
    sdk.mock_verified_protocol_version(14)
        .expect("the mock takes a verified version");
    let (wallet, mirror) = wallet_holding(sdk, &[PLAIN_LABEL, KEPT_LABEL]).await;

    let not_active = |error: &PlatformWalletError| {
        matches!(
            error,
            PlatformWalletError::Sdk(dash_sdk::Error::ProtocolVersionNotActive {
                required: 15,
                network: 14,
                pinned: false,
                cause: None,
                ..
            })
        )
    };
    let read = wallet
        .dpns_name_give_up_choices(&owner, PLAIN_LABEL)
        .await
        .expect_err("the read is refused");
    assert!(not_active(&read), "{read:?}");
    for method in [
        DpnsNameGiveUpMethod::Delete,
        DpnsNameGiveUpMethod::Tombstone,
    ] {
        let written = wallet
            .give_up_dpns_name(
                &owner,
                PLAIN_LABEL,
                &Identifier::from(DOCUMENT),
                method,
                &lock_checking_signer(&wallet),
            )
            .await
            .expect_err("the give-up is refused");
        assert!(not_active(&written), "{method:?}: {written:?}");
    }
    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
    assert!(mirror.stored_dpns_removals().is_empty());
}

/// A contested name is not deleted: the wallet refuses before a key is selected, where the
/// signer would be asked
#[tokio::test]
async fn should_refuse_to_delete_a_contested_name_before_anything_is_signed() {
    let owner = Identifier::from(OWNER);
    let (wallet, mirror) = wallet_with_name(CONTESTED_LABEL, owner).await;

    let error = wallet
        .give_up_dpns_name(
            &owner,
            CONTESTED_LABEL,
            &Identifier::from(DOCUMENT),
            DpnsNameGiveUpMethod::Delete,
            &lock_checking_signer(&wallet),
        )
        .await
        .expect_err("a contested name is not deleted");
    assert!(
        matches!(error, PlatformWalletError::InvalidParameter(ref message)
            if message.contains("can only be tombstoned")),
        "{error:?}"
    );
    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![CONTESTED_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
    assert!(mirror.stored_dpns_removals().is_empty());
}

/// A give-up the network did not accept changes nothing stored: here the signer holds no
/// key, so nothing is broadcast, and the label and the marketplace row both stay
#[tokio::test]
async fn should_keep_a_name_whose_give_up_was_not_accepted() {
    let owner = Identifier::from(OWNER);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
    let mut row = name_state_entry(Identifier::from(DOCUMENT), owner, DpnsNameSaleStatus::Owned);
    row.label = PLAIN_LABEL.to_string();
    row.normalized_label = convert_to_homograph_safe_chars(PLAIN_LABEL);
    wallet.record_dpns_name_states(vec![row], vec![]).await;

    let error = wallet
        .give_up_dpns_name(
            &owner,
            PLAIN_LABEL,
            &Identifier::from(DOCUMENT),
            DpnsNameGiveUpMethod::Delete,
            &lock_checking_signer(&wallet),
        )
        .await
        .expect_err("no key is available to sign with");
    assert!(
        matches!(error,
            PlatformWalletError::Sdk(dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(ref message)))
                if message.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX)
        ),
        "expected signer-unavailable error, got {error:?}"
    );
    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
    assert!(mirror.stored_dpns_removals().is_empty());
    assert_eq!(
        wallet
            .local_dpns_name_states(Some(&owner))
            .await
            .expect("the rows are read")
            .len(),
        1
    );
}

/// What [`IdentityWallet::dpns_give_up_target`] read for [`PLAIN_LABEL`], held by `owner`.
fn plain_target(owner: Identifier) -> DpnsGiveUpTarget {
    DpnsGiveUpTarget {
        choices: DpnsNameGiveUpChoices {
            document_id: Identifier::from(DOCUMENT),
            label: PLAIN_LABEL.to_string(),
            can_delete: true,
            can_tombstone: true,
        },
        domain: domain_document(PLAIN_LABEL, Identifier::from(DOCUMENT), owner),
        state: DpnsDomainState::from_document(&domain_document(
            PLAIN_LABEL,
            Identifier::from(DOCUMENT),
            owner,
        ))
        .expect("the domain reads as marketplace state"),
    }
}

/// An `Owned` marketplace row of `label` under `document_id`, held by `owner`.
fn owned_row(document_id: Identifier, owner: Identifier, label: &str) -> DpnsNameStateEntry {
    let mut row = name_state_entry(document_id, owner, DpnsNameSaleStatus::Owned);
    row.label = label.to_string();
    row.normalized_label = convert_to_homograph_safe_chars(label);
    row
}

/// Once the network confirmed a name gone the wallet drops the name's marketplace rows (the
/// checked one and a registration made again elsewhere), then its label, and keeps the
/// owner's other names
#[tokio::test]
async fn should_forget_a_given_up_name_rows_first_and_keep_the_others() {
    let owner = Identifier::from(OWNER);
    let document_id = Identifier::from(DOCUMENT);
    let again = Identifier::from([0xC3; 32]);
    let kept = Identifier::from([0xC4; 32]);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
    wallet
        .record_dpns_name_states(
            vec![
                owned_row(document_id, owner, PLAIN_LABEL),
                owned_row(again, owner, PLAIN_LABEL),
                owned_row(kept, owner, KEPT_LABEL),
            ],
            vec![],
        )
        .await;
    let stores_before = mirror.stored_dpns.lock().expect("stored log").len();
    let order_before = mirror.store_order().len();

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![KEPT_LABEL.to_string()]
    );
    let stored = mirror.stored_dpns.lock().expect("stored log").clone();
    assert_eq!(stored.len(), stores_before + 1, "one row removal");
    // The row removal is stored before the identity snapshot that drops the label: the label
    // is what lets a later sync find a row whose removal failed
    assert_eq!(
        mirror.store_order()[order_before..],
        ["rows", "identity"],
        "rows first, then the label"
    );
    assert_eq!(
        stored.last().expect("the give-up changeset").removed,
        [document_id, again].into_iter().collect()
    );
    assert_eq!(
        mirror.stored_label_lists(&owner).last(),
        Some(&vec![KEPT_LABEL.to_string()])
    );
    let rows = wallet
        .local_dpns_name_states(Some(&owner))
        .await
        .expect("the rows are read");
    assert_eq!(
        rows.iter().map(|row| row.document_id).collect::<Vec<_>>(),
        vec![kept]
    );
}

/// A backend that refuses the row removal leaves the wallet as it was: the label stays, so
/// the next marketplace sync pass finds the name gone and removes it
#[tokio::test]
async fn should_keep_the_label_when_the_row_removal_is_not_stored() {
    let owner = Identifier::from(OWNER);
    let document_id = Identifier::from(DOCUMENT);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
    wallet
        .record_dpns_name_states(vec![owned_row(document_id, owner, PLAIN_LABEL)], vec![])
        .await;
    mirror.fail_stores();

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
    assert_eq!(
        wallet
            .local_dpns_name_states(Some(&owner))
            .await
            .expect("the rows are read")
            .len(),
        1
    );
}

/// A signer able to sign with every key: key selection passes and the give-up reaches the
/// network. It is never asked to sign, as the mock SDK answers no nonce.
fn signer_with_every_key() -> KeyFilter<impl Fn(&IdentityPublicKey) -> bool + Send + Sync> {
    KeyFilter(|_| true)
}

/// Through the public entry point, each way of giving up gets as far as reading the
/// identity contract nonce of its transition, before anything is signed. The mock answers
/// no nonce, so nothing is sent, nothing stored changes and nothing is held.
#[tokio::test]
async fn should_change_nothing_and_hold_nothing_when_a_give_up_stops_before_signing() {
    let owner = Identifier::from(OWNER);
    for method in [
        DpnsNameGiveUpMethod::Delete,
        DpnsNameGiveUpMethod::Tombstone,
    ] {
        let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
        wallet
            .record_dpns_name_states(
                vec![owned_row(Identifier::from(DOCUMENT), owner, PLAIN_LABEL)],
                vec![],
            )
            .await;

        let error = wallet
            .give_up_dpns_name(
                &owner,
                PLAIN_LABEL,
                &Identifier::from(DOCUMENT),
                method,
                &signer_with_every_key(),
            )
            .await
            .expect_err("the mock answers no nonce");
        assert!(
            error
                .to_string()
                .contains("GetIdentityContractNonceRequest"),
            "{method:?}: expected the give-up to stop at its nonce fetch, got {error}"
        );
        assert_eq!(
            dpns_labels(&wallet, &owner).await,
            vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()],
            "{method:?}"
        );
        assert!(mirror.stored_dpns_removals().is_empty(), "{method:?}");
        // Nothing left the wallet, so the give-up is not held as unresolved
        assert!(wallet.dpns_operation_gate.holds_nothing(), "{method:?}");
        assert_eq!(
            wallet
                .local_dpns_name_states(Some(&owner))
                .await
                .expect("the rows are read")
                .len(),
            1,
            "{method:?}"
        );
    }
}

/// The user is shown one registration; a name deleted and registered again since (here the
/// network holds another document) is refused before anything is signed
#[tokio::test]
async fn should_refuse_to_give_up_a_registration_the_user_was_not_shown() {
    let owner = Identifier::from(OWNER);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;

    for method in [
        DpnsNameGiveUpMethod::Delete,
        DpnsNameGiveUpMethod::Tombstone,
    ] {
        let error = wallet
            .give_up_dpns_name(
                &owner,
                PLAIN_LABEL,
                &Identifier::from([0xEE; 32]),
                method,
                &lock_checking_signer(&wallet),
            )
            .await
            .expect_err("another registration is refused");
        assert!(
            matches!(error, PlatformWalletError::InvalidParameter(ref message)
                if message.contains("registered again")),
            "{method:?}: {error:?}"
        );
    }
    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
    assert!(mirror.stored_dpns_removals().is_empty());
}

/// A wallet loaded cold holds no marketplace row of the name in memory. When the backend
/// refuses the row removal, the wallet keeps the label and a row for the checked document,
/// so the next marketplace sync removes the durable row by its id
#[tokio::test]
async fn should_keep_a_row_of_the_checked_document_when_a_cold_wallet_cannot_store_the_removal() {
    let owner = Identifier::from(OWNER);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
    mirror.fail_stores();

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
    let rows = wallet
        .local_dpns_name_states(Some(&owner))
        .await
        .expect("the rows are read");
    assert_eq!(
        rows.iter().map(|row| row.document_id).collect::<Vec<_>>(),
        vec![Identifier::from(DOCUMENT)]
    );
}

/// While an earlier give-up of the name is unconfirmed and the name is still held, neither
/// the choices nor another give-up proceed: both report it as unconfirmed (FFI code 20)
#[tokio::test]
async fn should_refuse_another_give_up_while_an_earlier_one_is_unconfirmed() {
    let owner = Identifier::from(OWNER);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
    wallet
        .dpns_operation_gate
        .hold((owner, convert_to_homograph_safe_chars(PLAIN_LABEL)));
    let unconfirmed = |error: &PlatformWalletError| matches!(error, PlatformWalletError::Sdk(sdk) if sdk.is_state_transition_result_unconfirmed());

    let read = wallet
        .dpns_name_give_up_choices(&owner, PLAIN_LABEL)
        .await
        .expect_err("the earlier give-up is unconfirmed");
    assert!(unconfirmed(&read), "{read:?}");
    for method in [
        DpnsNameGiveUpMethod::Delete,
        DpnsNameGiveUpMethod::Tombstone,
    ] {
        let written = wallet
            .give_up_dpns_name(
                &owner,
                PLAIN_LABEL,
                &Identifier::from(DOCUMENT),
                method,
                &lock_checking_signer(&wallet),
            )
            .await
            .expect_err("the earlier give-up is unconfirmed");
        assert!(unconfirmed(&written), "{method:?}: {written:?}");
    }
    assert!(mirror.stored_dpns_removals().is_empty());
}

/// A read showing the name no longer registered settles an unconfirmed give-up: it executed
#[tokio::test]
async fn should_settle_an_unconfirmed_give_up_once_the_name_is_gone() {
    let owner = Identifier::from(OWNER);
    let (wallet, _mirror) = wallet_holding(sdk_holding(PLAIN_LABEL, None).await, &[]).await;
    wallet
        .dpns_operation_gate
        .hold((owner, convert_to_homograph_safe_chars(PLAIN_LABEL)));

    let error = wallet
        .dpns_name_give_up_choices(&owner, PLAIN_LABEL)
        .await
        .expect_err("nobody holds the name");
    assert!(
        matches!(error, PlatformWalletError::DpnsNameNotFound { .. }),
        "{error:?}"
    );
    assert!(wallet.dpns_operation_gate.holds_nothing());
}

/// A name asked for with a `.DASH` suffix settles the hold of the label the SDK queried
#[tokio::test]
async fn should_settle_an_unconfirmed_give_up_asked_for_with_a_mixed_case_suffix() {
    let owner = Identifier::from(OWNER);
    let (wallet, _mirror) = wallet_holding(sdk_holding(PLAIN_LABEL, None).await, &[]).await;
    wallet
        .dpns_operation_gate
        .hold((owner, convert_to_homograph_safe_chars(PLAIN_LABEL)));

    let error = wallet
        .dpns_name_give_up_choices(&owner, &format!("{PLAIN_LABEL}.DASH"))
        .await
        .expect_err("nobody holds the name");
    assert!(
        matches!(error, PlatformWalletError::DpnsNameNotFound { .. }),
        "{error:?}"
    );
    assert!(wallet.dpns_operation_gate.holds_nothing());
}

/// A signer answering every signature with `signs`, for [`HoldWhenSigned`].
#[derive(Debug)]
struct Answers {
    signs: bool,
}

#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for Answers {
    async fn sign(
        &self,
        _: &IdentityPublicKey,
        _: &[u8],
    ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
        if self.signs {
            Ok(vec![7u8; 65].into())
        } else {
            Err(dpp::ProtocolError::Generic("the host declined".to_string()))
        }
    }

    async fn sign_create_witness(
        &self,
        _: &IdentityPublicKey,
        _: &[u8],
    ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
        Err(dpp::ProtocolError::Generic("no witness here".to_string()))
    }

    fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
        true
    }
}

/// The give-up is held only once the host's signature returns: a declined (or never
/// reached) signature holds nothing, so a call cancelled before it can not leave the name
/// held as sent
#[tokio::test]
async fn should_hold_a_give_up_only_once_it_is_signed() {
    let key = identity(
        Purpose::AUTHENTICATION,
        SecurityLevel::CRITICAL,
        KeyType::ECDSA_SECP256K1,
    )
    .public_keys()
    .values()
    .next()
    .expect("a key")
    .clone();
    for signs in [false, true] {
        let held = std::sync::atomic::AtomicBool::new(false);
        let hold = || held.store(true, std::sync::atomic::Ordering::SeqCst);
        let signer = Answers { signs };
        let wrapped = HoldWhenSigned {
            signer: &signer,
            hold: &hold,
        };
        let signed = wrapped.sign(&key, b"transition").await;
        assert_eq!(signed.is_ok(), signs);
        assert_eq!(held.load(std::sync::atomic::Ordering::SeqCst), signs);
    }
}

/// The identity contract nonce the mock reports for [`OWNER`] on the DPNS contract.
const STORED_NONCE: u64 = 4;

/// Through the public entry point each way is sent as its own transition: a delete is the
/// delete of the checked `domain` document, signed with the owner's first eligible key at
/// the next nonce, and a tombstone sends no delete. The mock answers no broadcast, so
/// neither is confirmed and nothing stored changes.
#[tokio::test]
async fn should_send_the_delete_of_the_checked_document_only_for_a_delete() {
    let owner = Identifier::from(OWNER);
    for method in [
        DpnsNameGiveUpMethod::Delete,
        DpnsNameGiveUpMethod::Tombstone,
    ] {
        let mut sdk = sdk_holding(
            PLAIN_LABEL,
            Some(domain_document(
                PLAIN_LABEL,
                Identifier::from(DOCUMENT),
                owner,
            )),
        )
        .await;
        sdk.mock()
            .expect_fetch(
                (owner, dpns_contract_id()),
                Some(IdentityContractNonceFetcher(STORED_NONCE)),
            )
            .await
            .expect("expects the nonce read");
        let (wallet, mirror) = wallet_holding(sdk.clone(), &[PLAIN_LABEL, KEPT_LABEL]).await;

        let error = wallet
            .give_up_dpns_name(
                &owner,
                PLAIN_LABEL,
                &Identifier::from(DOCUMENT),
                method,
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");

        // The delete exactly as the wallet would build it from the domain it checked
        let checked = sdk
            .dpns_name_give_up_options(PLAIN_LABEL)
            .await
            .expect("the options are read")
            .expect("the name is registered")
            .domain;
        let contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("bundled DPNS contract");
        let owner_identity = {
            let wm = wallet.wallet_manager.read().await;
            wm.get_wallet_info(&wallet.wallet_id)
                .expect("wallet info")
                .identity_manager
                .wallet_identity(&wallet.wallet_id, &owner)
                .expect("the owner")
                .identity
                .clone()
        };
        let first_key = owner_identity
            .public_keys()
            .values()
            .next()
            .expect("a key")
            .clone();
        let delete = BatchTransition::new_document_deletion_transition_from_document(
            checked,
            contract
                .document_type_for_name(DPNS_DOCUMENT_TYPE)
                .expect("the domain type"),
            &first_key,
            STORED_NONCE + 1,
            0,
            None,
            &FixedSignatureSigner,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("the delete");
        let delete_broadcast = delete
            .broadcast_request_for_state_transition()
            .expect("serializes");
        let deletes_sent = sdk.mock().request_count(&delete_broadcast).await;
        match method {
            DpnsNameGiveUpMethod::Delete => {
                assert_eq!(
                    deletes_sent, 1,
                    "the delete of the checked document is sent"
                )
            }
            DpnsNameGiveUpMethod::Tombstone => {
                assert_eq!(deletes_sent, 0, "a tombstone sends no delete");
                // The tombstone's entropy is random, so its request can not be rebuilt; the
                // mock's refusal prints the request it refused, which is read back here
                let sent = refused_broadcast(&error.to_string());
                let StateTransition::Batch(batch) = sent else {
                    panic!("a tombstone is a batch transition, got {sent:?}");
                };
                assert_eq!(batch.owner_id(), owner);
                assert_eq!(batch.transitions_len(), 1, "one transition, so no delete");
                let Some(BatchedTransitionRef::Document(DocumentTransition::Create(create))) =
                    batch.first_transition()
                else {
                    panic!("the tombstone is a document create");
                };
                assert_eq!(create.base().document_type_name(), "tombstone");
                assert_eq!(
                    create.data().get("normalizedLabel"),
                    Some(&Value::Text(normalize_dpns_label(PLAIN_LABEL)))
                );
                assert_eq!(
                    create.data().get("normalizedParentDomainName"),
                    Some(&Value::Text("dash".to_string()))
                );
            }
        }
        assert_eq!(
            dpns_labels(&wallet, &owner).await,
            vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()],
            "{method:?}"
        );
        assert!(mirror.stored_dpns_removals().is_empty(), "{method:?}");
    }
}

/// A buffered row removal whose commit fails leaves everything in place: the label and the
/// row stay, so the next marketplace sync finds the name gone and removes both
#[tokio::test]
async fn should_keep_the_label_and_row_when_the_row_removal_is_not_committed() {
    let owner = Identifier::from(OWNER);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
    wallet
        .record_dpns_name_states(
            vec![owned_row(Identifier::from(DOCUMENT), owner, PLAIN_LABEL)],
            vec![],
        )
        .await;
    mirror.fail_flushes();

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
    assert_eq!(
        wallet
            .local_dpns_name_states(Some(&owner))
            .await
            .expect("the rows are read")
            .len(),
        1
    );
}

/// A committed row removal followed by a failed identity snapshot drops the row and keeps
/// the label, which the next marketplace sync resolves as a departure
#[tokio::test]
async fn should_keep_the_label_when_the_snapshot_without_it_is_not_stored() {
    let owner = Identifier::from(OWNER);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
    wallet
        .record_dpns_name_states(
            vec![owned_row(Identifier::from(DOCUMENT), owner, PLAIN_LABEL)],
            vec![],
        )
        .await;
    mirror.fail_identity_stores();

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    assert_eq!(
        mirror.stored_dpns_removals(),
        vec![Identifier::from(DOCUMENT)]
    );
    assert!(wallet
        .local_dpns_name_states(Some(&owner))
        .await
        .expect("the rows are read")
        .is_empty());
    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
}

/// A read settles only the attempt it saw holding the name when it started: a give-up
/// signed after the read began keeps its hold whatever the read finds
#[test]
fn should_release_a_hold_only_for_the_attempt_a_read_saw() {
    let gate = DpnsOperationGate::default();
    let key = (
        Identifier::from(OWNER),
        convert_to_homograph_safe_chars(PLAIN_LABEL),
    );

    // The read starts while nothing is held, then a give-up is signed
    let seen = gate.holder(&key);
    let later = gate.hold(key.clone());
    gate.release(&key, seen);
    assert_eq!(gate.holder(&key), Some(later));

    // The read starts while `later` holds; a newer attempt takes over before it settles
    let seen = gate.holder(&key);
    let newer = gate.hold(key.clone());
    gate.release(&key, seen);
    assert_eq!(gate.holder(&key), Some(newer));

    // A read that saw the current attempt settles it
    gate.release(&key, Some(newer));
    assert!(gate.holds_nothing());
}

/// The state transition a mock SDK refused to answer, read back from its refusal: the mock
/// prints the refused `BroadcastStateTransitionRequest` with `{:?}`, whose
/// `state_transition` bytes are the serialized transition.
fn refused_broadcast(error: &str) -> StateTransition {
    let start = error
        .find("state_transition: [")
        .map(|at| at + "state_transition: [".len())
        .unwrap_or_else(|| panic!("no refused broadcast in {error}"));
    let end = start
        + error[start..]
            .find(']')
            .unwrap_or_else(|| panic!("an unterminated byte list in {error}"));
    let bytes: Vec<u8> = error[start..end]
        .split(',')
        .map(|byte| byte.trim().parse().expect("a byte"))
        .collect();
    StateTransition::deserialize_from_bytes_untrusted_in_version(&bytes, PlatformVersion::latest())
        .expect("a serialized state transition")
}

/// A wallet loaded cold holds no marketplace row in memory, while durable storage still
/// holds an `Owned` row of an older registration of the name (deleted and registered again
/// from another client). Giving up the current registration removes both rows before it
/// drops the label, so nothing is left marked owned on disk.
#[tokio::test]
async fn should_remove_the_stored_row_of_an_older_registration_of_the_name() {
    let owner = Identifier::from(OWNER);
    let older = Identifier::from([0xC5; 32]);
    let sdk = sdk_holding(
        PLAIN_LABEL,
        Some(domain_document(
            PLAIN_LABEL,
            Identifier::from(DOCUMENT),
            owner,
        )),
    )
    .await;
    let (wallet, mirror) = wallet_holding_over(
        sdk,
        &[PLAIN_LABEL, KEPT_LABEL],
        MirrorPersister::hydrated(vec![owned_row(older, owner, PLAIN_LABEL)]),
    )
    .await;

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    let removed: BTreeSet<Identifier> = mirror.stored_dpns_removals().into_iter().collect();
    assert_eq!(
        removed,
        [Identifier::from(DOCUMENT), older].into_iter().collect()
    );
    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![KEPT_LABEL.to_string()]
    );
}

/// When the stored rows can not be read, the label is kept as the cue for the next
/// marketplace sync; the checked document's row is still removed
#[tokio::test]
async fn should_keep_the_label_when_the_stored_rows_can_not_be_read() {
    let owner = Identifier::from(OWNER);
    let sdk = sdk_holding(
        PLAIN_LABEL,
        Some(domain_document(
            PLAIN_LABEL,
            Identifier::from(DOCUMENT),
            owner,
        )),
    )
    .await;
    let (wallet, mirror) = wallet_holding_over(
        sdk,
        &[PLAIN_LABEL, KEPT_LABEL],
        MirrorPersister::failing_with(PersistenceErrorKind::Fatal),
    )
    .await;

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    assert_eq!(
        mirror.stored_dpns_removals(),
        vec![Identifier::from(DOCUMENT)]
    );
    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()]
    );
}

/// The identity also owns the same label under another parent (a subdomain registration).
/// Giving up the `dash` registration leaves that registration's row, its sale price and the
/// shared label alone.
#[tokio::test]
async fn should_leave_a_registration_of_the_label_under_another_parent_alone() {
    let owner = Identifier::from(OWNER);
    let sibling = Identifier::from([0xC6; 32]);
    let (wallet, mirror) = wallet_with_name(PLAIN_LABEL, owner).await;
    let mut sibling_row = owned_row(sibling, owner, PLAIN_LABEL);
    sibling_row.normalized_parent_domain_name = "f00.dash".to_string();
    sibling_row.price = Some(5_000);
    wallet
        .record_dpns_name_states(
            vec![
                owned_row(Identifier::from(DOCUMENT), owner, PLAIN_LABEL),
                sibling_row.clone(),
            ],
            vec![],
        )
        .await;

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    assert_eq!(
        mirror.stored_dpns_removals(),
        vec![Identifier::from(DOCUMENT)]
    );
    let rows = wallet
        .local_dpns_name_states(Some(&owner))
        .await
        .expect("the rows are read");
    assert_eq!(rows, vec![sibling_row]);
    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![PLAIN_LABEL.to_string(), KEPT_LABEL.to_string()],
        "the other registration still needs the label"
    );
}

/// An owner whose only authentication key is an `ECDSA_HASH160` one the signer can use gets
/// a give-up to the network: the key is selected and the write reaches its nonce read
#[tokio::test]
async fn should_give_up_with_an_ecdsa_hash160_key() {
    let owner = Identifier::from(OWNER);
    let sdk = sdk_holding(
        PLAIN_LABEL,
        Some(domain_document(
            PLAIN_LABEL,
            Identifier::from(DOCUMENT),
            owner,
        )),
    )
    .await;
    let mirror = Arc::new(MirrorPersister::hydrated(Vec::new()));
    let wallet = mirror_backed_identity_wallet_with_sdk(Arc::clone(&mirror), Arc::new(sdk));
    let mut hash160_owner = identity(
        Purpose::AUTHENTICATION,
        SecurityLevel::CRITICAL,
        KeyType::ECDSA_HASH160,
    );
    hash160_owner.set_id(owner);
    wallet
        .wallet_manager
        .write()
        .await
        .get_wallet_info_mut(&wallet.wallet_id)
        .expect("wallet info")
        .identity_manager
        .add_identity(hash160_owner, 0, wallet.wallet_id, &wallet.persister)
        .expect("add identity");

    for method in [
        DpnsNameGiveUpMethod::Delete,
        DpnsNameGiveUpMethod::Tombstone,
    ] {
        let error = wallet
            .give_up_dpns_name(
                &owner,
                PLAIN_LABEL,
                &Identifier::from(DOCUMENT),
                method,
                &signer_with_every_key(),
            )
            .await
            .expect_err("the mock answers no nonce");
        assert!(
            error
                .to_string()
                .contains("GetIdentityContractNonceRequest"),
            "{method:?}: expected the give-up to reach its nonce read, got {error}"
        );
    }
}
