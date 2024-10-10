use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_nonce_request::GetIdentityNonceRequestV0;
use dapi_grpc::platform::v0::get_identity_nonce_response::{
    get_identity_nonce_response_v0, GetIdentityNonceResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::platform_value::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identity_nonce_v0(
        &self,
        GetIdentityNonceRequestV0 { identity_id, prove }: GetIdentityNonceRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityNonceResponseV0>, Error> {
        let identity_id = check_validation_result_with_data!(Identifier::from_vec(identity_id)
            .map(|bytes| bytes.0)
            .map_err(|_| QueryError::InvalidArgument(
                "identity id must be 32 bytes long".to_string()
            )));

        let response = if prove {
            let proof = self
                .drive
                .prove_identity_nonce(identity_id.0, None, platform_version)?;

            GetIdentityNonceResponseV0 {
                result: Some(get_identity_nonce_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let maybe_identity =
                self.drive
                    .fetch_identity_nonce(identity_id.0, true, None, platform_version)?;

            // default here is 0;
            let identity_nonce = maybe_identity.unwrap_or_default();

            GetIdentityNonceResponseV0 {
                metadata: Some(self.response_metadata_v0(platform_state)),
                result: Some(get_identity_nonce_response_v0::Result::IdentityNonce(
                    identity_nonce,
                )),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_nonce::{
        IDENTITY_NONCE_VALUE_FILTER, IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES,
    };
    use drive::util::test_helpers::test_utils::identities::create_test_identity_with_rng;
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityNonceRequestV0 {
            identity_id: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_identity_nonce_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity id must be 32 bytes long")));
    }

    #[test]
    fn test_identity_not_found_when_querying_identity_nonce() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityNonceRequestV0 {
            identity_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_identity_nonce_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        assert!(matches!(
            result.data,
            Some(GetIdentityNonceResponseV0 {
                result: Some(get_identity_nonce_response_v0::Result::IdentityNonce(0)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_identity_is_found_when_querying_identity_nonce() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let mut rng = StdRng::seed_from_u64(10);
        let id = rng.gen::<[u8; 32]>();
        let _unused_identity =
            create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
                .expect("expected to create a test identity");

        let request = GetIdentityNonceRequestV0 {
            identity_id: id.to_vec(),
            prove: false,
        };

        let result = platform
            .query_identity_nonce_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        assert!(matches!(
            result.data,
            Some(GetIdentityNonceResponseV0 {
                result: Some(get_identity_nonce_response_v0::Result::IdentityNonce(0)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_identity_is_found_when_querying_identity_nonce_after_update() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let mut rng = StdRng::seed_from_u64(10);
        let id = rng.gen::<[u8; 32]>();
        let identity = create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
            .expect("expected to create a test identity");

        platform
            .drive
            .merge_identity_nonce(
                identity.id().to_buffer(),
                1,
                &BlockInfo::genesis(),
                true,
                None,
                version,
            )
            .expect("expected to set nonce");

        platform
            .drive
            .merge_identity_nonce(
                identity.id().to_buffer(),
                3,
                &BlockInfo::genesis(),
                true,
                None,
                version,
            )
            .expect("expected to set nonce");

        let request = GetIdentityNonceRequestV0 {
            identity_id: id.to_vec(),
            prove: false,
        };

        let result = platform
            .query_identity_nonce_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        assert!(matches!(
            result.data,
            Some(GetIdentityNonceResponseV0 {
                result: Some(get_identity_nonce_response_v0::Result::IdentityNonce(nonce)),
                metadata: Some(_),
            }) if nonce & IDENTITY_NONCE_VALUE_FILTER == 3 && nonce >> IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES == 1
        ));
    }

    #[test]
    fn test_identity_is_found_when_querying_identity_nonce_after_update_for_past() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let mut rng = StdRng::seed_from_u64(10);
        let id = rng.gen::<[u8; 32]>();
        let identity = create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
            .expect("expected to create a test identity");

        platform
            .drive
            .merge_identity_nonce(
                identity.id().to_buffer(),
                1,
                &BlockInfo::default(),
                true,
                None,
                version,
            )
            .expect("expected to set nonce");

        platform
            .drive
            .merge_identity_nonce(
                identity.id().to_buffer(),
                3,
                &BlockInfo::default(),
                true,
                None,
                version,
            )
            .expect("expected to set nonce");

        // we already added 3, and now are adding 2
        platform
            .drive
            .merge_identity_nonce(
                identity.id().to_buffer(),
                2,
                &BlockInfo::default(),
                true,
                None,
                version,
            )
            .expect("expected to set nonce");

        let request = GetIdentityNonceRequestV0 {
            identity_id: id.to_vec(),
            prove: false,
        };

        let result = platform
            .query_identity_nonce_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        assert!(matches!(
            result.data,
            Some(GetIdentityNonceResponseV0 {
                result: Some(get_identity_nonce_response_v0::Result::IdentityNonce(3)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_identity_contract_nonce_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityNonceRequestV0 {
            identity_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_identity_nonce_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetIdentityNonceResponseV0 {
                result: Some(get_identity_nonce_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
