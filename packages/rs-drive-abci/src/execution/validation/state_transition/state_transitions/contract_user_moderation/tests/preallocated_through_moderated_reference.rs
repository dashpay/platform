//! Preallocated indexes through a `moderatedDocument` reference (protocol version 14): a
//! moderated post whose removal record keeps its text preallocates the trees of the likes
//! keyed by it. A moderator's removal leaves them like its record, and the restore puts the
//! post back onto them. A preallocated index keyed by a field the record drops is refused.

use super::*;
use drive::drive::RootTree;

const LIKE: &str = "like";

/// A like of a post, keyed by the post's text and id through `referenced`, a moderated
/// post's property the like agrees with
fn like_schema(agreed: &str) -> Value {
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "postId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 0,
                "refersTo": {
                    "type": "moderatedDocument",
                    "documentType": POST,
                    "where": { agreed: agreed },
                },
            },
            agreed: { "type": "string", "maxLength": 50, "position": 1 },
        },
        "indices": [
            {
                "name": "byPost",
                "properties": [{ "postId": "asc" }],
                "terminal": "$ownerId",
                "countable": "countable",
                "preallocated": true,
            },
            {
                "name": "byAgreedPost",
                "properties": [{ agreed: "asc" }, { "postId": "asc" }],
                "terminal": "$ownerId",
                "preallocated": true,
            },
        ],
        "required": ["postId", agreed],
        "additionalProperties": false,
    })
}

/// Posts nobody changes, only the moderators take down, each removal record keeping the post's
/// text and not its topic, and likes keyed by the text
async fn setup() -> Setup {
    Setup::new_at_with(
        Some(moderation(false, false, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "documentsMutable": false,
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                        "topic": { "type": "string", "maxLength": 50, "position": 1 },
                    },
                    "required": ["text", "topic"],
                    "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["text"] },
                })),
            );
            add_document_type(contract, LIKE, like_schema("text"));
        },
    )
    .await
}

impl Setup {
    /// Whether the empty member bucket of `path` below the like type's tree exists
    fn like_bucket_exists(&self, path: &[&[u8]], transaction: &Transaction) -> bool {
        let mut full_path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract.id().to_vec(),
            vec![1u8],
            LIKE.as_bytes().to_vec(),
        ];
        full_path.extend(path.iter().map(|key| key.to_vec()));
        let path_refs: Vec<&[u8]> = full_path.iter().map(Vec::as_slice).collect();
        self.platform
            .drive
            .grove
            .get(
                path_refs.as_slice(),
                &[0],
                Some(transaction),
                &PlatformVersion::latest().drive.grove_version,
            )
            .unwrap()
            .is_ok()
    }
}

#[tokio::test]
async fn should_preallocate_like_trees_for_a_moderated_post_through_removal_and_restore() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("text", "hello".into());
        })
        .await;
    assert_success(&setup.process(&create_post, &transaction));
    let post_id = post.id().to_buffer();
    let text = Value::Text("hello".to_string());
    let text_key = b"hello".to_vec();
    let assert_preallocated = |when: &str, transaction: &Transaction| {
        assert!(
            setup.like_bucket_exists(&[b"postId", &post_id], transaction),
            "the byPost bucket must be there {when}"
        );
        assert!(
            setup.like_bucket_exists(&[b"text", &text_key, b"postId", &post_id], transaction),
            "the byAgreedPost bucket must be there {when}"
        );
    };
    assert_preallocated("once the post is created", &transaction);

    let (_, like) = setup
        .create_document_of_type_with(&setup.stranger, LIKE, |document| {
            document.set("postId", Value::Identifier(post.id().to_buffer()));
            document.set("text", text.clone());
        })
        .await;
    assert_success(&setup.process(&like, &transaction));

    let stored_post = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    assert!(setup
        .stored_document(POST, post.id(), Some(&transaction))
        .is_none());
    assert_preallocated("after a moderator removed the post", &transaction);

    let restore = setup
        .moderate(
            &setup.moderator,
            restore_action(POST, setup.document_bytes(POST, &stored_post)),
        )
        .await;
    assert_success(&setup.process(&restore, &transaction));
    assert_preallocated("after the post is restored", &transaction);

    let (_, like_again) = setup
        .create_document_of_type_with(&setup.user, LIKE, |document| {
            document.set("postId", Value::Identifier(post.id().to_buffer()));
            document.set("text", text.clone());
        })
        .await;
    assert_success(&setup.process(&like_again, &transaction));
}

#[tokio::test]
async fn should_refuse_an_update_preallocating_through_a_field_the_record_drops() {
    let setup = setup().await;
    let mut contract = setup.contract.clone();
    contract.increment_version();
    // Added without the client's checks, so the platform judges it
    contract
        .set_document_schema(
            "topicLike",
            like_schema("topic"),
            false,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .expect("expected to add the type");
    let update = setup.contract_update(contract).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let result = setup.process(&update, &transaction);
    let refusal = match &result {
        StateTransitionExecutionResult::PaidConsensusError { error, .. }
        | StateTransitionExecutionResult::UnpaidConsensusError(error) => error,
        other => panic!("expected the update to be refused, got {other:?}"),
    };
    assert_eq!(refusal.code(), 10231, "{refusal}");
    assert!(
        refusal
            .to_string()
            .contains("\"topic\" of \"post\" is not kept by a moderator's removal"),
        "{refusal}"
    );
}
