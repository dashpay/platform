//! Fields only a contract's moderators write (`moderatorAbilities.changeFields`, protocol
//! version 14): a moderator's change through the moderation transition, and the gate on the
//! creates and replaces of a document's own owner.

use super::*;
use dpp::data_contract::config::v0::{DataContractConfigGettersV0, DataContractConfigSettersV0};
use dpp::data_contract::config::DataContractConfig;
use drive::config::DriveConfig;

const REPORT: &str = "report";
const TICKET: &str = "ticket";
const DOCUMENT_FIELD_NOT_CHANGEABLE_BY_MODERATORS: u32 = 41123;
const DOCUMENT_MODERATOR_FIELD_NOT_WRITABLE: u32 = 41124;
const INVALID_CONTRACT_MODERATION_DOCUMENT_FIELDS: u32 = 10905;
const DOCUMENT_NOT_FOUND: u32 = 40101;
const INVALID_DOCUMENT_REVISION: u32 = 40106;
const JSON_SCHEMA: u32 = 10101;
const DOCUMENT_PROPERTY_CONSTRAINT_VIOLATED: u32 = 10422;
const NOTICE: &str = "notice";
const CARD: &str = "card";

/// A report of a post, which its author can not replace, which moderators may delete, and
/// whose `status` and `resolution` only they write, queried by who handled it last and when.
/// Its `postId` refers to a post that may be deleted.
fn report_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "moderatorAbilities": { "delete": true, "changeFields": ["status", "resolution"] },
        "indices": [
            { "name": "byStatus", "properties": [{ "status": "asc" }, { "$createdAt": "asc" }] },
            { "name": "byModeratedAt", "properties": [{ "$moderatedAt": "asc" }] },
            {
                "name": "byModerator",
                "properties": [{ "$moderatedBy": "asc" }, { "$moderatedAt": "asc" }],
            },
        ],
        "properties": {
            "postId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 0,
                "refersTo": { "type": "deletableDocument", "documentType": POST },
            },
            "reason": { "type": "integer", "minimum": 0, "maximum": 8, "position": 1 },
            "status": { "type": "integer", "minimum": 1, "maximum": 3, "position": 2 },
            "resolution": { "type": "string", "minLength": 1, "maxLength": 100, "position": 3 },
        },
        "required": ["$createdAt", "postId", "reason"],
        "additionalProperties": false,
    })
}

/// A ticket its author may replace, whose `slot` and `weight` only moderators write, no two
/// tickets holding the same slot
fn ticket_schema() -> Value {
    platform_value!({
        "type": "object",
        "moderatorAbilities": { "changeFields": ["slot", "weight"] },
        "indices": [
            { "name": "bySlot", "properties": [{ "slot": "asc" }], "unique": true },
        ],
        "properties": {
            "title": { "type": "string", "minLength": 1, "maxLength": 50, "position": 0 },
            "slot": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 },
            "weight": { "type": "number", "minimum": 0, "position": 2 },
        },
        "required": ["title"],
        "additionalProperties": false,
    })
}

/// A notice that expires an hour after its creation, whose `status` only moderators write and
/// may never set to 3
fn notice_schema() -> Value {
    platform_value!({
        "type": "object",
        "ttl": 3600,
        "moderatorAbilities": { "changeFields": ["status"] },
        "properties": {
            "text": { "type": "string", "minLength": 1, "maxLength": 50, "position": 0 },
            "status": { "type": "integer", "minimum": 1, "maximum": 3, "position": 1 },
        },
        "propertyConstraints": {
            "neverThree": { "anyOf": [{ "absent": "status" }, { "notEqual": ["status", 3] }] },
        },
        "required": ["$createdAt", "text"],
        "additionalProperties": false,
    })
}

/// A card its owner may give away, whose `grade` only moderators write
fn card_schema() -> Value {
    platform_value!({
        "type": "object",
        "transferable": 1,
        "moderatorAbilities": { "changeFields": ["grade"] },
        "properties": {
            "name": { "type": "string", "minLength": 1, "maxLength": 50, "position": 0 },
            "grade": { "type": "integer", "minimum": 0, "maximum": 10, "position": 1 },
        },
        "required": ["name"],
        "additionalProperties": false,
    })
}

/// A contract whose owner and named moderator moderate posts, reports, tickets, notices and
/// cards
async fn setup() -> Setup {
    setup_where(|_| {}).await
}

/// The same contract, with `modify_config` applied to its config first
async fn setup_where(modify_config: impl FnOnce(&mut DataContractConfig)) -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            let mut config = contract.config().clone();
            modify_config(&mut config);
            contract.set_config(config);
            add_document_type(contract, POST, post_schema(true));
            add_document_type(contract, REPORT, report_schema());
            add_document_type(contract, TICKET, ticket_schema());
            add_document_type(contract, NOTICE, notice_schema());
            add_document_type(contract, CARD, card_schema());
        },
    )
    .await
}

