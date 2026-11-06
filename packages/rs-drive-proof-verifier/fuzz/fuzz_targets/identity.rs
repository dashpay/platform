//! Identity by id, plus its balance, public keys and DPNS contract nonce.
//!
//! Balance, keys and contract nonce must verify to what the recorded proofs of
//! the signed root yield. The corpus has no full-identity proof, so an
//! accepted identity is checked against the proven balance and keys instead.

#![no_main]

use std::sync::LazyLock;

use dapi_grpc::platform::v0::{
    self as platform, get_identity_balance_request, get_identity_balance_response,
    get_identity_contract_nonce_request, get_identity_contract_nonce_response,
    get_identity_keys_request, get_identity_keys_response, get_identity_request,
    get_identity_response, key_request_type, AllKeys, KeyRequestType,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{Identity, IdentityNonce};
use dpp::version::PlatformVersion;
use drive_proof_verifier::types::{
    IdentityBalance, IdentityContractNonceFetcher, IdentityPublicKeys,
};
use drive_proof_verifier::Error;
use drive_proof_verifier_fuzz::{
    assert_consistent, decode, metadata, proof, recorded, verify, DPNS_CONTRACT,
    IDENTITY_BALANCE_PROOF, IDENTITY_CONTRACT_NONCE_PROOF, IDENTITY_ID, IDENTITY_KEYS_PROOF,
    PLATFORM_VERSIONS,
};
use libfuzzer_sys::fuzz_target;

type Verified<T> = Result<Option<T>, Error>;

static BALANCE: LazyLock<Option<IdentityBalance>> = LazyLock::new(|| {
    let grovedb_proof = decode(IDENTITY_BALANCE_PROOF);
    recorded(|platform_version| balance(platform_version, &grovedb_proof))
});
static KEYS: LazyLock<Option<IdentityPublicKeys>> = LazyLock::new(|| {
    let grovedb_proof = decode(IDENTITY_KEYS_PROOF);
    recorded(|platform_version| keys(platform_version, &grovedb_proof))
});
static CONTRACT_NONCE: LazyLock<Option<IdentityNonce>> = LazyLock::new(|| {
    let grovedb_proof = decode(IDENTITY_CONTRACT_NONCE_PROOF);
    recorded(|platform_version| contract_nonce(platform_version, &grovedb_proof))
});

fn identity(platform_version: &PlatformVersion, grovedb_proof: &[u8]) -> Verified<Identity> {
    verify::<platform::GetIdentityRequest, _>(
        platform_version,
        get_identity_request::GetIdentityRequestV0 {
            id: IDENTITY_ID.to_vec(),
            prove: true,
        },
        get_identity_response::GetIdentityResponseV0 {
            metadata: Some(metadata()),
            result: Some(
                get_identity_response::get_identity_response_v0::Result::Proof(proof(
                    grovedb_proof,
                )),
            ),
        },
    )
}

fn balance(platform_version: &PlatformVersion, grovedb_proof: &[u8]) -> Verified<IdentityBalance> {
    verify::<platform::GetIdentityBalanceRequest, _>(
        platform_version,
        get_identity_balance_request::GetIdentityBalanceRequestV0 {
            id: IDENTITY_ID.to_vec(),
            prove: true,
        },
        get_identity_balance_response::GetIdentityBalanceResponseV0 {
            metadata: Some(metadata()),
            result: Some(
                get_identity_balance_response::get_identity_balance_response_v0::Result::Proof(
                    proof(grovedb_proof),
                ),
            ),
        },
    )
}

fn keys(platform_version: &PlatformVersion, grovedb_proof: &[u8]) -> Verified<IdentityPublicKeys> {
    verify::<platform::GetIdentityKeysRequest, _>(
        platform_version,
        get_identity_keys_request::GetIdentityKeysRequestV0 {
            identity_id: IDENTITY_ID.to_vec(),
            request_type: Some(KeyRequestType {
                request: Some(key_request_type::Request::AllKeys(AllKeys {})),
            }),
            limit: None,
            offset: None,
            prove: true,
        },
        get_identity_keys_response::GetIdentityKeysResponseV0 {
            metadata: Some(metadata()),
            result: Some(
                get_identity_keys_response::get_identity_keys_response_v0::Result::Proof(proof(
                    grovedb_proof,
                )),
            ),
        },
    )
}

fn contract_nonce(
    platform_version: &PlatformVersion,
    grovedb_proof: &[u8],
) -> Verified<IdentityNonce> {
    verify::<platform::GetIdentityContractNonceRequest, IdentityContractNonceFetcher>(
        platform_version,
        get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0 {
            identity_id: IDENTITY_ID.to_vec(),
            contract_id: DPNS_CONTRACT.id().to_vec(),
            prove: true,
        },
        get_identity_contract_nonce_response::GetIdentityContractNonceResponseV0 {
            metadata: Some(metadata()),
            result: Some(
                get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::Proof(
                    proof(grovedb_proof),
                ),
            ),
        },
    )
    .map(|nonce| nonce.map(|nonce| nonce.0))
}

fuzz_target!(|data: &[u8]| {
    for platform_version in PLATFORM_VERSIONS {
        assert_consistent(balance(platform_version, data), &BALANCE);
        assert_consistent(keys(platform_version, data), &KEYS);
        assert_consistent(contract_nonce(platform_version, data), &CONTRACT_NONCE);

        if let Ok(identity) = identity(platform_version, data) {
            let identity = identity.map(|identity| {
                let keys = identity
                    .public_keys()
                    .iter()
                    .map(|(id, key)| (*id, Some(key.clone())))
                    .collect::<IdentityPublicKeys>();
                (identity.balance(), keys)
            });
            assert_eq!(
                identity,
                BALANCE.zip(KEYS.clone()),
                "a proof of the signed root verified to an identity the recorded proofs do not yield"
            );
        }
    }
});
