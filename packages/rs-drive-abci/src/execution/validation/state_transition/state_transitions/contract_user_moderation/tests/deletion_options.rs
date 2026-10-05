//! What a moderator's deletion leaves, as the document type says (protocol version 14): a
//! removal record unless `moderatorAbilities.deleteKeepsRecord` is false, keeping the values
//! `moderatorAbilities.deleteKeepsFields` lists, and the owner's storage refund only when
//! `moderatorAbilities.deleteRefundsOwner` is true.

use super::*;
use drive::drive::contract::paths::contract_document_removals_path;
use drive::error::proof::ProofError;
use drive::error::Error as DriveError;
use drive::util::grove_operations::DirectQueryType;

/// A contract whose moderators delete posts with `abilities`
async fn setup_with(abilities: Value) -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({ "moderatorAbilities": abilities })),
            )
        },
    )
    .await
}

impl Setup {
    /// The values `removal` keeps, read under the post type, as the post's are
    fn kept_values(&self, removal: &ContractDocumentRemoval) -> BTreeMap<String, Value> {
        removal
            .kept_values(
                self.contract
                    .document_type_for_name(POST)
                    .expect("expected the post type"),
            )
            .expect("expected the kept values to read under the post type")
    }

    /// Whether the contract has a removal records tree at all
    fn has_removals_tree(&self) -> bool {
        let contract_id = self.contract.id().to_buffer();
        let path = contract_document_removals_path(&contract_id);
        self.platform
            .drive
            .grove_get_raw_optional(
                path[..3].as_ref().into(),
                path[3],
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to read the contract's other tree")
            .is_some()
    }
}

#[tokio::test]
async fn should_delete_without_a_record_and_leave_nothing_to_restore() {
    let setup = setup_with(platform_value!({ "delete": true, "deleteKeepsRecord": false })).await;
    // No type of the contract keeps records, so it has no records tree.
    assert!(!setup.has_removals_tree());

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    setup.commit(transaction);
    let author_balance = setup.balance(setup.user.id(), None);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert!(setup.check_tx(&delete).is_empty());
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);
    assert_eq!(setup.stored_document(POST, post.id(), None), None);
    assert!(!setup.has_removals_tree());
    // The owner still forfeits the refund: that is the other option.
    assert_eq!(setup.balance(setup.user.id(), None), author_balance);

    // The proof shows the document gone, which is all there is to show.
    let platform_version = PlatformVersion::latest();
    let proof = setup
        .platform
        .drive
        .prove_state_transition(&delete, None, platform_version)
        .expect("expected to prove the deletion")
        .into_data()
        .expect("expected proof bytes");
    let known_contracts: BTreeMap<Identifier, DataContract> =
        BTreeMap::from([(setup.contract.id(), setup.contract.clone())]);
    let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
        &delete,
        &BlockInfo::default(),
        &proof,
        &|id| Ok(known_contracts.get(id).cloned().map(std::sync::Arc::new)),
        platform_version,
    )
    .expect("expected the proof to verify");
    match outcome.into_result() {
        StateTransitionProofResult::VerifiedDocuments(documents) => {
            assert_eq!(documents, BTreeMap::from([(post.id(), None)]));
        }
        other => panic!("expected the document gone, got {other:?}"),
    }
    // Without the contract no record proves it and the absence can not be read: the verifier
    // says the contract is missing, and passes on a lookup that failed, rather than calling the
    // proof wrong.
    let unknown = Drive::verify_state_transition_was_executed_with_proof(
        &delete,
        &BlockInfo::default(),
        &proof,
        &|_| Ok(None),
        platform_version,
    )
    .expect_err("a deletion of a type without records needs the contract");
    assert!(
        matches!(unknown, DriveError::Proof(ProofError::UnknownContract(_))),
        "expected the contract reported unknown, got {unknown:?}"
    );
    let failed = Drive::verify_state_transition_was_executed_with_proof(
        &delete,
        &BlockInfo::default(),
        &proof,
        &|_| {
            Err(DriveError::Proof(ProofError::ErrorRetrievingContract(
                "the provider is offline".to_string(),
            )))
        },
        platform_version,
    )
    .expect_err("a failed lookup leaves the absence unread");
    assert!(
        matches!(
            failed,
            DriveError::Proof(ProofError::ErrorRetrievingContract(_))
        ),
        "expected the lookup's failure, got {failed:?}"
    );

    // Without a record there is nothing to restore from.
    let transaction = setup.platform.drive.grove.start_transaction();
    let restore = setup
        .moderate(
            &setup.moderator,
            restore_action(POST, setup.document_bytes(POST, &stored)),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&restore, &transaction),
        CONTRACT_DOCUMENT_REMOVAL_NOT_FOUND,
    );
}