/// A notice by `actor`, without the status only moderators write
async fn create_notice(setup: &Setup, actor: &Actor) -> (Document, StateTransition) {
    setup
        .create_document_of_type_with(actor, NOTICE, |document| {
            let properties = document.properties_mut();
            properties.remove("status");
            properties.insert(
                "text".to_string(),
                Value::Text("closed on monday".to_string()),
            );
        })
        .await
}

fn change_action(
    document_type_name: &str,
    document_id: Identifier,
    fields: Value,
) -> ContractUserModerationAction {
    ContractUserModerationAction::ChangeDocumentFields {
        document_type_name: document_type_name.to_string(),
        document_id,
        fields: fields
            .into_btree_string_map()
            .expect("expected a map of fields"),
        reason: ContractModerationReason::from_text("handled"),
    }
}

/// A report by `actor` of the post `post_id`, with `fields` set beside its own
async fn create_report(
    setup: &Setup,
    actor: &Actor,
    post_id: Identifier,
    fields: Value,
) -> (Document, StateTransition) {
    setup
        .create_document_of_type_with(actor, REPORT, |document| {
            let properties = document.properties_mut();
            // The random document fills every optional field; a report starts without the
            // moderators' ones.
            properties.remove("status");
            properties.remove("resolution");
            properties.insert("postId".to_string(), Value::Identifier(post_id.to_buffer()));
            properties.insert("reason".to_string(), Value::U64(1));
            properties.extend(
                fields
                    .into_btree_string_map()
                    .expect("expected a map of fields"),
            );
        })
        .await
}

/// A ticket by `actor` titled `title`, with `fields` set beside it
async fn create_ticket(
    setup: &Setup,
    actor: &Actor,
    title: &str,
    fields: Value,
) -> (Document, StateTransition) {
    setup
        .create_document_of_type_with(actor, TICKET, |document| {
            let properties = document.properties_mut();
            properties.clear();
            properties.insert("title".to_string(), Value::Text(title.to_string()));
            properties.extend(
                fields
                    .into_btree_string_map()
                    .expect("expected a map of fields"),
            );
        })
        .await
}

