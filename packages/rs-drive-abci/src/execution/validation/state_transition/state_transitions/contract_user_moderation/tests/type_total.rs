//! A moderator's restore puts a document's summed value back in its type's sums (protocol
//! version 15): it is held to the total the type keeps of its summed property, as a create
//! is. A moderator's field change never writes that property.

use super::*;
use dpp::data_contract::config::v1::DataContractConfigSettersV1;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use drive::grovedb::TransactionArg;

const DOCUMENT_SUM_OVERFLOW: u32 = 40154;
/// A type whose `amount` only its moderators write.
const GRANT: &str = "grant";

/// A post moderators may delete, keeping the total of an `amount` that is never negative.
/// The contract does not size its integers, so `amount` reaches `i64::MAX`.
fn post_schema_keeping_its_total() -> Value {
    post_schema_with(platform_value!({
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
            "amount": { "type": "integer", "minimum": 0, "position": 1 }
        },
        "required": ["text", "amount"],
        "documentsSummable": "amount",
    }))
}

/// A grant keeping the total of an `amount` that only moderators write, required or not.
fn grant_schema(amount_is_required: bool) -> Value {
    let required = if amount_is_required {
        platform_value!(["text", "amount"])
    } else {
        platform_value!(["text"])
    };
    platform_value!({
        "type": "object",
        "moderatorAbilities": { "changeFields": ["amount"] },
        "documentsSummable": "amount",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
            "amount": { "type": "integer", "minimum": 0, "position": 1 }
        },
        "required": required,
        "additionalProperties": false,
    })
}

async fn setup() -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            contract.config_mut().set_sized_integer_types_enabled(false);
            add_document_type(contract, POST, post_schema_keeping_its_total());
        },
    )
    .await
}

async fn create(
    setup: &Setup,
    actor: &Actor,
    document_type_name: &str,
    text: &str,
    amount: i64,
    tx: &Transaction<'_>,
) -> Document {
    let (document, transition) = create_with(
        setup,
        actor,
        document_type_name,
        vec![
            ("text", Value::Text(text.to_string())),
            ("amount", Value::I64(amount)),
        ],
    )
    .await;
    assert_success(&setup.process(&transition, tx));
    setup
        .stored_document(document_type_name, document.id(), Some(tx))
        .expect("expected the created document")
}

/// The total the type keeps of `amount`.
fn total(setup: &Setup, document_type_name: &str, tx: TransactionArg) -> i64 {
    let document_type = setup
        .contract
        .document_type_for_name(document_type_name)
        .expect("expected the document type");
    setup
        .platform
        .drive
        .fetch_document_type_sum(
            &setup.contract,
            document_type,
            None,
            tx,
            PlatformVersion::latest(),
        )
        .expect("expected to read the total")
        .0
}

fn nonce(setup: &Setup, actor: &Actor, tx: TransactionArg) -> IdentityNonce {
    setup
        .platform
        .drive
        .fetch_identity_contract_nonce(
            actor.id().to_buffer(),
            setup.contract.id().to_buffer(),
            true,
            tx,
            PlatformVersion::latest(),
        )
        .expect("expected to read the nonce")
        .unwrap_or_default()
        & IDENTITY_NONCE_VALUE_FILTER
}

/// The removed post of `removed_amount` and its bytes, beside a live post that takes the
/// total to `live_amount`.
async fn removed_post_beside_a_live_one(
    setup: &Setup,
    removed_amount: i64,
    live_amount: i64,
) -> (Document, Vec<u8>, Document) {
    let tx = setup.platform.drive.grove.start_transaction();
    let removed = create(setup, &setup.user, POST, "removed", removed_amount, &tx).await;
    let bytes = setup.document_bytes(POST, &removed);
    let deletion = setup
        .moderate(&setup.moderator, delete_action(POST, removed.id()))
        .await;
    assert_success(&setup.process(&deletion, &tx));
    let live = create(setup, &setup.stranger, POST, "live", live_amount, &tx).await;
    setup.commit(tx);
    assert_eq!(total(setup, POST, None), live_amount);
    (removed, bytes, live)
}

#[tokio::test]
async fn should_refuse_a_restore_past_the_types_total_as_a_paid_error() {
    let setup = setup().await;
    let (removed, bytes, live) = removed_post_beside_a_live_one(&setup, 5, i64::MAX - 4).await;
    let removal = setup.post_removal(removed.id(), None).unwrap();

    let expected_nonce = setup.moderator.next_contract_nonce.get();
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    let balance = setup.balance(setup.moderator.id(), None);
    assert_eq!(
        setup
            .check_tx(&restore)
            .iter()
            .map(ErrorWithCode::code)
            .collect::<Vec<_>>(),
        vec![DOCUMENT_SUM_OVERFLOW]
    );

    let tx = setup.platform.drive.grove.start_transaction();
    let result = setup.process(&restore, &tx);
    assert_paid_with_code(&result, DOCUMENT_SUM_OVERFLOW);
    let StateTransitionExecutionResult::PaidConsensusError {
        error: ConsensusError::StateError(StateError::DocumentSumOverflowError(error)),
        actual_fees,
        ..
    } = &result
    else {
        panic!("expected a paid sum overflow refusal, got {result:?}");
    };
    assert_eq!(error.document_type_name(), POST);
    assert_eq!(error.summed_property(), "amount");
    assert_eq!(error.limit(), i64::MAX);
    assert!(actual_fees.total_base_fee() > 0);
    assert_eq!(
        setup.balance(setup.moderator.id(), Some(&tx)),
        balance - actual_fees.total_base_fee()
    );
    assert_eq!(nonce(&setup, &setup.moderator, Some(&tx)), expected_nonce);

    // Nothing of the restore is kept: the post stays removed, its record unrestored.
    assert!(setup
        .stored_document(POST, removed.id(), Some(&tx))
        .is_none());
    assert_eq!(setup.post_removal(removed.id(), Some(&tx)), Some(removal));
    assert_eq!(
        setup.stored_document(POST, live.id(), Some(&tx)),
        Some(live)
    );
    assert_eq!(total(&setup, POST, Some(&tx)), i64::MAX - 4);
}

#[tokio::test]
async fn should_restore_a_document_that_takes_the_total_to_the_most_it_holds() {
    let setup = setup().await;
    let (removed, bytes, _) = removed_post_beside_a_live_one(&setup, 5, i64::MAX - 5).await;

    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert!(setup.check_tx(&restore).is_empty());
    let tx = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&restore, &tx));
    assert_eq!(total(&setup, POST, Some(&tx)), i64::MAX);
    assert_eq!(
        setup.stored_document(POST, removed.id(), Some(&tx)),
        Some(removed)
    );
}

/// Why a field change reads no total: the property a type sums is never one its moderators
/// write. A summed property is required, and a field moderators write is not.
#[test]
fn should_refuse_a_summed_property_among_the_fields_moderators_write() {
    for (amount_is_required, reason) in [
        (
            true,
            "it is required, and a document's owner can not set it",
        ),
        (
            false,
            "must be listed in the document type's `required` array",
        ),
    ] {
        let mut contract =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned();
        contract.set_config(
            contract
                .config()
                .clone()
                .with_moderation(Some(moderators_without_lists())),
        );
        contract.config_mut().set_sized_integer_types_enabled(false);
        let error = contract
            .set_document_schema(
                GRANT,
                grant_schema(amount_is_required),
                true,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect_err("a summed property moderators write must be refused");
        assert!(
            format!("{error:?}").contains(reason),
            "expected the refusal to say {reason}, got {error:?}"
        );
    }
}
