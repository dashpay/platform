//! Identity-family proof-vector regression tests: balance, nonce, contract
//! nonce, and public keys, plus the corrupted-proof negative case. Every
//! case replays a recorded Dash Core fixture proof through
//! [`FromProof::maybe_from_proof_with_metadata`] with the real fixture
//! quorum signature, so both the grovedb and tenderdash layers are checked.

#![cfg(feature = "mocks")]

mod common;

use common::{identifier, load_case, Case, Expected, RequestSpec, NETWORK};
use dapi_grpc::platform::v0::get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result as ContractNonceResult;
use dapi_grpc::platform::v0::{
    self as platform, get_identity_balance_request, get_identity_balance_response,
    get_identity_contract_nonce_request, get_identity_contract_nonce_response,
    get_identity_keys_request, get_identity_keys_response, get_identity_nonce_request,
    get_identity_nonce_response, key_request_type, AllKeys, KeyRequestType,
};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::serialization::PlatformDeserializable;
use drive_proof_verifier::types::{
    IdentityBalance, IdentityContractNonceFetcher, IdentityNonceFetcher, IdentityPublicKeys,
};
use drive_proof_verifier::{Error, FromProof};

fn balance_pair(
    case: &Case,
) -> (
    platform::GetIdentityBalanceRequest,
    platform::GetIdentityBalanceResponse,
) {
    let RequestSpec::IdentityBalance { identity_id } = &case.manifest.request else {
        panic!("{}: expected an identity_balance request", case.name);
    };
    let request = platform::GetIdentityBalanceRequest {
        version: Some(get_identity_balance_request::Version::V0(
            get_identity_balance_request::GetIdentityBalanceRequestV0 {
                id: identifier(identity_id).to_vec(),
                prove: true,
            },
        )),
    };
    let response = platform::GetIdentityBalanceResponse {
        version: Some(get_identity_balance_response::Version::V0(
            get_identity_balance_response::GetIdentityBalanceResponseV0 {
                metadata: Some(case.metadata()),
                result: Some(
                    get_identity_balance_response::get_identity_balance_response_v0::Result::Proof(
                        case.grpc_proof(),
                    ),
                ),
            },
        )),
    };
    (request, response)
}

