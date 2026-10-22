//! Both wire formats bind the requested first-after index, complete identity, and signed state.
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
    get_identity_by_non_unique_public_key_hash_response_v0::{
        IdentityProvedResponse, Result as LookupResult,
    },
    GetIdentityByNonUniquePublicKeyHashResponseV0, Version,
};
use dapi_grpc::platform::v0::{
    GetIdentityByNonUniquePublicKeyHashRequest, GetIdentityByNonUniquePublicKeyHashResponse, Proof,
    ResponseMetadata,
};
use dapi_grpc::Message;
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::block::block_info::BlockInfo;
use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::prelude::{CoreBlockHeight, DataContract};
use dpp::prelude::{Identifier, Identity};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::PathQuery;
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use drive_proof_verifier::FromProof;
use std::sync::Arc;
use tenderdash_abci::{
    proto::types::{CanonicalVote, SignedMsgType, StateId},
    signatures::{Hashable, Signable},
};

/// A protocol version far above any real one. Recorded vectors signed with it stay newer
/// than every build, so the SDK tests replaying them never depend on the latest version.
const NEWER_PROTOCOL_VERSION: u32 = 1_000;

struct Fixture {
    drive: Drive,
    producer: &'static PlatformVersion,
    hash: [u8; 20],
    identities: Vec<Identity>,
}
impl Fixture {
    fn new(version: u32) -> Self {
        let producer = PlatformVersion::get(version).unwrap();
        let drive = setup_drive_with_initial_state_structure(Some(producer));
        let mut first = Identity::random_identity(3, Some(14), producer).unwrap();
        let key = first
            .public_keys()
            .values()
            .find(|k| !k.key_type().is_unique_key_type())
            .unwrap()
            .clone();
        let hash = key.public_key_hash().unwrap();
        first.set_id(Identifier::new([0x10; 32]));
        let mut second = Identity::random_identity(3, Some(15), producer).unwrap();
        second.add_public_key(key);
        second.set_id(Identifier::new([0x20; 32]));
        let identities = vec![first, second];
        for identity in &identities {
            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    producer,
                )
                .unwrap();
        }
        Self {
            drive,
            producer,
            hash,
            identities,
        }
    }
    fn root(&self) -> [u8; 32] {
        self.drive
            .grove
            .root_hash(None, &self.producer.drive.grove_version)
            .unwrap()
            .unwrap()
    }
    fn prove(&self, query: &PathQuery) -> Vec<u8> {
        self.drive
            .grove_get_proved_path_query(query, None, &mut vec![], &self.producer.drive)
            .unwrap()
    }
    fn response(
        &self,
        legacy: bool,
        after: Option<[u8; 32]>,
    ) -> GetIdentityByNonUniquePublicKeyHashResponse {
        self.response_signed_as(legacy, after, self.producer.protocol_version)
    }
    /// Build either wire format from this state and sign it as `protocol_version`,
    /// which may name a version the producer does not answer in that format.
    fn response_signed_as(
        &self,
        legacy: bool,
        after: Option<[u8; 32]>,
        protocol_version: u32,
    ) -> GetIdentityByNonUniquePublicKeyHashResponse {
        let metadata = ResponseMetadata {
            height: 10,
            core_chain_locked_height: 1,
            time_ms: 1_700_000_000_000,
            protocol_version,
            chain_id: "dash-testnet-51".into(),
            ..Default::default()
        };
        let result = if legacy {
            let mut query =
                Drive::identity_id_by_non_unique_public_key_hash_query(self.hash, after);
            query.query.limit = Some(1);
            let identity = self
                .identities
                .iter()
                .find(|id| after.is_none_or(|a| id.id().to_buffer() > a));
            LookupResult::LegacyProof(IdentityProvedResponse {
                grovedb_identity_public_key_hash_proof: Some(signed(
                    self.prove(&query),
                    &metadata,
                    self.root(),
                )),
                identity_proof_bytes: identity.map(|id| {
                    self.prove(
                        &Drive::full_identity_query(
                            &id.id().to_buffer(),
                            &self.producer.drive.grove_version,
                        )
                        .unwrap(),
                    )
                }),
            })
        } else {
            // Force the single-proof producer so older versions can be signed over it too.
            let mut producer = self.producer.clone();
            producer
                .drive
                .methods
                .identity
                .prove
                .prove_full_identity_by_non_unique_public_key_hash = 1;
            let bytes = self
                .drive
                .prove_full_identity_by_non_unique_public_key_hash(
                    self.hash, after, None, &producer,
                )
                .unwrap()
                .into_current()
                .unwrap();
            LookupResult::Proof(signed(bytes, &metadata, self.root()))
        };
        GetIdentityByNonUniquePublicKeyHashResponse {
            version: Some(Version::V0(GetIdentityByNonUniquePublicKeyHashResponseV0 {
                result: Some(result),
                metadata: Some(metadata),
            })),
        }
    }
}
struct Provider;
impl ContextProvider for Provider {
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
        kind: u32,
        hash: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        assert_eq!((kind, hash), (106, [2; 32]));
        Ok(key().public_key().0.to_compressed())
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}
fn key() -> SecretKey<Bls12381G2Impl> {
    SecretKey::from_be_bytes(&[1; 32]).into_option().unwrap()
}
fn signed(bytes: Vec<u8>, metadata: &ResponseMetadata, root: [u8; 32]) -> Proof {
    let mut proof = Proof {
        grovedb_proof: bytes,
        quorum_hash: vec![2; 32],
        quorum_type: 106,
        round: 1,
        block_id_hash: vec![3; 32],
        ..Default::default()
    };
    let state = StateId {
        app_version: u64::from(metadata.protocol_version),
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root.to_vec(),
        height: metadata.height,
    };
    let vote = CanonicalVote {
        r#type: SignedMsgType::Precommit.into(),
        block_id: proof.block_id_hash.clone(),
        chain_id: metadata.chain_id.clone(),
        height: metadata.height as i64,
        round: i64::from(proof.round),
        state_id: state
            .calculate_msg_hash(
                &metadata.chain_id,
                metadata.height as i64,
                proof.round as i32,
            )
            .unwrap(),
    };
    let digest = vote
        .calculate_sign_hash(
            &metadata.chain_id,
            proof.quorum_type as u8,
            &[2; 32],
            metadata.height as i64,
            proof.round as i32,
        )
        .unwrap();
    proof.signature = key()
        .sign(SignatureSchemes::Basic, &digest)
        .unwrap()
        .as_raw_value()
        .to_compressed()
        .to_vec();
    proof
}
fn verify(
    response: GetIdentityByNonUniquePublicKeyHashResponse,
    hash: [u8; 20],
    after: Option<[u8; 32]>,
) -> Result<Option<Identity>, Box<drive_proof_verifier::Error>> {
    let request: GetIdentityByNonUniquePublicKeyHashRequest =
        GetIdentityByNonUniquePublicKeyHashRequestV0 {
            public_key_hash: hash.to_vec(),
            start_after: after.map(|a| a.to_vec()),
            prove: true,
        }
        .into();
    // Exercise actual protobuf encoding/decoding before FromProof.
    let response =
        GetIdentityByNonUniquePublicKeyHashResponse::decode(response.encode_to_vec().as_slice())
            .unwrap();
    <Identity as FromProof<GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof(
        request,
        response,
        Network::Testnet,
        PlatformVersion::latest(),
        &Provider,
    )
    .map_err(Box::new)
}
fn inner(
    response: &mut GetIdentityByNonUniquePublicKeyHashResponse,
) -> &mut GetIdentityByNonUniquePublicKeyHashResponseV0 {
    let Some(Version::V0(v0)) = &mut response.version else {
        panic!("v0")
    };
    v0
}
fn proof(response: &mut GetIdentityByNonUniquePublicKeyHashResponse) -> &mut Proof {
    match inner(response).result.as_mut().unwrap() {
        LookupResult::Proof(proof) => proof,
        LookupResult::LegacyProof(legacy) => legacy
            .grovedb_identity_public_key_hash_proof
            .as_mut()
            .unwrap(),
        _ => panic!("proof"),
    }
}
/// The newest version whose nodes answer in the given wire format.
fn producer_for(legacy: bool) -> u32 {
    if legacy {
        13
    } else {
        PlatformVersion::latest().protocol_version
    }
}
fn assert_format_refused(
    result: Result<Option<Identity>, Box<drive_proof_verifier::Error>>,
    context: &str,
) {
    let error = result.expect_err(context);
    assert!(
        matches!(&*error, drive_proof_verifier::Error::ResponseDecodeError { error } if error.contains("proof format")),
        "{context}: {error}"
    );
}
#[test]
fn should_verify_nonunique_pagination_in_the_format_each_producer_version_emits() {
    for version in [12, 13, PlatformVersion::latest().protocol_version] {
        let f = Fixture::new(version);
        let legacy = version <= 13;
        for (after, expected) in [
            (None, Some(&f.identities[0])),
            (Some([0x10; 32]), Some(&f.identities[1])),
            (Some([0x20; 32]), None),
        ] {
            assert_eq!(
                verify(f.response(legacy, after), f.hash, after)
                    .unwrap()
                    .as_ref(),
                expected,
                "PV{version}, legacy={legacy}"
            );
        }
    }
}
/// A node must not choose the format: a correctly signed, otherwise valid answer in the
/// format its signed version does not emit is refused, never reinterpreted.
#[test]
fn should_refuse_nonunique_answers_in_a_format_the_signed_version_does_not_emit() {
    for (version, legacy) in [
        (12, false),
        (13, false),
        (PlatformVersion::latest().protocol_version, true),
    ] {
        let f = Fixture::new(version);
        for after in [None, Some([0x10; 32]), Some([0x20; 32])] {
            assert_format_refused(
                verify(f.response(legacy, after), f.hash, after),
                &format!("PV{version}, legacy={legacy}, after={after:?}"),
            );
        }
    }
}
/// A version newer than this build is read with the newest known rules: its nodes are
/// expected to answer in the newest format, still bound to the signed root.
#[test]
fn should_read_nonunique_answers_signed_by_an_unknown_newer_version_with_newest_rules() {
    let newer = PlatformVersion::latest().protocol_version + 1;
    let f = Fixture::new(PlatformVersion::latest().protocol_version);
    for (after, expected) in [
        (None, Some(&f.identities[0])),
        (Some([0x10; 32]), Some(&f.identities[1])),
        (Some([0x20; 32]), None),
    ] {
        assert_eq!(
            verify(f.response_signed_as(false, after, newer), f.hash, after)
                .unwrap()
                .as_ref(),
            expected,
            "PV{newer}, after={after:?}"
        );
        assert_format_refused(
            verify(f.response_signed_as(true, after, newer), f.hash, after),
            &format!("PV{newer} legacy, after={after:?}"),
        );
    }
    let mut forged = f.response_signed_as(false, None, newer);
    inner(&mut forged)
        .metadata
        .as_mut()
        .unwrap()
        .protocol_version = newer + 1;
    assert!(matches!(
        *verify(forged, f.hash, None).unwrap_err(),
        drive_proof_verifier::Error::InvalidSignature { .. }
    ));
}
#[test]
fn should_reject_nonunique_wrong_hash_cursor_signature_and_metadata_tampering() {
    for legacy in [true, false] {
        let f = Fixture::new(producer_for(legacy));
        let response = f.response(legacy, None);
        assert!(verify(response.clone(), [9; 20], None).is_err());
        assert!(verify(response.clone(), f.hash, Some([0x10; 32])).is_err());
        assert!(verify(f.response(legacy, Some([0x10; 32])), f.hash, None).is_err());
        for field in 0..9 {
            let mut changed = response.clone();
            let m = inner(&mut changed).metadata.as_mut().unwrap();
            match field {
                0 => m.height += 1,
                1 => m.time_ms += 1,
                2 => m.core_chain_locked_height += 1,
                // Another version answering in the same format, so only the signature can refuse it.
                3 => {
                    m.protocol_version = if legacy {
                        12
                    } else {
                        PlatformVersion::latest().protocol_version + 1
                    }
                }
                4 => m.chain_id.push('x'),
                5 => m.protocol_version = u32::MAX,
                6 => m.protocol_version = 0,
                7 => m.protocol_version = PlatformVersion::latest().protocol_version + 1,
                _ => proof(&mut changed).signature[0] ^= 1,
            }
            let error = verify(changed, f.hash, None)
                .expect_err(&format!("legacy={legacy}, field={field}"));
            if field == 3 {
                assert!(
                    matches!(*error, drive_proof_verifier::Error::InvalidSignature { .. }),
                    "legacy={legacy}: {error}"
                );
            }
        }
    }
}
#[test]
fn should_reject_legacy_unbound_incomplete_malformed_and_v0_proofs() {
    let f = Fixture::new(13);
    for case in 0..7 {
        let mut response = f.response(true, None);
        let Some(LookupResult::LegacyProof(legacy)) = &mut inner(&mut response).result else {
            panic!("legacy")
        };
        match case {
            0 => legacy.identity_proof_bytes = None,
            1 => legacy.identity_proof_bytes = Some(vec![1]),
            2 => legacy.identity_proof_bytes = Some(vec![0]),
            3 => {
                legacy
                    .grovedb_identity_public_key_hash_proof
                    .as_mut()
                    .unwrap()
                    .grovedb_proof = vec![0]
            }
            4 => {
                legacy.identity_proof_bytes = Some(
                    f.prove(
                        &Drive::full_identity_query(&[0x20; 32], &f.producer.drive.grove_version)
                            .unwrap(),
                    ),
                )
            }
            5 => {
                legacy.identity_proof_bytes =
                    Some(f.prove(&Drive::identity_balance_query(&[0x10; 32])))
            }
            _ => {
                let mut identity = Identity::random_identity(3, Some(16), f.producer).unwrap();
                identity.set_id(Identifier::new([0x30; 32]));
                f.drive
                    .add_new_identity(
                        identity,
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        f.producer,
                    )
                    .unwrap();
                legacy.identity_proof_bytes = Some(
                    f.prove(
                        &Drive::full_identity_query(&[0x10; 32], &f.producer.drive.grove_version)
                            .unwrap(),
                    ),
                );
            }
        }
        assert!(verify(response, f.hash, None).is_err(), "case={case}");
    }
    let mut absence = f.response(true, Some([0xff; 32]));
    let Some(LookupResult::LegacyProof(legacy)) = &mut inner(&mut absence).result else {
        panic!("legacy")
    };
    legacy.identity_proof_bytes = Some(
        f.prove(&Drive::full_identity_query(&[0x10; 32], &f.producer.drive.grove_version).unwrap()),
    );
    assert!(verify(absence, f.hash, Some([0xff; 32])).is_err());
}
#[test]
fn should_verify_nonunique_hash_absence_in_both_formats() {
    for legacy in [true, false] {
        let mut f = Fixture::new(producer_for(legacy));
        f.hash = [9; 20];
        f.identities.clear();
        assert!(verify(f.response(legacy, None), f.hash, None)
            .unwrap()
            .is_none());
    }
}