/// A replacement of `document`, a ticket, by `actor`, its owner
async fn replace_ticket(setup: &Setup, actor: &Actor, document: Document) -> StateTransition {
    BatchTransition::new_document_replacement_transition_from_document(
        document,
        setup
            .contract
            .document_type_for_name(TICKET)
            .expect("expected the ticket type"),
        &actor.key,
        actor.contract_nonce(),
        0,
        None,
        &actor.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the replacement")
}

impl Setup {
    /// The ids of the reports `query` (a where and order-by query) returns
    fn query_reports(&self, query: Value, transaction: Option<&Transaction>) -> Vec<Identifier> {
        let platform_version = PlatformVersion::latest();
        let document_type = self
            .contract
            .document_type_for_name(REPORT)
            .expect("expected the report type");
        let query = DriveDocumentQuery::from_value(
            query,
            &self.contract,
            document_type,
            &DriveConfig::default(),
            platform_version,
        )
        .expect("expected a valid query");
        self.platform
            .drive
            .query_documents(query, None, false, transaction, None)
            .expect("expected to query the reports")
            .documents_owned()
            .iter()
            .map(|document| document.id())
            .collect()
    }

    /// Proves the committed state for a document field change and returns the document the
    /// proof shows
    fn assert_change_proved(&self, transition: &StateTransition) -> Document {
        let platform_version = PlatformVersion::latest();
        let proof = self
            .platform
            .drive
            .prove_state_transition(transition, None, platform_version)
            .expect("expected to prove the state transition")
            .into_data()
            .expect("expected proof bytes");
        let known_contracts: BTreeMap<Identifier, DataContract> =
            BTreeMap::from([(self.contract.id(), self.contract.clone())]);
        let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            &proof,
            &|id| Ok(known_contracts.get(id).cloned().map(std::sync::Arc::new)),
            platform_version,
        )
        .expect("expected the proof to verify");
        assert!(
            !outcome.is_execution_proved(),
            "expected AffectedState, got {:?}",
            outcome
        );
        match outcome.into_result() {
            StateTransitionProofResult::VerifiedDocuments(mut documents) => {
                assert_eq!(documents.len(), 1);
                documents
                    .pop_first()
                    .and_then(|(_, document)| document)
                    .expect("expected the changed document")
            }
            other => panic!("expected the changed document, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn should_let_a_moderator_change_the_fields_only_moderators_write() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (report, create_report) =
        create_report(&setup, &setup.stranger, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create_report, &transaction));
    let reported = setup
        .stored_document(REPORT, report.id(), Some(&transaction))
        .expect("expected the report to be stored");
    // Its author can not replace a report, and still it carries a revision: a moderator's
    // change is stored as an update.
    assert_eq!(reported.revision(), Some(1));
    assert_eq!(reported.get("status"), None);
    setup.commit(transaction);

    let moderator_balance = setup.balance(setup.moderator.id(), None);
    let author_balance = setup.balance(setup.stranger.id(), None);
    let transaction = setup.platform.drive.grove.start_transaction();
    let change = setup
        .moderate(
            &setup.moderator,
            change_action(
                REPORT,
                report.id(),
                platform_value!({ "status": 2u64, "resolution": "the post was removed" }),
            ),
        )
        .await;
    assert_success(&setup.process_at(&change, BLOCK_TIME_MS + 5_000, &transaction));
    let changed = setup
        .stored_document(REPORT, report.id(), Some(&transaction))
        .expect("expected the report to stay");
    assert!(changed
        .get("status")
        .is_some_and(|status| status.equal_underlying_data(&Value::U64(2))));
    assert_eq!(
        changed.get("resolution"),
        Some(&Value::Text("the post was removed".to_string()))
    );
    // Everything else is as its author wrote it: the moderator's change is no modification of
    // theirs, so no clock moved.
    assert_eq!(changed.revision(), Some(2));
    assert_eq!(changed.owner_id(), reported.owner_id());
    assert_eq!(changed.created_at(), reported.created_at());
    assert_eq!(changed.updated_at(), reported.updated_at());
    assert_eq!(changed.get("postId"), reported.get("postId"));
    assert_eq!(changed.get("reason"), reported.get("reason"));
    setup.commit(transaction);
    // The moderator paid for it, and the author did not: what the change removed of the bytes
    // the author paid for (the report's entry under an absent status) is refunded to them.
    assert!(setup.balance(setup.moderator.id(), None) < moderator_balance);
    assert!(setup.balance(setup.stranger.id(), None) >= author_balance);

    let proved = setup.assert_change_proved(&change);
    assert_eq!(proved.id(), report.id());
    assert_eq!(proved.revision(), Some(2));

    // A `null` removes a field, and the contract owner moderates too.
    let transaction = setup.platform.drive.grove.start_transaction();
    let clear = setup
        .moderate(
            &setup.owner,
            change_action(REPORT, report.id(), platform_value!({ "resolution": null })),
        )
        .await;
    assert_success(&setup.process(&clear, &transaction));
    let cleared = setup
        .stored_document(REPORT, report.id(), Some(&transaction))
        .expect("expected the report to stay");
    assert_eq!(cleared.get("resolution"), None);
    assert!(cleared.get("status").is_some());
    assert_eq!(cleared.revision(), Some(3));
    setup.commit(transaction);
    assert_eq!(setup.assert_change_proved(&clear).get("resolution"), None);
}

#[tokio::test]
async fn should_resolve_a_report_whose_post_a_moderator_deleted() {
    // A replace checks every deletableDocument reference again, and would refuse a report
    // whose post is gone; a moderator's change leaves the references as they were checked.
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (report, create_report) =
        create_report(&setup, &setup.stranger, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create_report, &transaction));

    let delete_post = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete_post, &transaction));
    assert!(setup
        .stored_document(POST, post.id(), Some(&transaction))
        .is_none());

    let resolve = setup
        .moderate(
            &setup.moderator,
            change_action(REPORT, report.id(), platform_value!({ "status": 3u64 })),
        )
        .await;
    assert_success(&setup.process(&resolve, &transaction));
    assert!(setup
        .stored_document(REPORT, report.id(), Some(&transaction))
        .and_then(|report| report.get("status").cloned())
        .is_some_and(|status| status.equal_underlying_data(&Value::U64(3))));
}