// The crate's own Error type is what the public API returns; its size is
// not this test's concern.
#[allow(clippy::result_large_err)]
fn verify_balance_case(case: &Case) -> Result<Option<IdentityBalance>, Error> {
    let (request, response) = balance_pair(case);
    IdentityBalance::maybe_from_proof(
        request,
        response,
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
}

#[test]
fn identity_balance() {
    let case = load_case("identity-balance");
    let Expected::IdentityBalance { balance: expected } = case.manifest.expected else {
        panic!("manifest expectation mismatch");
    };
    let (request, response) = balance_pair(&case);
    let (balance, metadata, proof) = IdentityBalance::maybe_from_proof_with_metadata(
        request,
        response,
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
    .expect("identity balance proof must verify");
    assert_eq!(balance, Some(expected));
    assert_eq!(metadata.height, case.manifest.block.height);
    assert_eq!(metadata.chain_id, case.manifest.block.chain_id);
    assert_eq!(proof.grovedb_proof, case.grovedb_proof);
}

#[test]
fn identity_nonce() {
    let case = load_case("identity-nonce");
    let RequestSpec::IdentityNonce { identity_id } = &case.manifest.request else {
        panic!("manifest request mismatch");
    };
    let Expected::IdentityNonce { nonce: expected } = case.manifest.expected else {
        panic!("manifest expectation mismatch");
    };
    let request = platform::GetIdentityNonceRequest {
        version: Some(get_identity_nonce_request::Version::V0(
            get_identity_nonce_request::GetIdentityNonceRequestV0 {
                identity_id: identifier(identity_id).to_vec(),
                prove: true,
            },
        )),
    };
    let response = platform::GetIdentityNonceResponse {
        version: Some(get_identity_nonce_response::Version::V0(
            get_identity_nonce_response::GetIdentityNonceResponseV0 {
                metadata: Some(case.metadata()),
                result: Some(
                    get_identity_nonce_response::get_identity_nonce_response_v0::Result::Proof(
                        case.grpc_proof(),
                    ),
                ),
            },
        )),
    };
    let fetcher = IdentityNonceFetcher::maybe_from_proof(
        request,
        response,
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
    .expect("identity nonce proof must verify")
    .expect("nonce must be proven present");
    assert_eq!(fetcher.0, expected);
}

#[test]
fn identity_contract_nonce() {
    let case = load_case("identity-contract-nonce");
    let RequestSpec::IdentityContractNonce {
        identity_id,
        contract_id,
    } = &case.manifest.request
    else {
        panic!("manifest request mismatch");
    };
    let Expected::IdentityContractNonce { nonce: expected } = case.manifest.expected else {
        panic!("manifest expectation mismatch");
    };
    let request = platform::GetIdentityContractNonceRequest {
        version: Some(get_identity_contract_nonce_request::Version::V0(
            get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0 {
                identity_id: identifier(identity_id).to_vec(),
                contract_id: identifier(contract_id).to_vec(),
                prove: true,
            },
        )),
    };
    let response = platform::GetIdentityContractNonceResponse {
        version: Some(get_identity_contract_nonce_response::Version::V0(
            get_identity_contract_nonce_response::GetIdentityContractNonceResponseV0 {
                metadata: Some(case.metadata()),
                result: Some(ContractNonceResult::Proof(case.grpc_proof())),
            },
        )),
    };
    let fetcher = IdentityContractNonceFetcher::maybe_from_proof(
        request,
        response,
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
    .expect("identity contract nonce proof must verify")
    .expect("contract nonce must be proven present");
    assert_eq!(fetcher.0, expected);
}

#[test]
fn identity_keys() {
    let case = load_case("identity-keys");
    let RequestSpec::IdentityKeys { identity_id } = &case.manifest.request else {
        panic!("manifest request mismatch");
    };
    let Expected::IdentityKeys { serialized_keys } = &case.manifest.expected else {
        panic!("manifest expectation mismatch");
    };
    let request = platform::GetIdentityKeysRequest {
        version: Some(get_identity_keys_request::Version::V0(
            get_identity_keys_request::GetIdentityKeysRequestV0 {
                identity_id: identifier(identity_id).to_vec(),
                request_type: Some(KeyRequestType {
                    request: Some(key_request_type::Request::AllKeys(AllKeys {})),
                }),
                limit: None,
                offset: None,
                prove: true,
            },
        )),
    };
    let response = platform::GetIdentityKeysResponse {
        version: Some(get_identity_keys_response::Version::V0(
            get_identity_keys_response::GetIdentityKeysResponseV0 {
                metadata: Some(case.metadata()),
                result: Some(
                    get_identity_keys_response::get_identity_keys_response_v0::Result::Proof(
                        case.grpc_proof(),
                    ),
                ),
            },
        )),
    };
    let keys = IdentityPublicKeys::maybe_from_proof(
        request,
        response,
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
    .expect("identity keys proof must verify")
    .expect("keys must be proven present");

    let expected: Vec<IdentityPublicKey> = serialized_keys
        .iter()
        .map(|serialized| {
            IdentityPublicKey::deserialize_from_bytes(&common::hex_vec(serialized))
                .expect("expected fixture key must deserialize")
        })
        .collect();
    assert_eq!(keys.len(), expected.len());
    for expected_key in &expected {
        let key = keys
            .get(&expected_key.id())
            .unwrap_or_else(|| panic!("key id {} missing from proof", expected_key.id()))
            .as_ref()
            .unwrap_or_else(|| panic!("key id {} proven absent", expected_key.id()));
        assert_eq!(key, expected_key);
    }
    assert_eq!(
        keys.keys().copied().collect::<Vec<KeyID>>(),
        vec![0, 1, 2],
        "keys must arrive ordered by id"
    );
}

// A flipped bit inside the grovedb proof must surface as a clean proof
// error before the quorum signature is ever consulted -- never a panic and
// never an Ok result.
#[test]
fn corrupted_grovedb_proof_is_rejected() {
    let case = load_case("identity-balance-corrupted-proof");
    let Expected::Error { class } = case.manifest.expected else {
        panic!("manifest expectation mismatch");
    };
    assert_eq!(class, common::ErrorClass::ProofInvalid);
    let error = verify_balance_case(&case).expect_err("corrupted proof must not verify");
    assert!(
        matches!(error, Error::GroveDBError { .. } | Error::DriveError { .. }),
        "expected a grovedb/drive proof error, got: {error:?}"
    );
}
