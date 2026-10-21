//! Read-only discovery through the SDK and its trusted HTTPS quorum provider.
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
    get_identity_by_non_unique_public_key_hash_response_v0::Result as LookupResult, Version,
};
use dapi_grpc::platform::v0::GetIdentityByNonUniquePublicKeyHashRequest;
use dash_sdk::platform::types::identity::NonUniquePublicKeyHashQuery;
use dash_sdk::{
    platform::{Fetch, Identifier, Identity},
    SdkBuilder,
};
use dpp::dashcore::Network;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::platform_value::string_encoding::Encoding;
use dpp::version::PlatformVersion;
use drive_proof_verifier::FromProof;
use rs_dapi_client::{DapiRequestExecutor, RequestSettings};
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use std::num::NonZeroUsize;

#[tokio::test]
#[ignore = "requires live mainnet and the trusted HTTPS quorum service"]
async fn should_discover_mainnet_identity() {
    // Explorer supplies only the candidate ID; the SDK authenticates its keys below.
    check(
        Network::Mainnet,
        "5EwNaBtXG6fKafCix4gr9FEGGX7gHUjwdHEhmR8Bx8zd",
        true,
    )
    .await;
}

#[tokio::test]
#[ignore = "requires live mainnet and the trusted HTTPS quorum service"]
async fn should_prove_mainnet_unique_key_hashes_absent_from_nonunique_index() {
    check(
        Network::Mainnet,
        "EWSqsaghuwHRjtutbXK3nR11KbRkg9a12PNAAkJWRTpY",
        false,
    )
    .await;
}
#[tokio::test]
#[ignore = "requires live testnet and the trusted HTTPS quorum service"]
async fn should_discover_testnet_identity() {
    check(
        Network::Testnet,
        "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
        true,
    )
    .await;
}
async fn check(network: Network, id: &str, require_positive: bool) {
    let provider =
        TrustedHttpContextProvider::new(network, None, NonZeroUsize::new(100).unwrap()).unwrap();
    let builder = match network {
        Network::Mainnet => SdkBuilder::new_mainnet(),
        _ => SdkBuilder::new_testnet(),
    };
    let sdk = builder.with_context_provider(provider).build().unwrap();
    let id = Identifier::from_string(id, Encoding::Base58).unwrap();
    let identity = match Identity::fetch(&sdk, id).await {
        Ok(Some(identity)) => identity,
        other => panic!("{network:?} authenticated identity fetch failed: {other:?}"),
    };
    let mut checked = 0;
    let mut nonunique_matches = 0;
    for key in identity.public_keys().values() {
        let hash = key.public_key_hash().unwrap();
        let mut after = None;
        let mut found = false;
        let mut complete = false;
        for page in 0..100 {
            match Identity::fetch(
                &sdk,
                NonUniquePublicKeyHashQuery {
                    key_hash: hash,
                    after,
                },
            )
            .await
            {
                Ok(Some(result)) => {
                    assert!(after.is_none_or(|a| result.id().to_buffer() > a));
                    found |= result.id() == id;
                    after = Some(result.id().to_buffer());
                    println!(
                        "{network:?} hash={} page={page} identity={} verified_protocol={}",
                        hex::encode(hash),
                        result.id(),
                        sdk.version().protocol_version
                    );
                }
                Ok(None) => {
                    println!(
                        "{network:?} hash={} page={page} proved_absence",
                        hex::encode(hash)
                    );
                    complete = true;
                    break;
                }
                Err(error) => {
                    eprintln!("{network:?} discovery failed: {error}");
                    break;
                }
            }
        }
        assert!(complete, "pagination must terminate with a proved absence");
        if key.key_type().is_unique_key_type() {
            println!(
                "{network:?} unique key hash={} nonunique_match={found} verified_protocol={}",
                hex::encode(hash),
                sdk.version().protocol_version
            );
        } else {
            nonunique_matches += 1;
            assert!(
                found,
                "non-unique key must discover the authenticated identity"
            );
        }
        checked += 1;
    }
    assert!(checked > 0, "identity must have a public key");
    assert!(
        !require_positive || nonunique_matches > 0,
        "positive discovery fixture must contain a nonunique key"
    );
    // Authenticate an exposed raw response as well, to record the actual wire variant.
    let hash = identity
        .public_keys()
        .values()
        .next()
        .unwrap()
        .public_key_hash()
        .unwrap();
    let request: GetIdentityByNonUniquePublicKeyHashRequest =
        GetIdentityByNonUniquePublicKeyHashRequestV0 {
            public_key_hash: hash.to_vec(),
            start_after: None,
            prove: true,
        }
        .into();
    let response = sdk
        .execute(request.clone(), RequestSettings::default())
        .await
        .unwrap();
    let Some(Version::V0(v0)) = &response.inner.version else {
        panic!("missing version")
    };
    let format = match &v0.result {
        Some(LookupResult::LegacyProof(_)) => "legacy field2",
        Some(LookupResult::Proof(_)) => "current field4",
        _ => panic!("missing proof"),
    };
    let (_, metadata, _) = <Identity as FromProof<GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof_with_metadata(
        request, response.inner, network, PlatformVersion::latest(), &sdk.context_provider().unwrap(),
    ).unwrap();
    sdk.verify_response_metadata("get_identity_by_non_unique_public_key_hash", &metadata)
        .unwrap();
    println!(
        "{network:?} authenticated_wire={format} node={} height={} protocol={}",
        response.address, metadata.height, metadata.protocol_version
    );
    println!("{network:?} fully verified identity discovery passed for {checked} key hashes");
}
