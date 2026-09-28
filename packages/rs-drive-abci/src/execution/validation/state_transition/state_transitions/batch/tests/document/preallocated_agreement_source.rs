//! A preallocated index keyed through a `propertyAgreement` through the full
//! ABCI pipeline (protocol version 14). The fixtures bind `like.hashtag` (at
//! most 63 characters) to `post.hashtag` and key the `byHashtagPost` index
//! through the pair, so creating a post writes its hashtag as a tree key.

use super::*;

mod preallocated_agreement_source_tests {
    use super::super::reference_test_setup::{
        assert_successful, create_document, register_contract_at,
    };
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::identity::signer::Signer;
    use dpp::identity::IdentityPublicKey;
    use dpp::prelude::DataContract;

    /// `post.hashtag` holds at most 63 characters, 252 bytes: every value fits
    /// a tree key.
    const PREALLOCATED_FITS_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-agreement-preallocated-fits.json";

    /// `post.hashtag` holds up to 280 characters, which registration refuses
    /// now; applied directly, as a contract registered before would be.
    const PREALLOCATED_TOO_WIDE_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-agreement-preallocated-too-wide.json";

    /// Creates a post under `post_hashtag`, asserting success, then a like on
    /// it under `like_hashtag`, and returns the like's result. Uses two
    /// nonces from `nonce`.
    #[allow(clippy::too_many_arguments)]
    async fn like_a_post<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        post_hashtag: &str,
        like_hashtag: &str,
        owner: Identifier,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let (post, result) = create_document(
            platform,
            platform_state,
            contract,
            "post",
            &[("hashtag", Value::Text(post_hashtag.to_string()))],
            owner,
            key,
            nonce,
            signer,
            rng,
            platform_version,
        )
        .await;
        assert_successful(&result, "the post must be created");
        let (_, result) = create_document(
            platform,
            platform_state,
            contract,
            "like",
            &[
                ("postId", Value::Identifier(post.id().to_buffer())),
                ("hashtag", Value::Text(like_hashtag.to_string())),
            ],
            owner,
            key,
            nonce + 1,
            signer,
            rng,
            platform_version,
        )
        .await;
        result
    }

    /// A post type whose hashtag fits a tree key keys the preallocated
    /// `byHashtagPost` index through the agreement: its posts, at the widest
    /// hashtag too, and the likes agreeing with them are created.
    #[tokio::test]
    async fn should_create_posts_and_likes_when_the_agreement_source_fits_a_tree_key() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(5312);
        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let contract = register_contract_at(
            &platform,
            PREALLOCATED_FITS_CONTRACT_PATH,
            identity.id(),
            true,
            platform_version,
        );

        // 63 characters of four bytes each: 252 bytes, the widest hashtag
        let widest = "\u{1F600}".repeat(63);
        for (nonce, hashtag) in [(2, "dash"), (4, widest.as_str())] {
            let result = like_a_post(
                &platform,
                &platform_state,
                &contract,
                hashtag,
                hashtag,
                identity.id(),
                &key,
                nonce,
                &signer,
                &mut rng,
                platform_version,
            )
            .await;
            assert_successful(
                &result,
                &format!("a like on a post under a {}-byte hashtag", hashtag.len()),
            );
        }
    }

    /// A contract applied before registration bounded the source of a
    /// preallocated index's agreement: its posts are created, those under a
    /// hashtag no like can carry included, and so are the likes on the
    /// others.
    #[tokio::test]
    async fn should_create_posts_under_an_agreement_source_wider_than_a_tree_key() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(5313);
        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let contract = register_contract_at(
            &platform,
            PREALLOCATED_TOO_WIDE_CONTRACT_PATH,
            identity.id(),
            true,
            platform_version,
        );

        // 70 characters of four bytes each: past any like's 63 characters
        // and past the 255 bytes of a tree key
        let (_, result) = create_document(
            &platform,
            &platform_state,
            &contract,
            "post",
            &[("hashtag", Value::Text("\u{1F600}".repeat(70)))],
            identity.id(),
            &key,
            2,
            &signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_successful(&result, "a post under a hashtag no like can carry");

        let result = like_a_post(
            &platform,
            &platform_state,
            &contract,
            "dash",
            "dash",
            identity.id(),
            &key,
            3,
            &signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_successful(&result, "a like on a post under a short hashtag");
    }
}