#[tokio::test]
async fn should_refund_the_owner_when_the_type_says_so() {
    let setup = setup_with(platform_value!({ "delete": true, "deleteRefundsOwner": true })).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    setup.commit(transaction);
    let author_balance = setup.balance(user_id, None);
    let moderator_balance = setup.balance(setup.moderator.id(), None);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);

    // The author is refunded, as for their own deletion, and pays nothing; the moderator pays
    // for the transition and the record.
    assert!(
        setup.balance(user_id, None) > author_balance,
        "the author is refunded the post's storage"
    );
    assert!(setup.balance(setup.moderator.id(), None) < moderator_balance);
    // The record is kept, the default: the two options are independent.
    let removal = setup
        .post_removal(post.id(), None)
        .expect("expected the removal record");
    assert_eq!(removal.document_owner_id, user_id);
    assert_eq!(setup.assert_removal_proved(&delete), removal);
}

#[tokio::test]
async fn should_refund_without_a_record_too() {
    let setup = setup_with(platform_value!({
        "delete": true,
        "deleteKeepsRecord": false,
        "deleteRefundsOwner": true,
    }))
    .await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    setup.commit(transaction);
    let author_balance = setup.balance(user_id, None);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);
    assert!(setup.balance(user_id, None) > author_balance);
    assert!(!setup.has_removals_tree());
}

#[tokio::test]
async fn should_keep_what_a_deletion_leaves_fixed_with_the_type() {
    let setup = setup_with(platform_value!({ "delete": true })).await;
    assert!(setup.has_removals_tree());

    for abilities in [
        platform_value!({ "delete": true, "deleteKeepsRecord": false }),
        platform_value!({ "delete": true, "deleteRefundsOwner": true }),
    ] {
        let transaction = setup.platform.drive.grove.start_transaction();
        let mut changed = setup.contract.clone();
        changed.increment_version();
        let mut schema = changed
            .document_type_for_name(POST)
            .expect("expected the post type")
            .schema()
            .clone();
        schema
            .insert("moderatorAbilities".to_string(), abilities)
            .expect("expected to change the keyword");
        add_document_type(&mut changed, POST, schema);
        let update = setup.contract_update(changed).await;
        assert_paid_with_code(&setup.process(&update, &transaction), DOCUMENT_TYPE_UPDATE);
    }
}

/// A contract whose moderators delete posts carrying a hashtag and an object of tags and a
/// note, their records keeping `kept` (`moderatorAbilities.deleteKeepsFields`)
async fn setup_keeping(kept: Value) -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                        "hashtag": { "type": "string", "maxLength": 61, "position": 1 },
                        "meta": {
                            "type": "object",
                            "position": 2,
                            "properties": {
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string", "maxLength": 20 },
                                    "maxItems": 5,
                                    "position": 0,
                                },
                                "note": { "type": "string", "maxLength": 20, "position": 1 },
                            },
                            "additionalProperties": false,
                        },
                    },
                    "required": ["text", "$createdAt"],
                    "moderatorAbilities": { "delete": true, "deleteKeepsFields": kept },
                })),
            )
        },
    )
    .await
}