/// Callers replaying a verified answer get the signed proof back: for a legacy answer
/// that is the index proof, which carries the quorum signature.
#[test]
fn should_return_the_signed_proof_of_either_format() {
    for legacy in [true, false] {
        let f = Fixture::new(producer_for(legacy));
        let mut response = f.response(legacy, None);
        let signed = proof(&mut response).clone();
        let request: GetIdentityByNonUniquePublicKeyHashRequest =
            GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash: f.hash.to_vec(),
                start_after: None,
                prove: true,
            }
            .into();
        let (identity, _, returned) = <Identity as FromProof<
            GetIdentityByNonUniquePublicKeyHashRequest,
        >>::maybe_from_proof_with_metadata(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &Provider,
        )
        .unwrap();
        assert_eq!(identity.as_ref(), Some(&f.identities[0]), "legacy={legacy}");
        assert_eq!(returned, signed, "legacy={legacy}");
    }
}

#[test]
fn should_not_interpret_a_malformed_current_nonunique_proof_as_legacy() {
    let f = Fixture::new(producer_for(false));
    let mut response = f.response(false, None);
    let legacy = f.response(true, None);
    let Some(Version::V0(legacy)) = legacy.version else {
        panic!("legacy")
    };
    let Some(LookupResult::LegacyProof(legacy)) = legacy.result else {
        panic!("legacy")
    };
    proof(&mut response).grovedb_proof = legacy.encode_to_vec();
    assert!(verify(response, f.hash, None).is_err());
}

