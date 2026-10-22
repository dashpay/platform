//! Authenticated answers recorded from mainnet protocol 13 nodes, replayed offline.
//!
//! Recorded by `rs-sdk/tests/identity_discovery_live.rs::should_capture_mainnet_v0_nonunique_vectors`.
//! These are request V0 answers, the only version 4.1 nodes serve.
//! The quorum keys are the ones that signed these answers, not trust anchors.
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
    get_identity_by_non_unique_public_key_hash_response_v0::Result as LookupResult, Version,
};
use dapi_grpc::platform::v0::{
    GetIdentityByNonUniquePublicKeyHashRequest, GetIdentityByNonUniquePublicKeyHashResponse,
};
use dapi_grpc::Message;
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier, Identity};
use dpp::version::PlatformVersion;
use drive_proof_verifier::FromProof;
use std::sync::Arc;

fn vector(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/vectors/identity-discovery/mainnet")
            .join(format!("{name}.bin")),
    )
    .unwrap()
}

/// Returns the recorded key only for the quorum that signed the recorded answer.
struct SigningQuorum {
    quorum_type: u32,
    quorum_hash: Vec<u8>,
    key: [u8; 48],
}
impl ContextProvider for SigningQuorum {
    fn get_data_contract(
        &self,
        _: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        assert_eq!(
            (quorum_type, quorum_hash.as_slice()),
            (self.quorum_type, self.quorum_hash.as_slice())
        );
        Ok(self.key)
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

/// Top-level protobuf fields of `bytes`; these messages use only varint and
/// length-delimited fields.
fn fields(mut bytes: &[u8]) -> Vec<(u64, &[u8])> {
    fn varint(bytes: &mut &[u8]) -> u64 {
        let mut value = 0;
        for shift in (0..64).step_by(7) {
            let (byte, rest) = bytes.split_first().unwrap();
            *bytes = rest;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return value;
            }
        }
        panic!("varint overflow")
    }
    let mut fields = vec![];
    while !bytes.is_empty() {
        let key = varint(&mut bytes);
        let value = match key & 7 {
            0 => {
                let start = bytes;
                varint(&mut bytes);
                &start[..start.len() - bytes.len()]
            }
            2 => {
                let length = varint(&mut bytes) as usize;
                let (value, rest) = bytes.split_at(length);
                bytes = rest;
                value
            }
            wire => panic!("unexpected wire type {wire}"),
        };
        fields.push((key >> 3, value));
    }
    fields
}

fn replay(
    name: &str,
) -> (
    GetIdentityByNonUniquePublicKeyHashRequest,
    GetIdentityByNonUniquePublicKeyHashResponse,
    SigningQuorum,
) {
    let request = GetIdentityByNonUniquePublicKeyHashRequest::decode(
        vector(&format!("{name}-request")).as_slice(),
    )
    .unwrap();
    let response = GetIdentityByNonUniquePublicKeyHashResponse::decode(
        vector(&format!("{name}-response")).as_slice(),
    )
    .unwrap();
    let Some(Version::V0(v0)) = &response.version else {
        panic!("v0")
    };
    let Some(LookupResult::Proof(proved)) = &v0.result else {
        panic!("recorded mainnet answers carry two proofs")
    };
    let proof = proved
        .grovedb_identity_public_key_hash_proof
        .as_ref()
        .unwrap();
    let quorum = SigningQuorum {
        quorum_type: proof.quorum_type,
        quorum_hash: proof.quorum_hash.clone(),
        key: vector(&format!("{name}-quorum-key")).try_into().unwrap(),
    };
    (request, response, quorum)
}

fn verify(
    request: GetIdentityByNonUniquePublicKeyHashRequest,
    response: GetIdentityByNonUniquePublicKeyHashResponse,
    quorum: &SigningQuorum,
    verifier: &PlatformVersion,
) -> Result<Option<Identity>, Box<drive_proof_verifier::Error>> {
    <Identity as FromProof<GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof(
        request,
        response,
        Network::Mainnet,
        verifier,
        quorum,
    )
    .map_err(Box::new)
}

/// A deployed protocol 13 node's answer stays verifiable by this client under both the
/// producer's and the newest verifier rules.
#[test]
fn should_verify_recorded_mainnet_v0_nonunique_answers() {
    for (name, expected) in [
        (
            "match",
            Some(Identifier::from_bytes(&vector("match-identity")).unwrap()),
        ),
        ("absence", None),
    ] {
        // Response version 0 carries result field 2: the index proof (field 1) and, when
        // an identity matched, its separate full-identity proof bytes (field 2).
        let bytes = vector(&format!("{name}-response"));
        let [(1, v0)] = fields(&bytes)[..] else {
            panic!("{name}: one response version 0")
        };
        let v0 = fields(v0);
        let numbers: Vec<u64> = v0.iter().map(|(number, _)| *number).collect();
        assert_eq!(
            numbers,
            [2, 3],
            "{name}: two-proof result and metadata only"
        );
        let proved: Vec<u64> = fields(v0[0].1).iter().map(|(number, _)| *number).collect();
        assert_eq!(
            proved,
            if expected.is_some() {
                vec![1, 2]
            } else {
                vec![1]
            },
            "{name}: two-proof fields"
        );

        for verifier in [PlatformVersion::get(13).unwrap(), PlatformVersion::latest()] {
            let (request, response, quorum) = replay(name);
            let found = verify(request, response, &quorum, verifier).unwrap();
            assert_eq!(
                found.map(|identity| identity.id()),
                expected,
                "{name}, verifier PV{}",
                verifier.protocol_version
            );
        }
    }
}

#[test]
fn should_refuse_altered_recorded_mainnet_v0_answers() {
    let (request, response, quorum) = replay("match");
    let latest = PlatformVersion::latest();

    // A truncated identity proof no longer decodes.
    let mut truncated = response.clone();
    let Some(Version::V0(v0)) = &mut truncated.version else {
        panic!("v0")
    };
    let Some(LookupResult::Proof(proved)) = &mut v0.result else {
        panic!("two proofs")
    };
    let identity_proof = proved.identity_proof_bytes.as_mut().unwrap();
    let last = identity_proof.len() - 1;
    identity_proof[last] ^= 1;
    let error = verify(request.clone(), truncated, &quorum, latest).unwrap_err();
    assert!(
        matches!(&*error, drive_proof_verifier::Error::GroveDBError { error, .. } if error.contains("unable to decode proof")),
        "{error}"
    );

    // A changed byte of the identity's own data still decodes, but no longer hashes to
    // the signed root.
    let mut altered = response.clone();
    let Some(Version::V0(v0)) = &mut altered.version else {
        panic!("v0")
    };
    let Some(LookupResult::Proof(proved)) = &mut v0.result else {
        panic!("two proofs")
    };
    let identity_proof = proved.identity_proof_bytes.as_mut().unwrap();
    let id = vector("match-identity");
    let at = identity_proof
        .windows(id.len())
        .position(|window| window == id.as_slice())
        .expect("the identity proof names the identity");
    identity_proof[at + id.len() / 2] ^= 1;
    let error = verify(request.clone(), altered, &quorum, latest).unwrap_err();
    assert!(
        matches!(&*error, drive_proof_verifier::Error::GroveDBError { error, .. } if error.contains("invalid proof")),
        "{error}"
    );

    let mut relabeled = response.clone();
    let Some(Version::V0(v0)) = &mut relabeled.version else {
        panic!("v0")
    };
    // Protocol 12 builds the same queries, so only the signature can refuse it.
    v0.metadata.as_mut().unwrap().protocol_version = 12;
    assert!(matches!(
        *verify(request.clone(), relabeled, &quorum, latest).unwrap_err(),
        drive_proof_verifier::Error::InvalidSignature { .. }
    ));

    let mut other_cursor = request;
    let Some(RequestVersion::V0(v0)) = &mut other_cursor.version else {
        panic!("v0")
    };
    v0.start_after = Some(vec![0xff; 32]);
    let error = verify(other_cursor, response, &quorum, latest).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("must bind its first eligible identity or prove absence"),
        "{error}"
    );
}