#[tokio::test]
async fn should_refuse_a_field_change_that_breaks_a_rule() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (report, create_report) =
        create_report(&setup, &setup.stranger, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create_report, &transaction));

    // A field the type does not keep for its moderators, and a type that keeps none.
    let not_listed = setup
        .moderate(
            &setup.moderator,
            change_action(REPORT, report.id(), platform_value!({ "reason": 1u64 })),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&not_listed, &transaction),
        DOCUMENT_FIELD_NOT_CHANGEABLE_BY_MODERATORS,
    );
    // The mempool refuses it with the same code as a block
    assert!(setup
        .check_tx(&not_listed)
        .iter()
        .any(|error| error.code() == DOCUMENT_FIELD_NOT_CHANGEABLE_BY_MODERATORS));
    let not_kept = setup
        .moderate(
            &setup.moderator,
            change_action(POST, post.id(), platform_value!({ "text": "edited" })),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&not_kept, &transaction),
        DOCUMENT_FIELD_NOT_CHANGEABLE_BY_MODERATORS,
    );

    // Only a moderator changes them: not the author of the report, not anyone else.
    for actor in [&setup.stranger, &setup.user] {
        let by_a_user = setup
            .moderate(
                actor,
                change_action(REPORT, report.id(), platform_value!({ "status": 2u64 })),
            )
            .await;
        assert_paid_with_code(
            &setup.process(&by_a_user, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }

    // A value the schema refuses.
    let out_of_range = setup
        .moderate(
            &setup.moderator,
            change_action(REPORT, report.id(), platform_value!({ "status": 9u64 })),
        )
        .await;
    assert_paid_with_code(&setup.process(&out_of_range, &transaction), JSON_SCHEMA);

    // A document or a type that does not exist.
    let missing = setup
        .moderate(
            &setup.moderator,
            change_action(
                REPORT,
                Identifier::from([0xEE; 32]),
                platform_value!({ "status": 2u64 }),
            ),
        )
        .await;
    assert_paid_with_code(&setup.process(&missing, &transaction), DOCUMENT_NOT_FOUND);
    let unknown_type = setup
        .moderate(
            &setup.moderator,
            change_action("nothing", report.id(), platform_value!({ "status": 2u64 })),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&unknown_type, &transaction),
        INVALID_DOCUMENT_TYPE,
    );

    // No field at all, or a system one, is refused before any state is read.
    for fields in [platform_value!({}), platform_value!({ "$updatedAt": 5u64 })] {
        let malformed = setup
            .moderate(&setup.moderator, change_action(REPORT, report.id(), fields))
            .await;
        assert_unpaid_with_code(
            &setup.process(&malformed, &transaction),
            INVALID_CONTRACT_MODERATION_DOCUMENT_FIELDS,
        );
    }

    // Nothing was changed by any of it.
    let stored = setup
        .stored_document(REPORT, report.id(), Some(&transaction))
        .expect("expected the report");
    assert_eq!(stored.revision(), Some(1));
    assert_eq!(stored.get("status"), None);
}

#[tokio::test]
async fn should_let_only_moderators_write_those_fields_in_their_own_documents() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));

    // A user can not file a report already marked handled.
    let (_, preset) = create_report(
        &setup,
        &setup.stranger,
        post.id(),
        platform_value!({ "status": 3u64 }),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&preset, &transaction),
        DOCUMENT_MODERATOR_FIELD_NOT_WRITABLE,
    );
    // The mempool refuses it too: the transformer judges it, not the block's state validation.
    assert!(setup
        .check_tx(&preset)
        .iter()
        .any(|error| error.code() == DOCUMENT_MODERATOR_FIELD_NOT_WRITABLE));

    // A moderator, and the contract owner, who moderates too, may.
    for actor in [&setup.moderator, &setup.owner] {
        let (_, by_a_moderator) = create_report(
            &setup,
            actor,
            post.id(),
            platform_value!({ "status": 1u64 }),
        )
        .await;
        assert_success(&setup.process(&by_a_moderator, &transaction));
    }

    // A ticket's author may replace it, but not its slot: neither setting it, changing it nor
    // removing it.
    let (mut ticket, create_ticket) =
        create_ticket(&setup, &setup.user, "printer", platform_value!({})).await;
    assert_success(&setup.process(&create_ticket, &transaction));
    ticket.set("slot", Value::U64(4));
    ticket.increment_revision().expect("expected a revision");
    let sets_the_slot = replace_ticket(&setup, &setup.user, ticket.clone()).await;
    assert_paid_with_code(
        &setup.process(&sets_the_slot, &transaction),
        DOCUMENT_MODERATOR_FIELD_NOT_WRITABLE,
    );

    let assign = setup
        .moderate(
            &setup.moderator,
            change_action(TICKET, ticket.id(), platform_value!({ "slot": 4u64 })),
        )
        .await;
    assert_success(&setup.process(&assign, &transaction));
    let assigned = setup
        .stored_document(TICKET, ticket.id(), Some(&transaction))
        .expect("expected the ticket");
    assert_eq!(assigned.revision(), Some(2));

    // A replace built on the revision before the moderator's change is refused: the author
    // rereads the ticket rather than writing over the change.
    let mut stale = ticket.clone();
    stale.set("title", Value::Text("scanner".to_string()));
    stale.properties_mut().remove("slot");
    let stale_replace = replace_ticket(&setup, &setup.user, stale).await;
    assert_paid_with_code(
        &setup.process(&stale_replace, &transaction),
        INVALID_DOCUMENT_REVISION,
    );

    // Built on the ticket as it now stands, a replace that keeps the slot passes, and one that
    // changes or drops it does not.
    let mut changes_the_slot = assigned.clone();
    changes_the_slot.set("slot", Value::U64(5));
    changes_the_slot
        .increment_revision()
        .expect("expected a revision");
    let replace = replace_ticket(&setup, &setup.user, changes_the_slot).await;
    assert_paid_with_code(
        &setup.process(&replace, &transaction),
        DOCUMENT_MODERATOR_FIELD_NOT_WRITABLE,
    );
    let mut drops_the_slot = assigned.clone();
    drops_the_slot.properties_mut().remove("slot");
    drops_the_slot
        .increment_revision()
        .expect("expected a revision");
    let replace = replace_ticket(&setup, &setup.user, drops_the_slot).await;
    assert_paid_with_code(
        &setup.process(&replace, &transaction),
        DOCUMENT_MODERATOR_FIELD_NOT_WRITABLE,
    );
    let mut keeps_the_slot = assigned.clone();
    keeps_the_slot.set("title", Value::Text("scanner".to_string()));
    keeps_the_slot
        .increment_revision()
        .expect("expected a revision");
    let replace = replace_ticket(&setup, &setup.user, keeps_the_slot).await;
    assert_success(&setup.process(&replace, &transaction));
    let replaced = setup
        .stored_document(TICKET, ticket.id(), Some(&transaction))
        .expect("expected the ticket");
    assert_eq!(
        replaced.get("title"),
        Some(&Value::Text("scanner".to_string()))
    );
    assert!(replaced
        .get("slot")
        .is_some_and(|slot| slot.equal_underlying_data(&Value::U64(4))));
}

