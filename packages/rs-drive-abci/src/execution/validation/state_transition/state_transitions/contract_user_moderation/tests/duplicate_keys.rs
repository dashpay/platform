use super::*;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;

#[tokio::test]
async fn should_admit_duplicate_moderator_fields_to_mempool_then_reject_them_paid_in_block() {
    let version = PlatformVersion::latest();
    let setup = Setup::new_at_with(Some(moderation(false, false, THE_MODERATOR)), version, |contract| {
        add_document_type(contract, "nested", platform_value!({
            "type": "object", "moderatorAbilities": {"changeFields": ["meta"]},
            "properties": {
                "title": {"type": "string", "maxLength": 32, "position": 0},
                "meta": {"type": "object", "position": 1,
                    "properties": {"name": {"type": "string", "maxLength": 32, "position": 0}},
                    "required": ["name"], "additionalProperties": false}
            },
            "required": ["title"], "additionalProperties": false
        }));
    }).await;
    let (document, creation) = setup
        .create_document_of_type_with(&setup.user, "nested", |document| {
            document.properties_mut().clear();
            document.set("title", "original".into());
        })
        .await;
    let tx = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&creation, &tx));
    setup.commit(tx);
    let before = setup
        .stored_document("nested", document.id(), None)
        .unwrap();
    let nonce = setup.moderator.next_contract_nonce.get();
    let moderation = setup
        .moderate(
            &setup.moderator,
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name: "nested".into(),
                document_id: document.id(),
                fields: BTreeMap::from([(
                    "meta".into(),
                    Value::Map(vec![
                        ("name".into(), "first".into()),
                        ("name".into(), "last".into()),
                    ]),
                )]),
                reason: ContractModerationReason::from_text("handled"),
            },
        )
        .await;
    let raw = moderation.serialize_to_bytes().unwrap();
    assert_eq!(
        StateTransition::deserialize_from_bytes_untrusted_exact_in_version(&raw, version).unwrap(),
        moderation
    );
    assert!(
        setup.check_tx(&moderation).is_empty(),
        "moderation runs property validation in the block"
    );
    let balance = setup.balance(setup.moderator.id(), None);
    let tx = setup.platform.drive.grove.start_transaction();
    let result = setup.process(&moderation, &tx);
    assert_paid_with_code(&result, 10103);
    let StateTransitionExecutionResult::PaidConsensusError { actual_fees, .. } = result else {
        unreachable!()
    };
    assert!(actual_fees.total_base_fee() > 0);
    assert_eq!(
        setup.balance(setup.moderator.id(), Some(&tx)),
        balance - actual_fees.total_base_fee()
    );
    let consumed = setup
        .platform
        .drive
        .fetch_identity_contract_nonce(
            setup.moderator.id().to_buffer(),
            setup.contract.id().to_buffer(),
            true,
            Some(&tx),
            version,
        )
        .unwrap()
        .unwrap();
    assert_eq!(consumed & IDENTITY_NONCE_VALUE_FILTER, nonce);
    assert_eq!(
        setup
            .stored_document("nested", document.id(), Some(&tx))
            .unwrap(),
        before
    );
    setup.commit(tx);
}
