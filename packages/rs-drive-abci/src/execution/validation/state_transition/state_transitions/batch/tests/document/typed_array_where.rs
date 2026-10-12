//! `where` entries over typed arrays through the full ABCI pipeline (protocol
//! version 14). The fixture's `post` holds `tags`, `members` and `scores`; a
//! `like` echoes its post's `tags` (an equality between two typed arrays),
//! a `copy` its `members` and `scores`, a `tagged` names one of its tags
//! (`contains`), a `memberNote` is written only by one of its members
//! (`contains` of the writer), and a `subset` names some of its tags
//! (`containsAll`). Lists are compared element by element, by value, and a
//! list a document leaves out is the empty one.

use super::*;

mod typed_array_where_tests {
    use super::super::reference_test_setup::{assert_successful, ReferenceTestSetup as Setup};
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use dpp::identifier::Identifier;

    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-where-typed-arrays.json";

    /// The write was refused because the post fails the `where` entry whose
    /// referring side is `referring`: a property for an equality, the
    /// condition as written for the others (code 40127).
    fn assert_mismatch(result: &StateTransitionsProcessingResult, referring: &str, because: &str) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(mismatch)
                ),
                ..
            }] if mismatch.referring_property() == referring,
            "{because}: got {result:?}"
        );
    }

    fn texts(texts: &[&str]) -> Value {
        Value::Array(
            texts
                .iter()
                .map(|text| Value::Text(text.to_string()))
                .collect(),
        )
    }

    impl Setup {
        /// A post holding exactly `properties`, created.
        async fn post(&mut self, properties: &[(&str, Value)]) -> Identifier {
            let (post, result) = self.create("post", properties).await;
            assert_successful(&result, "the post is created");
            post.id()
        }

        /// A document of `type_name` naming the post `post_id`, beside
        /// `properties`.
        async fn create_on(
            &mut self,
            type_name: &str,
            post_id: Identifier,
            properties: &[(&str, Value)],
        ) -> StateTransitionsProcessingResult {
            let mut all = vec![("postId", Value::Identifier(post_id.to_buffer()))];
            all.extend(properties.iter().cloned());
            self.create(type_name, &all).await.1
        }
    }

    /// A like echoes its post's tags exactly: the same tags in the same
    /// order. Reordered, fewer, more or none differ.
    #[tokio::test]
    async fn should_agree_on_a_list_with_the_same_elements_in_the_same_order() {
        let mut setup = Setup::new(CONTRACT_PATH, 6101);
        let post = setup.post(&[("tags", texts(&["dash", "news"]))]).await;

        let result = setup
            .create_on("like", post, &[("tags", texts(&["dash", "news"]))])
            .await;
        assert_successful(&result, "the same tags in the same order agree");

        for (tags, because) in [
            (
                Some(texts(&["news", "dash"])),
                "the order of the elements counts",
            ),
            (Some(texts(&["dash"])), "a missing element differs"),
            (
                Some(texts(&["dash", "news", "tech"])),
                "an extra element differs",
            ),
            (None, "a list left out is empty, and the post's is not"),
        ] {
            let properties: Vec<(&str, Value)> =
                tags.into_iter().map(|tags| ("tags", tags)).collect();
            let result = setup.create_on("like", post, &properties).await;
            assert_mismatch(&result, "tags", because);
        }
    }

    /// A list left out is the empty one: a like leaving out its tags agrees
    /// with a post that has none, and so does an empty list, either way round.
    #[tokio::test]
    async fn should_agree_on_lists_left_out_or_empty() {
        let mut setup = Setup::new(CONTRACT_PATH, 6102);
        let untagged = setup.post(&[("label", "plain".into())]).await;
        let result = setup.create_on("like", untagged, &[]).await;
        assert_successful(&result, "two lists left out agree");
        let result = setup
            .create_on("like", untagged, &[("tags", texts(&[]))])
            .await;
        assert_successful(&result, "an empty list agrees with one left out");

        let empty = setup.post(&[("tags", texts(&[]))]).await;
        let result = setup.create_on("like", empty, &[]).await;
        assert_successful(&result, "a list left out agrees with an empty one");

        let result = setup
            .create_on("like", untagged, &[("tags", texts(&["dash"]))])
            .await;
        assert_mismatch(&result, "tags", "a tag the post does not have differs");
    }

    /// Elements compare by value, not by how a transition carries them: an
    /// identifier equals its 32 bytes, and an integer is the same at any width.
    #[tokio::test]
    async fn should_compare_list_elements_by_value() {
        let mut setup = Setup::new(CONTRACT_PATH, 6103);
        let alice = Identifier::new([1; 32]);
        let bob = Identifier::new([2; 32]);
        let post = setup
            .post(&[
                (
                    "members",
                    Value::Array(vec![
                        Value::Identifier(alice.to_buffer()),
                        Value::Identifier(bob.to_buffer()),
                    ]),
                ),
                ("scores", Value::Array(vec![Value::U16(3), Value::U16(700)])),
            ])
            .await;

        let result = setup
            .create_on(
                "copy",
                post,
                &[
                    (
                        "members",
                        Value::Array(vec![
                            Value::Bytes(alice.to_buffer().to_vec()),
                            Value::Bytes(bob.to_buffer().to_vec()),
                        ]),
                    ),
                    ("scores", Value::Array(vec![Value::U64(3), Value::U64(700)])),
                ],
            )
            .await;
        assert_successful(&result, "equal values agree however they are carried");

        let result = setup
            .create_on(
                "copy",
                post,
                &[
                    (
                        "members",
                        Value::Array(vec![
                            Value::Identifier(alice.to_buffer()),
                            Value::Identifier(bob.to_buffer()),
                        ]),
                    ),
                    ("scores", Value::Array(vec![Value::U64(700), Value::U64(3)])),
                ],
            )
            .await;
        assert_mismatch(&result, "scores", "the order of the scores counts");
    }

    /// A `contains` holds when the post's list holds the referring value; a
    /// value left out is in no list, and a list left out holds nothing.
    #[tokio::test]
    async fn should_hold_contains_for_a_listed_value_only() {
        let mut setup = Setup::new(CONTRACT_PATH, 6104);
        let post = setup.post(&[("tags", texts(&["dash", "news"]))]).await;
        let condition = "{ \"contains\": \"tag\" }";

        let result = setup
            .create_on("tagged", post, &[("tag", "news".into())])
            .await;
        assert_successful(&result, "a listed tag is contained");

        let result = setup
            .create_on("tagged", post, &[("tag", "tech".into())])
            .await;
        assert_mismatch(&result, condition, "an unlisted tag is not contained");

        let result = setup.create_on("tagged", post, &[]).await;
        assert_mismatch(&result, condition, "a tag left out is in no list");

        let untagged = setup.post(&[("label", "plain".into())]).await;
        let result = setup
            .create_on("tagged", untagged, &[("tag", "dash".into())])
            .await;
        assert_mismatch(&result, condition, "a list left out holds nothing");
    }

    /// `contains` of `"$ownerId"` is a writer gate: only a listed member
    /// writes a note on the post.
    #[tokio::test]
    async fn should_let_only_a_listed_member_write() {
        let mut setup = Setup::new(CONTRACT_PATH, 6105);
        let writer = setup.owner;
        let condition = "{ \"contains\": \"$ownerId\" }";
        let with_writer = setup
            .post(&[(
                "members",
                Value::Array(vec![
                    Value::Identifier([9; 32]),
                    Value::Identifier(writer.to_buffer()),
                ]),
            )])
            .await;
        let result = setup
            .create_on("memberNote", with_writer, &[("text", "hi".into())])
            .await;
        assert_successful(&result, "a listed member writes");

        let without_writer = setup
            .post(&[("members", Value::Array(vec![Value::Identifier([9; 32])]))])
            .await;
        let result = setup
            .create_on("memberNote", without_writer, &[("text", "hi".into())])
            .await;
        assert_mismatch(&result, condition, "an unlisted writer is refused");

        let no_members = setup.post(&[("label", "plain".into())]).await;
        let result = setup
            .create_on("memberNote", no_members, &[("text", "hi".into())])
            .await;
        assert_mismatch(&result, condition, "a post without members admits no one");
    }

    /// A `containsAll` holds when the post's list holds every element of the
    /// referring list, in any order; every list holds all of an empty one or
    /// of one left out.
    #[tokio::test]
    async fn should_hold_contains_all_for_every_element_in_any_order() {
        let mut setup = Setup::new(CONTRACT_PATH, 6106);
        let post = setup
            .post(&[("tags", texts(&["dash", "news", "tech"]))])
            .await;
        let condition = "{ \"containsAll\": \"tags\" }";

        for (tags, because) in [
            (
                Some(texts(&["tech", "dash"])),
                "a subset in another order is held",
            ),
            (Some(texts(&["news", "news"])), "a repeated element is held"),
            (Some(texts(&[])), "every list holds all of an empty one"),
            (None, "a list left out is empty"),
        ] {
            let properties: Vec<(&str, Value)> =
                tags.into_iter().map(|tags| ("tags", tags)).collect();
            let result = setup.create_on("subset", post, &properties).await;
            assert_successful(&result, because);
        }

        let result = setup
            .create_on("subset", post, &[("tags", texts(&["dash", "sport"]))])
            .await;
        assert_mismatch(&result, condition, "an element the post lacks is not held");

        let untagged = setup.post(&[("label", "plain".into())]).await;
        let result = setup.create_on("subset", untagged, &[]).await;
        assert_successful(&result, "a list left out holds all of another left out");
        let result = setup
            .create_on("subset", untagged, &[("tags", texts(&["dash"]))])
            .await;
        assert_mismatch(&result, condition, "a list left out holds no element");
    }

    /// A replace changing the like's tags asks about the pair again, and the
    /// changed list must still be its post's.
    #[tokio::test]
    async fn should_check_a_changed_list_again_on_a_replace() {
        let mut setup = Setup::new(CONTRACT_PATH, 6107);
        let post = setup.post(&[("tags", texts(&["dash", "news"]))]).await;
        let (mut like, result) = setup
            .create(
                "like",
                &[
                    ("postId", Value::Identifier(post.to_buffer())),
                    ("tags", texts(&["dash", "news"])),
                ],
            )
            .await;
        assert_successful(&result, "the like is created");

        like.set("note", "first".into());
        let result = setup.replace("like", &mut like).await;
        assert_successful(&result, "a replace keeping the tags agrees");

        like.set("tags", texts(&["news", "dash"]));
        let result = setup.replace("like", &mut like).await;
        assert_mismatch(&result, "tags", "reordered tags no longer agree");
    }
}