#[tokio::test]
async fn should_keep_the_unique_indexes_over_fields_moderators_write() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (first, create_first) =
        create_ticket(&setup, &setup.user, "printer", platform_value!({})).await;
    assert_success(&setup.process(&create_first, &transaction));
    let (second, create_second) =
        create_ticket(&setup, &setup.stranger, "scanner", platform_value!({})).await;
    assert_success(&setup.process(&create_second, &transaction));

    let assign = |ticket: Identifier, slot: u64| {
        setup.moderate(
            &setup.moderator,
            change_action(TICKET, ticket, platform_value!({ "slot": slot })),
        )
    };
    assert_success(&setup.process(&assign(first.id(), 1).await, &transaction));
    // Another ticket can not take the slot the first holds.
    assert_paid_with_code(
        &setup.process(&assign(second.id(), 1).await, &transaction),
        DUPLICATE_UNIQUE_INDEX,
    );
    // Giving the first the slot it already holds changes nothing: refused rather than written.
    assert_paid_with_code(
        &setup.process(&assign(first.id(), 1).await, &transaction),
        INVALID_CONTRACT_MODERATION_DOCUMENT_FIELDS,
    );
    // Once the first gives the slot up, the second can take it.
    let release = setup
        .moderate(
            &setup.moderator,
            change_action(TICKET, first.id(), platform_value!({ "slot": null })),
        )
        .await;
    assert_success(&setup.process(&release, &transaction));
    assert_success(&setup.process(&assign(second.id(), 1).await, &transaction));
}

#[tokio::test]
async fn should_tie_the_fields_to_the_moderation_declaration() {
    // No moderation, no fields only moderators write: there would be nobody to write them.
    let error = {
        let mut contract =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned();
        contract
            .set_document_schema(
                REPORT,
                ticket_schema(),
                true,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect_err("expected the keyword to be refused without moderation")
    };
    assert!(
        format!("{error:?}").contains("moderatorAbilities"),
        "expected the refusal to name the keyword, got {error:?}"
    );

    // A document type an update adds may keep fields for its moderators; an existing one can
    // not start or stop keeping them.
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let mut flipped = setup.contract.clone();
    flipped.increment_version();
    let mut schema = flipped
        .document_type_for_name(TICKET)
        .expect("expected the ticket type")
        .schema()
        .clone();
    if let Value::Map(map) = &mut schema {
        map.retain(|(key, _)| key != &Value::Text("moderatorAbilities".to_string()));
    }
    add_document_type(&mut flipped, TICKET, schema);
    let update = setup.contract_update(flipped).await;
    assert_paid_with_code(&setup.process(&update, &transaction), DOCUMENT_TYPE_UPDATE);

    // A type the update adds may keep them.
    let mut added = setup.contract.clone();
    added.increment_version();
    add_document_type(&mut added, "queue", ticket_schema());
    let update = setup.contract_update(added).await;
    assert_success(&setup.process(&update, &transaction));
}

#[tokio::test]
async fn should_refuse_a_change_that_changes_nothing() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (report, create_report) =
        create_report(&setup, &setup.stranger, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create_report, &transaction));

    // Removing a field the report does not hold, and setting one to the value it holds.
    let remove_absent = setup
        .moderate(
            &setup.moderator,
            change_action(REPORT, report.id(), platform_value!({ "status": null })),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&remove_absent, &transaction),
        INVALID_CONTRACT_MODERATION_DOCUMENT_FIELDS,
    );
    let set = setup
        .moderate(
            &setup.moderator,
            change_action(REPORT, report.id(), platform_value!({ "status": 2u64 })),
        )
        .await;
    assert_success(&setup.process(&set, &transaction));
    let set_again = setup
        .moderate(
            &setup.moderator,
            change_action(REPORT, report.id(), platform_value!({ "status": 2u64 })),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&set_again, &transaction),
        INVALID_CONTRACT_MODERATION_DOCUMENT_FIELDS,
    );
    // Neither bumped the revision.
    assert_eq!(
        setup
            .stored_document(REPORT, report.id(), Some(&transaction))
            .and_then(|report| report.revision()),
        Some(2)
    );
}