#[test]
#[ignore = "regenerates deterministic signed SDK compatibility fixtures"]
fn should_export_nonunique_sdk_fixtures() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/identity-discovery");
    std::fs::create_dir_all(&directory).unwrap();
    // The versions on either side of the wire change, fixed so that re-running on a later
    // branch rewrites the same files the SDK tests read.
    for version in [12, 13, 14] {
        let f = Fixture::new(version);
        std::fs::write(directory.join("key-hash.bin"), f.hash).unwrap();
        std::fs::write(
            directory.join("quorum-key.bin"),
            key().public_key().to_bytes(),
        )
        .unwrap();
        for legacy in [true, false] {
            for (page, after) in [None, Some([0x10; 32]), Some([0x20; 32])]
                .into_iter()
                .enumerate()
            {
                let response = f.response(legacy, after);
                if legacy == (version <= 13) {
                    verify(response.clone(), f.hash, after).unwrap();
                } else {
                    assert_format_refused(verify(response.clone(), f.hash, after), "export");
                }
                let name = format!(
                    "pv{version}-{}-{page}.bin",
                    if legacy { "legacy" } else { "current" }
                );
                std::fs::write(directory.join(name), response.encode_to_vec()).unwrap();
            }
        }
    }
    // A version newer than any build, answering in the single-proof format.
    let f = Fixture::new(14);
    let response = f.response_signed_as(false, None, NEWER_PROTOCOL_VERSION);
    verify(response.clone(), f.hash, None).unwrap();
    std::fs::write(
        directory.join("newer-current-0.bin"),
        response.encode_to_vec(),
    )
    .unwrap();
}

#[test]
fn should_reject_unknown_legacy_nonunique_verifier_generation() {
    let f = Fixture::new(13);
    let mut security = PlatformVersion::latest().clone();
    security
        .drive
        .methods
        .verify
        .identity
        .verify_full_identity_by_non_unique_public_key_hash = 255;
    let response = f.response(true, None);
    let Some(Version::V0(v0)) = response.version else {
        panic!("v0")
    };
    let Some(LookupResult::LegacyProof(proof)) = v0.result else {
        panic!("legacy")
    };
    let error = Drive::verify_legacy_identity_by_non_unique_public_key_hash(
        &proof
            .grovedb_identity_public_key_hash_proof
            .unwrap()
            .grovedb_proof,
        proof.identity_proof_bytes.as_deref(),
        f.hash,
        None,
        f.producer,
        &security,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        drive::error::Error::Drive(drive::error::drive::DriveError::UnknownVersionMismatch {
            received: 255,
            ..
        })
    ));
}

#[path = "identity_discovery/document_outcome.rs"]
mod document_outcome;
#[path = "identity_discovery/mainnet.rs"]
mod mainnet;
