//! Read-only discovery through the SDK and its trusted HTTPS quorum provider.
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::{
    GetIdentityByNonUniquePublicKeyHashRequestV0, GetIdentityByNonUniquePublicKeyHashRequestV1,
};
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
    get_identity_by_non_unique_public_key_hash_response_v0::Result as LookupResult,
    get_identity_by_non_unique_public_key_hash_response_v1::Result as V1LookupResult, Version,
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
    // Authenticate exposed raw responses as well, to record the actual wire variants.
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
        panic!("a V0 request must get a V0 response")
    };
    assert!(
        matches!(&v0.result, Some(LookupResult::Proof(_))),
        "every node answers V0 with two proofs"
    );
    let (_, metadata, _) = <Identity as FromProof<GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof_with_metadata(
        request, response.inner, network, PlatformVersion::latest(), &sdk.context_provider().unwrap(),
    ).unwrap();
    sdk.verify_response_metadata("get_identity_by_non_unique_public_key_hash", &metadata)
        .unwrap();
    println!(
        "{network:?} authenticated_wire=v0 two proofs node={} height={} protocol={}",
        response.address, metadata.height, metadata.protocol_version
    );
    // V1 is served from protocol 14 by nodes that know it; older nodes refuse it.
    let request: GetIdentityByNonUniquePublicKeyHashRequest =
        GetIdentityByNonUniquePublicKeyHashRequestV1 {
            public_key_hash: hash.to_vec(),
            start_after: None,
            prove: true,
        }
        .into();
    match sdk
        .execute(request.clone(), RequestSettings::default())
        .await
    {
        Ok(response) => {
            let Some(Version::V1(v1)) = &response.inner.version else {
                panic!("a V1 request must get a V1 response")
            };
            assert!(matches!(&v1.result, Some(V1LookupResult::Proof(_))));
            let (_, metadata, _) = <Identity as FromProof<
                GetIdentityByNonUniquePublicKeyHashRequest,
            >>::maybe_from_proof_with_metadata(
                request,
                response.inner,
                network,
                PlatformVersion::latest(),
                &sdk.context_provider().unwrap(),
            )
            .unwrap();
            println!(
                "{network:?} authenticated_wire=v1 one proof node={} height={} protocol={}",
                response.address, metadata.height, metadata.protocol_version
            );
        }
        Err(error) => println!("{network:?} v1 refused: {error}"),
    }
    println!("{network:?} fully verified identity discovery passed for {checked} key hashes");
}

/// Records authenticated mainnet V0 answers (two proofs) as offline replay vectors.
/// Run it only to refresh them; the replay test in drive-proof-verifier needs no network.
/// It rewrites committed files, so running the live checks with `--ignored` skips it
/// unless `DASH_SDK_CAPTURE_MAINNET_VECTORS=1` is set.
#[tokio::test]
#[ignore = "requires live mainnet and DASH_SDK_CAPTURE_MAINNET_VECTORS=1; rewrites drive-proof-verifier's mainnet vectors"]
async fn should_capture_mainnet_v0_nonunique_vectors() {
    use dapi_grpc::Message;
    use dash_context_provider::ContextProvider;
    if std::env::var("DASH_SDK_CAPTURE_MAINNET_VECTORS").as_deref() != Ok("1") {
        eprintln!(
            "skipped: set DASH_SDK_CAPTURE_MAINNET_VECTORS=1 to rewrite the recorded mainnet vectors"
        );
        return;
    }
    let network = Network::Mainnet;
    let provider =
        TrustedHttpContextProvider::new(network, None, NonZeroUsize::new(100).unwrap()).unwrap();
    let sdk = SdkBuilder::new_mainnet()
        .with_context_provider(provider)
        .build()
        .unwrap();
    let id = Identifier::from_string(
        "5EwNaBtXG6fKafCix4gr9FEGGX7gHUjwdHEhmR8Bx8zd",
        Encoding::Base58,
    )
    .unwrap();
    let identity = Identity::fetch(&sdk, id).await.unwrap().unwrap();
    let hash = identity
        .public_keys()
        .values()
        .find(|key| !key.key_type().is_unique_key_type())
        .expect("fixture identity has a non-unique key")
        .public_key_hash()
        .unwrap();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-drive-proof-verifier/tests/vectors/identity-discovery/mainnet");
    std::fs::create_dir_all(&directory).unwrap();
    for (name, after) in [("match", None), ("absence", Some([0xff; 32]))] {
        let request: GetIdentityByNonUniquePublicKeyHashRequest =
            GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash: hash.to_vec(),
                start_after: after.map(|a: [u8; 32]| a.to_vec()),
                prove: true,
            }
            .into();
        let response = sdk
            .execute(request.clone(), RequestSettings::default())
            .await
            .unwrap()
            .inner;
        let Some(Version::V0(v0)) = &response.version else {
            panic!("missing version")
        };
        let Some(LookupResult::Proof(proved)) = &v0.result else {
            panic!("mainnet must answer V0 with two proofs")
        };
        let proof = proved
            .grovedb_identity_public_key_hash_proof
            .clone()
            .unwrap();
        let provider = sdk.context_provider().unwrap();
        let (found, metadata, _) = <Identity as FromProof<
            GetIdentityByNonUniquePublicKeyHashRequest,
        >>::maybe_from_proof_with_metadata(
            request.clone(),
            response.clone(),
            network,
            PlatformVersion::latest(),
            &provider,
        )
        .unwrap();
        assert_eq!(metadata.protocol_version, 13);
        assert_eq!(found.is_some(), after.is_none());
        let quorum_hash: [u8; 32] = proof.quorum_hash.clone().try_into().unwrap();
        let quorum_key = provider
            .get_quorum_public_key(
                proof.quorum_type,
                quorum_hash,
                metadata.core_chain_locked_height,
            )
            .unwrap();
        let write = |suffix: &str, bytes: &[u8]| {
            std::fs::write(directory.join(format!("{name}-{suffix}.bin")), bytes).unwrap()
        };
        write("request", &request.encode_to_vec());
        write("response", &response.encode_to_vec());
        write("quorum-key", &quorum_key);
        if let Some(found) = found {
            write("identity", &found.id().to_buffer());
        }
        println!(
            "captured {name}: height={} protocol={} quorum={}",
            metadata.height,
            metadata.protocol_version,
            hex::encode(quorum_hash)
        );
    }
}