#[tokio::test]
async fn should_refuse_a_change_its_type_rules_or_its_expiry_forbid() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (notice, create) = create_notice(&setup, &setup.user).await;
    assert_success(&setup.process(&create, &transaction));

    // The type's `propertyConstraints` judge the changed document as they judge a replace.
    let three = setup
        .moderate(
            &setup.moderator,
            change_action(NOTICE, notice.id(), platform_value!({ "status": 3u64 })),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&three, &transaction),
        DOCUMENT_PROPERTY_CONSTRAINT_VIOLATED,
    );
    let two = setup
        .moderate(
            &setup.moderator,
            change_action(NOTICE, notice.id(), platform_value!({ "status": 2u64 })),
        )
        .await;
    assert_success(&setup.process(&two, &transaction));

    // Once its time to live has passed, nobody writes it any more.
    let expired_at = BLOCK_TIME_MS + 3_600_000 + 1;
    let after_expiry = setup
        .moderate(
            &setup.moderator,
            change_action(NOTICE, notice.id(), platform_value!({ "status": 1u64 })),
        )
        .await;
    assert_paid_with_code(
        &setup.process_at(&after_expiry, expired_at, &transaction),
        DOCUMENT_EXPIRED,
    );
}

#[tokio::test]
async fn should_change_and_delete_documents_of_a_contract_that_can_not_be_deleted() {
    // Yappr's shape: the contract itself can not be deleted and its reports are immutable, so
    // their index entries are stored without storage flags. A moderator's change moves the
    // report's `byStatus` entry twice, and its deletion removes what the changes wrote.
    let setup = setup_where(|config| config.set_can_be_deleted(false)).await;
    assert!(!setup.contract.config().can_be_deleted());
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (report, create_report) =
        create_report(&setup, &setup.stranger, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create_report, &transaction));
    setup.commit(transaction);

    for status in [2u64, 3] {
        let transaction = setup.platform.drive.grove.start_transaction();
        let change = setup
            .moderate(
                &setup.moderator,
                change_action(REPORT, report.id(), platform_value!({ "status": status })),
            )
            .await;
        assert_success(&setup.process(&change, &transaction));
        setup.commit(transaction);
        assert!(setup
            .assert_change_proved(&change)
            .get("status")
            .is_some_and(|stored| stored.equal_underlying_data(&Value::U64(status))));
    }

    let transaction = setup.platform.drive.grove.start_transaction();
    let delete = setup
        .moderate(&setup.moderator, delete_action(REPORT, report.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);
    assert_eq!(setup.stored_document(REPORT, report.id(), None), None);
}

#[tokio::test]
async fn should_stamp_the_moderator_who_last_wrote_the_fields() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (report, create) =
        create_report(&setup, &setup.stranger, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create, &transaction));
    let (untouched, create_untouched) =
        create_report(&setup, &setup.stranger, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create_untouched, &transaction));
    // A report nobody moderated carries no stamp.
    let filed = setup
        .stored_document(REPORT, report.id(), Some(&transaction))
        .expect("expected the report");
    assert_eq!(filed.moderated_at(), None);
    assert_eq!(filed.moderated_by(), None);
    setup.commit(transaction);

    let handled_at = BLOCK_TIME_MS + 5_000;
    let transaction = setup.platform.drive.grove.start_transaction();
    let handle = setup
        .moderate(
            &setup.moderator,
            change_action(REPORT, report.id(), platform_value!({ "status": 2u64 })),
        )
        .await;
    assert_success(&setup.process_at(&handle, handled_at, &transaction));
    let handled = setup
        .stored_document(REPORT, report.id(), Some(&transaction))
        .expect("expected the report");
    assert_eq!(handled.moderated_at(), Some(handled_at));
    assert_eq!(handled.moderated_by(), Some(setup.moderator.id()));
    // Its own clocks stay as its author left them.
    assert_eq!(handled.updated_at(), filed.updated_at());
    setup.commit(transaction);
    let proved = setup.assert_change_proved(&handle);
    assert_eq!(proved.moderated_by(), Some(setup.moderator.id()));
    assert_eq!(proved.moderated_at(), Some(handled_at));

    // The reports are queried by who handled them and when; the one nobody touched is in
    // neither.
    let moderator_id = Value::Identifier(setup.moderator.id().to_buffer());
    assert_eq!(
        setup.query_reports(
            platform_value!({
                "where": [["$moderatedBy", "==", moderator_id.clone()]],
                "orderBy": [["$moderatedAt", "asc"]],
            }),
            None,
        ),
        vec![report.id()]
    );
    assert_eq!(
        setup.query_reports(
            platform_value!({
                "where": [["$moderatedAt", ">", BLOCK_TIME_MS]],
                "orderBy": [["$moderatedAt", "asc"]],
            }),
            None,
        ),
        vec![report.id()]
    );

    // The last moderator to write the fields is the one stamped: the contract owner, later,
    // takes over the stamp and the entries of both indexes.
    let resolved_at = BLOCK_TIME_MS + 9_000;
    let transaction = setup.platform.drive.grove.start_transaction();
    let resolve = setup
        .moderate(
            &setup.owner,
            change_action(
                REPORT,
                report.id(),
                platform_value!({ "resolution": "the post was removed" }),
            ),
        )
        .await;
    assert_success(&setup.process_at(&resolve, resolved_at, &transaction));
    setup.commit(transaction);
    let resolved = setup
        .stored_document(REPORT, report.id(), None)
        .expect("expected the report");
    assert_eq!(resolved.moderated_at(), Some(resolved_at));
    assert_eq!(resolved.moderated_by(), Some(setup.owner.id()));
    assert!(setup
        .query_reports(
            platform_value!({
                "where": [["$moderatedBy", "==", moderator_id]],
                "orderBy": [["$moderatedAt", "asc"]],
            }),
            None,
        )
        .is_empty());
    assert_eq!(
        setup.query_reports(
            platform_value!({
                "where": [["$moderatedBy", "==", Value::Identifier(setup.owner.id().to_buffer())]],
                "orderBy": [["$moderatedAt", "asc"]],
            }),
            None,
        ),
        vec![report.id()]
    );
    assert_eq!(
        setup.query_reports(
            platform_value!({
                "where": [["$moderatedAt", ">", handled_at]],
                "orderBy": [["$moderatedAt", "asc"]],
            }),
            None,
        ),
        vec![report.id()]
    );
    // The report nobody moderated still carries no stamp.
    let untouched = setup
        .stored_document(REPORT, untouched.id(), None)
        .expect("expected the untouched report");
    assert_eq!(untouched.moderated_at(), None);
    assert_eq!(untouched.moderated_by(), None);
}

