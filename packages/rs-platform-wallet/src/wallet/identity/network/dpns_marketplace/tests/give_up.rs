//! Giving up a DPNS name: what a holder is offered, what is refused before anything is
//! signed, and that the wallet forgets a name only once the network confirmed it gone.

use super::super::super::signing_key::tests::KeyFilter;
use super::super::give_up::DpnsGiveUpTarget;
use super::*;
use dash_sdk::query_types::Documents;
use dash_sdk::{Sdk, SdkBuilder};
use dpp::data_contracts::SystemDataContract;
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;

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
    let mirror = Arc::new(MirrorPersister::hydrated(Vec::new()));
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

    wallet
        .forget_given_up_dpns_name(&owner, &plain_target(owner))
        .await;

    assert_eq!(
        dpns_labels(&wallet, &owner).await,
        vec![KEPT_LABEL.to_string()]
    );
    let stored = mirror.stored_dpns.lock().expect("stored log").clone();
    assert_eq!(stored.len(), stores_before + 1, "one row removal");
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

/// Through the public entry point, each way of giving up reaches the network as its own
/// transition (the delete of the checked document, the create of a tombstone). The mock
/// answers no nonce, so neither is confirmed, and nothing stored changes.
#[tokio::test]
async fn should_send_each_give_up_to_the_network_and_change_nothing_before_it_is_confirmed() {
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
        .unresolved()
        .insert((owner, convert_to_homograph_safe_chars(PLAIN_LABEL)));
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
        .unresolved()
        .insert((owner, convert_to_homograph_safe_chars(PLAIN_LABEL)));

    let error = wallet
        .dpns_name_give_up_choices(&owner, PLAIN_LABEL)
        .await
        .expect_err("nobody holds the name");
    assert!(
        matches!(error, PlatformWalletError::DpnsNameNotFound { .. }),
        "{error:?}"
    );
    assert!(wallet.dpns_operation_gate.unresolved().is_empty());
}