#[tokio::test]
async fn should_keep_the_listed_fields_of_a_deleted_document_in_its_record() {
    let setup = setup_keeping(platform_value!(["hashtag", "meta.tags", "$createdAt"])).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("hashtag", platform_value!("dash"));
            document.set(
                "meta",
                platform_value!({ "tags": ["privacy", "payments"], "note": "not kept" }),
            );
        })
        .await;
    assert_success(&setup.process(&create, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    setup.commit(transaction);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert!(setup.check_tx(&delete).is_empty());
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);
    assert_eq!(setup.stored_document(POST, post.id(), None), None);

    // The document is gone; what its type keeps public is in the record, copied as it was
    // stored and read back under the type as the document's values are, nested values and the
    // creation time included. The note, not listed, went with the document.
    let removal = setup
        .post_removal(post.id(), None)
        .expect("expected the removal record");
    assert_eq!(
        setup.kept_values(&removal),
        BTreeMap::from([
            (
                "$createdAt".to_string(),
                Value::U64(stored.created_at().expect("expected a creation time")),
            ),
            ("hashtag".to_string(), platform_value!("dash")),
            (
                "meta.tags".to_string(),
                platform_value!(["privacy", "payments"])
            ),
        ])
    );
    // The proof of the deletion shows the same record, kept values included.
    assert_eq!(setup.assert_removal_proved(&delete), removal);

    // A restore brings the document back as it was; the record, marked restored, keeps what it
    // kept.
    let restore = setup
        .moderate(
            &setup.moderator,
            restore_action(POST, setup.document_bytes(POST, &stored)),
        )
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&restore, &transaction));
    setup.commit(transaction);
    let restored = setup
        .post_removal(post.id(), None)
        .expect("expected the removal record");
    assert!(restored.is_restored());
    assert_eq!(restored.kept_fields, removal.kept_fields);
    assert_eq!(setup.assert_removal_proved(&restore), restored);

    // The record now says the deletion was undone, so it no longer proves the deletion
    let platform_version = PlatformVersion::latest();
    let proof = setup
        .platform
        .drive
        .prove_state_transition(&delete, None, platform_version)
        .expect("expected to prove the deletion's record")
        .into_data()
        .expect("expected proof bytes");
    let known_contracts: BTreeMap<Identifier, DataContract> =
        BTreeMap::from([(setup.contract.id(), setup.contract.clone())]);
    Drive::verify_state_transition_was_executed_with_proof(
        &delete,
        &BlockInfo::default(),
        &proof,
        &|id| Ok(known_contracts.get(id).cloned().map(std::sync::Arc::new)),
        platform_version,
    )
    .expect_err("a restored record does not prove the deletion");
}

#[tokio::test]
async fn should_leave_out_of_the_record_a_path_the_document_holds_no_value_at() {
    let setup = setup_keeping(platform_value!(["hashtag", "meta.tags", "$createdAt"])).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.properties_mut().remove("hashtag");
            document.set("meta", platform_value!({ "note": "no tags" }));
        })
        .await;
    assert_success(&setup.process(&create, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    setup.commit(transaction);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);

    let removal = setup
        .post_removal(post.id(), None)
        .expect("expected the removal record");
    assert_eq!(
        setup.kept_values(&removal),
        BTreeMap::from([(
            "$createdAt".to_string(),
            Value::U64(stored.created_at().expect("expected a creation time")),
        )])
    );
    assert_eq!(setup.assert_removal_proved(&delete), removal);
}

#[tokio::test]
async fn should_keep_the_fields_a_record_keeps_fixed_with_the_type() {
    let setup = setup_keeping(platform_value!(["hashtag"])).await;

    for kept in [
        platform_value!(["hashtag", "meta.tags"]),
        platform_value!(["text"]),
    ] {
        let transaction = setup.platform.drive.grove.start_transaction();
        let mut changed = setup.contract.clone();
        changed.increment_version();
        let mut schema = changed
            .document_type_for_name(POST)
            .expect("expected the post type")
            .schema()
            .clone();
        schema
            .insert(
                "moderatorAbilities".to_string(),
                platform_value!({ "delete": true, "deleteKeepsFields": kept }),
            )
            .expect("expected to change the keyword");
        add_document_type(&mut changed, POST, schema);
        let update = setup.contract_update(changed).await;
        assert_paid_with_code(&setup.process(&update, &transaction), DOCUMENT_TYPE_UPDATE);
    }
}
