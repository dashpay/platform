//! `where` entries over typed arrays through a `moderatedDocument` reference
//! (protocol version 14). Once a moderator removes the post an echo names, a
//! replace of the echo checks the entries it asks about against the post's
//! removal record, which keeps the post's `tags` and `members`: the equality of
//! the echo's `tags` with the post's when the replace changes them, and the
//! `contains` of the writer among the post's `members` on every replace.

use super::*;

const TAGGED_POST: &str = "taggedPost";
const ECHO: &str = "echo";

fn string_list(position: u64) -> Value {
    platform_value!({
        "type": "array",
        "items": { "type": "string", "minLength": 1, "maxLength": 63 },
        "maxItems": 4,
        "position": position,
    })
}

/// A post only moderators take down, each removal keeping its tags and its
/// members
fn tagged_post_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["tags", "members"] },
        "properties": {
            "tags": string_list(0),
            "members": {
                "type": "array",
                "items": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                },
                "maxItems": 4,
                "position": 1,
            },
        },
        "additionalProperties": false,
    })
}

/// An echo of a post's tags, written only by one of its members
fn echo_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
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
                    "documentType": TAGGED_POST,
                    "where": {
                        "tags": "tags",
                        "members": { "contains": "$ownerId" },
                    },
                },
            },
            "tags": string_list(1),
            "note": { "type": "string", "maxLength": 50, "position": 2 },
        },
        "required": ["postId"],
        "additionalProperties": false,
    })
}

async fn setup() -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(contract, TAGGED_POST, tagged_post_schema());
            add_document_type(contract, ECHO, echo_schema());
        },
    )
    .await
}

fn texts(texts: &[&str]) -> Value {
    Value::Array(
        texts
            .iter()
            .map(|text| Value::Text(text.to_string()))
            .collect(),
    )
}

fn assert_where_refused(execution: &StateTransitionExecutionResult, referring: &str) {
    assert!(
        matches!(
            execution,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(e)
                ),
                ..
            } if e.referring_property() == referring
        ),
        "expected the where entry {referring} to refuse the write, got {execution:?}"
    );
}

/// The record keeps the post's tags and members: an echo leaving its tags
/// alone is asked only whether its writer is still a member, and one changing
/// them must still echo the tags the record kept, in their order. A list the
/// post held none of is kept as the empty one.
#[tokio::test]
async fn should_compare_lists_with_a_removal_record() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let member = Value::Array(vec![Value::Identifier(setup.stranger.id().to_buffer())]);
    let (post, create_post) = create_with(
        &setup,
        &setup.user,
        TAGGED_POST,
        vec![
            ("tags", texts(&["dash", "news"])),
            ("members", member.clone()),
        ],
    )
    .await;
    assert_success(&setup.process(&create_post, &transaction));
    let (untagged, create_untagged) =
        create_with(&setup, &setup.user, TAGGED_POST, vec![("members", member)]).await;
    assert_success(&setup.process(&create_untagged, &transaction));

    let (_, outsider) = create_with(
        &setup,
        &setup.user,
        ECHO,
        vec![
            ("postId", Value::Identifier(post.id().to_buffer())),
            ("tags", texts(&["dash", "news"])),
        ],
    )
    .await;
    assert_where_refused(
        &setup.process(&outsider, &transaction),
        "{ \"contains\": \"$ownerId\" }",
    );
    let (mut echo, create_echo) = create_with(
        &setup,
        &setup.stranger,
        ECHO,
        vec![
            ("postId", Value::Identifier(post.id().to_buffer())),
            ("tags", texts(&["dash", "news"])),
        ],
    )
    .await;
    assert_success(&setup.process(&create_echo, &transaction));
    let (mut untagged_echo, create_untagged_echo) = create_with(
        &setup,
        &setup.stranger,
        ECHO,
        vec![("postId", Value::Identifier(untagged.id().to_buffer()))],
    )
    .await;
    assert_success(&setup.process(&create_untagged_echo, &transaction));

    for removed in [post.id(), untagged.id()] {
        let remove = setup
            .moderate(&setup.moderator, delete_action(TAGGED_POST, removed))
            .await;
        assert_success(&setup.process(&remove, &transaction));
    }

    // The writer is asked about on every replace, and is still a kept member
    echo.set("note", "still here".into());
    let edit = replacement(&setup.stranger, &setup.contract, ECHO, &mut echo).await;
    assert_success(&setup.process(&edit, &transaction));

    // Changed tags are compared with the kept ones, order and all
    let mut reordered = echo.clone();
    reordered.set("tags", texts(&["news", "dash"]));
    let refused = replacement(&setup.stranger, &setup.contract, ECHO, &mut reordered).await;
    assert_where_refused(&setup.process(&refused, &transaction), "tags");

    // The untagged post's record keeps no tags, the empty list an empty one equals
    untagged_echo.set("tags", texts(&[]));
    let emptied = replacement(&setup.stranger, &setup.contract, ECHO, &mut untagged_echo).await;
    assert_success(&setup.process(&emptied, &transaction));
    let mut tagged = untagged_echo.clone();
    tagged.set("tags", texts(&["dash"]));
    let refused = replacement(&setup.stranger, &setup.contract, ECHO, &mut tagged).await;
    assert_where_refused(&setup.process(&refused, &transaction), "tags");
}