#[tokio::test]
async fn should_stamp_a_moderator_who_writes_the_fields_in_their_own_documents() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));

    // A moderator filing a report already marked handled is stamped as its moderator, at the
    // block's time.
    let (report, create) = create_report(
        &setup,
        &setup.moderator,
        post.id(),
        platform_value!({ "status": 1u64 }),
    )
    .await;
    assert_success(&setup.process_at(&create, BLOCK_TIME_MS + 1_000, &transaction));
    let filed = setup
        .stored_document(REPORT, report.id(), Some(&transaction))
        .expect("expected the report");
    assert_eq!(filed.moderated_at(), Some(BLOCK_TIME_MS + 1_000));
    assert_eq!(filed.moderated_by(), Some(setup.moderator.id()));
    // One filed without the moderators' fields is not, even by a moderator.
    let (plain, create_plain) =
        create_report(&setup, &setup.moderator, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create_plain, &transaction));
    let plain = setup
        .stored_document(REPORT, plain.id(), Some(&transaction))
        .expect("expected the report");
    assert_eq!(plain.moderated_by(), None);

    // A ticket's author who does not moderate keeps the stamp a moderator's change left when
    // replacing the rest.
    let (ticket, create_the_ticket) =
        create_ticket(&setup, &setup.user, "printer", platform_value!({})).await;
    assert_success(&setup.process(&create_the_ticket, &transaction));
    let assign = setup
        .moderate(
            &setup.moderator,
            change_action(TICKET, ticket.id(), platform_value!({ "slot": 4u64 })),
        )
        .await;
    assert_success(&setup.process_at(&assign, BLOCK_TIME_MS + 2_000, &transaction));
    let mut renamed = setup
        .stored_document(TICKET, ticket.id(), Some(&transaction))
        .expect("expected the ticket");
    renamed.set("title", Value::Text("scanner".to_string()));
    renamed.increment_revision().expect("expected a revision");
    let replace = replace_ticket(&setup, &setup.user, renamed).await;
    assert_success(&setup.process_at(&replace, BLOCK_TIME_MS + 3_000, &transaction));
    let replaced = setup
        .stored_document(TICKET, ticket.id(), Some(&transaction))
        .expect("expected the ticket");
    assert_eq!(
        replaced.get("title"),
        Some(&Value::Text("scanner".to_string()))
    );
    assert_eq!(replaced.moderated_at(), Some(BLOCK_TIME_MS + 2_000));
    assert_eq!(replaced.moderated_by(), Some(setup.moderator.id()));

    // A moderator replacing their own ticket's slot is stamped; replacing only its title
    // leaves the stamp as it was.
    let (own, create_own) =
        create_ticket(&setup, &setup.moderator, "desk", platform_value!({})).await;
    assert_success(&setup.process(&create_own, &transaction));
    let mut reslotted = setup
        .stored_document(TICKET, own.id(), Some(&transaction))
        .expect("expected the ticket");
    reslotted.set("slot", Value::U64(7));
    reslotted.increment_revision().expect("expected a revision");
    let replace = replace_ticket(&setup, &setup.moderator, reslotted).await;
    assert_success(&setup.process_at(&replace, BLOCK_TIME_MS + 4_000, &transaction));
    let mut retitled = setup
        .stored_document(TICKET, own.id(), Some(&transaction))
        .expect("expected the ticket");
    assert_eq!(retitled.moderated_at(), Some(BLOCK_TIME_MS + 4_000));
    assert_eq!(retitled.moderated_by(), Some(setup.moderator.id()));
    retitled.set("title", Value::Text("chair".to_string()));
    retitled.increment_revision().expect("expected a revision");
    let replace = replace_ticket(&setup, &setup.moderator, retitled).await;
    assert_success(&setup.process_at(&replace, BLOCK_TIME_MS + 6_000, &transaction));
    let kept = setup
        .stored_document(TICKET, own.id(), Some(&transaction))
        .expect("expected the ticket");
    assert_eq!(kept.moderated_at(), Some(BLOCK_TIME_MS + 4_000));
}

