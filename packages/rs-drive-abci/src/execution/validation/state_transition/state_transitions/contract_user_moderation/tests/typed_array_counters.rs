//! Counters per tag keyed through a typed-array `where` (protocol version 14), through the
//! whole pipeline: Yappr's like makes its `tags` equal to its post's, so creating a post
//! preallocates one counter per tag (`byTagPost`), every like of the post moves each of them,
//! and a like whose tags differ from the post's is refused. A moderator's removal of the post
//! keeps the counters, an unlike still moves them, and the restore puts the post back onto
//! them.

use super::*;
use dpp::tests::json_document::json_document_to_json_value;
use drive::drive::RootTree;
use drive::grovedb::Element;
use drive::util::grove_operations::DirectQueryType;

const LIKE: &str = "like";

/// Yappr's post and like, shared with rs-drive's suite.
const TAG_COUNTERS_CONTRACT: &str =
    "../rs-drive/tests/supporting_files/contract/yappr-likes/yappr-likes-tags-counters-contract.json";

/// The schema of the fixture's document type `name`
fn fixture_schema(name: &str) -> Value {
    let contract =
        json_document_to_json_value(TAG_COUNTERS_CONTRACT).expect("expected to read the fixture");
    contract["documentSchemas"][name].clone().into()
}

/// The fixture's post and like, on a contract whose appointed moderator takes posts down
async fn setup() -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(contract, POST, fixture_schema(POST));
            add_document_type(contract, LIKE, fixture_schema(LIKE));
        },
    )
    .await
}

impl Setup {
    /// The counter `byTagPost` keeps for `post_id` under `tag`, if there is one
    fn tag_counter(
        &self,
        tag: &str,
        post_id: Identifier,
        transaction: &Transaction,
    ) -> Option<i64> {
        let path: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract.id().to_vec(),
            vec![1u8],
            LIKE.as_bytes().to_vec(),
            b"tags".to_vec(),
            tag.as_bytes().to_vec(),
            b"postId".to_vec(),
        ];
        let path_refs: Vec<&[u8]> = path.iter().map(Vec::as_slice).collect();
        match self
            .platform
            .drive
            .grove_get_raw_optional(
                path_refs.as_slice().into(),
                post_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                Some(transaction),
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to read the counter")
        {
            Some(Element::SumItem(count, _)) => Some(count),
            Some(other) => panic!("expected a counter under {tag}, got {other:?}"),
            None => None,
        }
    }

    /// `actor`'s like of the post `post_id` by `author`, carrying `tags` (none when `None`)
    async fn like(
        &self,
        actor: &Actor,
        post_id: Identifier,
        author: Identifier,
        tags: Option<&[&str]>,
    ) -> (Document, StateTransition) {
        let mut properties = vec![
            ("postId", Value::Identifier(post_id.to_buffer())),
            ("postAuthor", Value::Identifier(author.to_buffer())),
        ];
        if let Some(tags) = tags {
            properties.push(("tags", Value::from(tags)));
        }
        create_with(self, actor, LIKE, properties).await
    }

    /// The deletion of `like` by `actor`, its owner, carrying its values
    async fn unlike(&self, actor: &Actor, like: Document) -> StateTransition {
        BatchTransition::new_document_deletion_transition_from_document(
            like,
            self.contract
                .document_type_for_name(LIKE)
                .expect("expected the like type"),
            &actor.key,
            actor.contract_nonce(),
            0,
            None,
            &actor.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("expected to build the unlike")
    }
}

/// The like was refused because its `tags` are not its post's (code 40127)
fn assert_tags_mismatch(execution: &StateTransitionExecutionResult) {
    assert!(
        matches!(
            execution,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(mismatch)
                ),
                ..
            } if mismatch.referring_property() == "tags"
        ),
        "expected the tags to be refused, got {execution:?}"
    );
}

#[tokio::test]
async fn should_count_likes_per_tag_of_a_post_through_removal_and_restore() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let author = setup.user.id();
    let (post, create_post) = create_with(
        &setup,
        &setup.user,
        POST,
        vec![
            ("content", Value::Text("hello".to_string())),
            ("tags", Value::from(&["dash", "news"][..])),
        ],
    )
    .await;
    assert_success(&setup.process(&create_post, &transaction));
    let (untagged, create_untagged) = create_with(
        &setup,
        &setup.user,
        POST,
        vec![("content", Value::Text("plain".to_string()))],
    )
    .await;
    assert_success(&setup.process(&create_untagged, &transaction));
    let counters = |transaction: &Transaction| {
        ["dash", "news"].map(|tag| setup.tag_counter(tag, post.id(), transaction))
    };
    assert_eq!(counters(&transaction), [Some(0), Some(0)], "preallocated");
    assert_eq!(setup.tag_counter("dash", untagged.id(), &transaction), None);

    // Only the post's tags, in its order, are its tags
    for tags in [Some(&["news", "dash"][..]), Some(&["dash"][..]), None] {
        let (_, refused) = setup.like(&setup.stranger, post.id(), author, tags).await;
        assert_tags_mismatch(&setup.process(&refused, &transaction));
    }
    let (_, refused) = setup
        .like(&setup.stranger, untagged.id(), author, Some(&["dash"]))
        .await;
    assert_tags_mismatch(&setup.process(&refused, &transaction));
    assert_eq!(counters(&transaction), [Some(0), Some(0)]);

    let (stranger_like, like) = setup
        .like(&setup.stranger, post.id(), author, Some(&["dash", "news"]))
        .await;
    assert_success(&setup.process(&like, &transaction));
    let (_, like) = setup
        .like(&setup.moderator, post.id(), author, Some(&["dash", "news"]))
        .await;
    assert_success(&setup.process(&like, &transaction));
    assert_eq!(counters(&transaction), [Some(2), Some(2)]);
    let (_, untagged_like) = setup
        .like(&setup.stranger, untagged.id(), author, None)
        .await;
    assert_success(&setup.process(&untagged_like, &transaction));
    assert_eq!(setup.tag_counter("dash", untagged.id(), &transaction), None);

    // The moderator removes the post: its counters stay, and an unlike still moves them
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
    assert_eq!(counters(&transaction), [Some(2), Some(2)]);
    let unlike = setup.unlike(&setup.stranger, stranger_like).await;
    assert_success(&setup.process(&unlike, &transaction));
    assert_eq!(counters(&transaction), [Some(1), Some(1)]);

    // Restored as it was removed, the post is back on its counters
    let restore = setup
        .moderate(
            &setup.moderator,
            restore_action(POST, setup.document_bytes(POST, &stored_post)),
        )
        .await;
    assert_success(&setup.process(&restore, &transaction));
    let (_, relike) = setup
        .like(&setup.stranger, post.id(), author, Some(&["dash", "news"]))
        .await;
    assert_success(&setup.process(&relike, &transaction));
    let (_, owner_like) = setup
        .like(&setup.owner, post.id(), author, Some(&["dash", "news"]))
        .await;
    assert_success(&setup.process(&owner_like, &transaction));
    assert_eq!(counters(&transaction), [Some(3), Some(3)]);
}