#[tokio::test]
async fn should_keep_the_stamp_through_a_transfer_and_a_restore() {
    let setup = setup().await;
    let platform_version = PlatformVersion::latest();
    let stamped_at = BLOCK_TIME_MS + 2_000;

    // A card a moderator graded, given away by its owner: the new owner holds it with the
    // stamp as the moderator left it.
    let transaction = setup.platform.drive.grove.start_transaction();
    let (card, create_card) = setup
        .create_document_of_type_with(&setup.user, CARD, |document| {
            let properties = document.properties_mut();
            properties.clear();
            properties.insert("name".to_string(), Value::Text("ace".to_string()));
        })
        .await;
    assert_success(&setup.process(&create_card, &transaction));
    let grade = setup
        .moderate(
            &setup.moderator,
            change_action(CARD, card.id(), platform_value!({ "grade": 7u64 })),
        )
        .await;
    assert_success(&setup.process_at(&grade, stamped_at, &transaction));
    let mut graded = setup
        .stored_document(CARD, card.id(), Some(&transaction))
        .expect("expected the card");
    graded.increment_revision().expect("expected a revision");
    let transfer = BatchTransition::new_document_transfer_transition_from_document(
        graded,
        setup
            .contract
            .document_type_for_name(CARD)
            .expect("expected the card type"),
        setup.stranger.id(),
        &setup.user.key,
        setup.user.contract_nonce(),
        0,
        None,
        &setup.user.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the transfer");
    assert_success(&setup.process_at(&transfer, BLOCK_TIME_MS + 3_000, &transaction));
    let given = setup
        .stored_document(CARD, card.id(), Some(&transaction))
        .expect("expected the card");
    assert_eq!(given.owner_id(), setup.stranger.id());
    assert_eq!(given.moderated_at(), Some(stamped_at));
    assert_eq!(given.moderated_by(), Some(setup.moderator.id()));

    // A handled report a moderator deletes and the contract owner restores comes back with the
    // stamp, which its bytes carry.
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (report, create) =
        create_report(&setup, &setup.stranger, post.id(), platform_value!({})).await;
    assert_success(&setup.process(&create, &transaction));
    let handle = setup
        .moderate(
            &setup.moderator,
            change_action(REPORT, report.id(), platform_value!({ "status": 2u64 })),
        )
        .await;
    assert_success(&setup.process_at(&handle, stamped_at, &transaction));
    setup.commit(transaction);
    let handled = setup
        .stored_document(REPORT, report.id(), None)
        .expect("expected the report");
    assert_eq!(handled.moderated_by(), Some(setup.moderator.id()));
    let bytes = setup.document_bytes(REPORT, &handled);

    let transaction = setup.platform.drive.grove.start_transaction();
    let delete = setup
        .moderate(&setup.moderator, delete_action(REPORT, report.id()))
        .await;
    assert_success(&setup.process_at(&delete, BLOCK_TIME_MS + 4_000, &transaction));
    let restore = setup
        .moderate(&setup.owner, restore_action(REPORT, bytes))
        .await;
    assert_success(&setup.process_at(&restore, BLOCK_TIME_MS + 5_000, &transaction));
    setup.commit(transaction);
    assert_eq!(
        setup.stored_document(REPORT, report.id(), None),
        Some(handled)
    );
}

#[tokio::test]
async fn should_prove_a_whole_number_set_on_a_number_field() {
    // The transition carries the integer a client sends for a whole number; the document stores
    // the `number` property as a float. The proof still shows the change.
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (ticket, create) = create_ticket(&setup, &setup.user, "printer", platform_value!({})).await;
    assert_success(&setup.process(&create, &transaction));
    let weigh = setup
        .moderate(
            &setup.moderator,
            change_action(TICKET, ticket.id(), platform_value!({ "weight": 2u64 })),
        )
        .await;
    assert_success(&setup.process(&weigh, &transaction));
    setup.commit(transaction);
    let proved = setup.assert_change_proved(&weigh);
    assert_eq!(proved.get("weight"), Some(&Value::Float(2.0)));
}
