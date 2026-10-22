//! Both request versions bind the requested first-after index, complete identity, and signed state.
mod signed_response;

use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::{
    GetIdentityByNonUniquePublicKeyHashRequestV0, GetIdentityByNonUniquePublicKeyHashRequestV1,
};
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
    get_identity_by_non_unique_public_key_hash_response_v0::{
        IdentityProvedResponse, Result as V0Result,
    },
    get_identity_by_non_unique_public_key_hash_response_v1::Result as V1Result,
    GetIdentityByNonUniquePublicKeyHashResponseV0, GetIdentityByNonUniquePublicKeyHashResponseV1,
    Version,
};
use dapi_grpc::platform::v0::{
    get_identity_by_non_unique_public_key_hash_request, GetIdentityByNonUniquePublicKeyHashRequest,
    GetIdentityByNonUniquePublicKeyHashResponse, Proof, ResponseMetadata,
};
use dapi_grpc::Message;
use dash_context_provider::ContextProvider;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::prelude::{Identifier, Identity};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::PathQuery;
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use drive_proof_verifier::{Error, FromProof};
use signed_response::{signed, Provider, NEWER_PROTOCOL_VERSION};
use Lookup::{V0, V1};

/// The request version, which also names the shape of the proved answer: V0 carries
/// separate index and identity proofs, V1 one proof of both.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Lookup {
    V0,
    V1,
}

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
    fn metadata(protocol_version: u32) -> ResponseMetadata {
        ResponseMetadata {
            height: 10,
            core_chain_locked_height: 1,
            time_ms: 1_700_000_000_000,
            protocol_version,
            chain_id: "dash-testnet-51".into(),
            ..Default::default()
        }
    }
    fn response(
        &self,
        lookup: Lookup,
        after: Option<[u8; 32]>,
    ) -> GetIdentityByNonUniquePublicKeyHashResponse {
        self.response_signed_as(lookup, after, self.producer.protocol_version)
    }
    /// The answer this state's producer gives to `lookup`, signed as `protocol_version`.
    fn response_signed_as(
        &self,
        lookup: Lookup,
        after: Option<[u8; 32]>,
        protocol_version: u32,
    ) -> GetIdentityByNonUniquePublicKeyHashResponse {
        let metadata = Self::metadata(protocol_version);
        match lookup {
            V0 => {
                let proof = self
                    .drive
                    .prove_full_identity_by_non_unique_public_key_hash_two_proofs(
                        self.hash,
                        after,
                        None,
                        self.producer,
                    )
                    .unwrap();
                self.two_proof_answer(
                    proof.identity_id_public_key_hash_proof,
                    proof.identity_proof,
                    metadata,
                )
            }
            V1 => {
                let proof = self
                    .drive
                    .prove_full_identity_by_non_unique_public_key_hash(
                        self.hash,
                        after,
                        None,
                        self.producer,
                    )
                    .unwrap();
                GetIdentityByNonUniquePublicKeyHashResponse {
                    version: Some(Version::V1(GetIdentityByNonUniquePublicKeyHashResponseV1 {
                        result: Some(V1Result::Proof(signed(proof, &metadata, self.root()))),
                        metadata: Some(metadata),
                    })),
                }
            }
        }
    }
    /// A V0 answer over the given proofs, signed over this state's root.
    fn two_proof_answer(
        &self,
        index_proof: Vec<u8>,
        identity_proof: Option<Vec<u8>>,
        metadata: ResponseMetadata,
    ) -> GetIdentityByNonUniquePublicKeyHashResponse {
        GetIdentityByNonUniquePublicKeyHashResponse {
            version: Some(Version::V0(GetIdentityByNonUniquePublicKeyHashResponseV0 {
                result: Some(V0Result::Proof(IdentityProvedResponse {
                    grovedb_identity_public_key_hash_proof: Some(signed(
                        index_proof,
                        &metadata,
                        self.root(),
                    )),
                    identity_proof_bytes: identity_proof,
                })),
                metadata: Some(metadata),
            })),
        }
    }
    /// The identity a lookup after `after` must return.
    fn expected(&self, after: Option<[u8; 32]>) -> Option<&Identity> {
        self.identities
            .iter()
            .find(|id| after.is_none_or(|a| id.id().to_buffer() > a))
    }
}
fn request(
    lookup: Lookup,
    hash: [u8; 20],
    after: Option<[u8; 32]>,
) -> GetIdentityByNonUniquePublicKeyHashRequest {
    let (public_key_hash, start_after) = (hash.to_vec(), after.map(|a| a.to_vec()));
    GetIdentityByNonUniquePublicKeyHashRequest {
        version: Some(match lookup {
            V0 => get_identity_by_non_unique_public_key_hash_request::Version::V0(
                GetIdentityByNonUniquePublicKeyHashRequestV0 {
                    public_key_hash,
                    start_after,
                    prove: true,
                },
            ),
            V1 => get_identity_by_non_unique_public_key_hash_request::Version::V1(
                GetIdentityByNonUniquePublicKeyHashRequestV1 {
                    public_key_hash,
                    start_after,
                    prove: true,
                },
            ),
        }),
    }
}
/// The version of a response, which the matching request must have.
fn lookup_of(response: &GetIdentityByNonUniquePublicKeyHashResponse) -> Lookup {
    match response.version {
        Some(Version::V0(_)) => V0,
        Some(Version::V1(_)) => V1,
        None => panic!("response without a version"),
    }
}
/// Verify `response` as the answer to the request of its own version, so every refusal
/// below comes from the proof, the signature or the request, never from the pairing.
fn verify(
    response: GetIdentityByNonUniquePublicKeyHashResponse,
    hash: [u8; 20],
    after: Option<[u8; 32]>,
) -> Result<Option<Identity>, Box<Error>> {
    verify_as(
        lookup_of(&response),
        response,
        hash,
        after,
        PlatformVersion::latest(),
    )
}
fn verify_as(
    lookup: Lookup,
    response: GetIdentityByNonUniquePublicKeyHashResponse,
    hash: [u8; 20],
    after: Option<[u8; 32]>,
    verifier: &PlatformVersion,
) -> Result<Option<Identity>, Box<Error>> {
    // Exercise actual protobuf encoding/decoding before FromProof.
    let response =
        GetIdentityByNonUniquePublicKeyHashResponse::decode(response.encode_to_vec().as_slice())
            .unwrap();
    <Identity as FromProof<GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof(
        request(lookup, hash, after),
        response,
        Network::Testnet,
        verifier,
        &Provider,
    )
    .map_err(Box::new)
}
fn signed_proof(response: &mut GetIdentityByNonUniquePublicKeyHashResponse) -> &mut Proof {
    match &mut response.version {
        Some(Version::V0(v0)) => match v0.result.as_mut() {
            Some(V0Result::Proof(proved)) => proved
                .grovedb_identity_public_key_hash_proof
                .as_mut()
                .unwrap(),
            _ => panic!("proved V0"),
        },
        Some(Version::V1(v1)) => match v1.result.as_mut() {
            Some(V1Result::Proof(proof)) => proof,
            _ => panic!("proved V1"),
        },
        None => panic!("version"),
    }
}
fn metadata(response: &mut GetIdentityByNonUniquePublicKeyHashResponse) -> &mut ResponseMetadata {
    match &mut response.version {
        Some(Version::V0(v0)) => v0.metadata.as_mut().unwrap(),
        Some(Version::V1(v1)) => v1.metadata.as_mut().unwrap(),
        None => panic!("version"),
    }
}
fn two_proofs(
    response: &mut GetIdentityByNonUniquePublicKeyHashResponse,
) -> &mut IdentityProvedResponse {
    let Some(Version::V0(v0)) = &mut response.version else {
        panic!("V0")
    };
    let Some(V0Result::Proof(proved)) = &mut v0.result else {
        panic!("proved V0")
    };
    proved
}
fn assert_invalid_signature(result: Result<Option<Identity>, Box<Error>>, context: &str) {
    let error = result.expect_err(context);
    assert!(
        matches!(*error, Error::InvalidSignature { .. }),
        "{context}: {error}"
    );
}
/// A proof that does not prove the rebuilt query: GroveDB refuses it, or one of the
/// strict two-proof checks does.
fn assert_proof_refused(result: Result<Option<Identity>, Box<Error>>, context: &str) {
    let error = result.expect_err(context);
    assert!(
        matches!(
            *error,
            Error::GroveDBError { .. } | Error::DriveError { .. }
        ),
        "{context}: {error}"
    );
}
/// The versions this test signs answers with: every version that serves V0, the first
/// that serves V1, and one newer than this build.
fn signed_versions(lookup: Lookup) -> Vec<u32> {
    let latest = PlatformVersion::latest().protocol_version;
    match lookup {
        V0 => vec![12, 13, latest, latest + 1],
        // No honest node answers V1 below 14, but such an answer is still a sound proof
        // of the signed state, so it verifies.
        V1 => vec![13, latest, latest + 1],
    }
}
/// The newest table this build knows for a signed version.
fn producer_for(signed: u32) -> u32 {
    signed.min(PlatformVersion::latest().protocol_version)
}

#[test]
fn should_verify_nonunique_pagination_in_both_versions() {
    for lookup in [V0, V1] {
        for signed in signed_versions(lookup) {
            let f = Fixture::new(producer_for(signed));
            for verifier in [PlatformVersion::get(13).unwrap(), PlatformVersion::latest()] {
                for after in [None, Some([0x10; 32]), Some([0x20; 32])] {
                    let response = f.response_signed_as(lookup, after, signed);
                    assert_eq!(
                        verify_as(lookup, response, f.hash, after, verifier)
                            .unwrap()
                            .as_ref(),
                        f.expected(after),
                        "{lookup:?} signed PV{signed}, verifier PV{}, after={after:?}",
                        verifier.protocol_version
                    );
                }
            }
        }
    }
}

/// The request version chooses the shape. A correctly signed, otherwise valid answer of
/// the other version is refused, never reinterpreted, whatever version signed it.
#[test]
fn should_refuse_a_nonunique_answer_of_the_other_version() {
    for (asked, answered) in [(V0, V1), (V1, V0)] {
        for signed in [13, PlatformVersion::latest().protocol_version] {
            let f = Fixture::new(signed);
            for after in [None, Some([0x10; 32]), Some([0x20; 32])] {
                let response = f.response(answered, after);
                assert!(verify(response.clone(), f.hash, after).is_ok());
                let error = verify_as(asked, response, f.hash, after, PlatformVersion::latest())
                    .expect_err("an answer of the other version");
                assert!(
                    matches!(&*error, Error::ResponseDecodeError { error } if error.contains("does not match request version")),
                    "{asked:?} asked, {answered:?} answered, PV{signed}: {error}"
                );
            }
        }
    }
}

/// A V0 answer whose result arrives at the field-4 number of the 5.0 prereleases has no
/// result to this client.
#[test]
fn should_find_no_proof_in_a_v0_answer_at_field_4() {
    let f = Fixture::new(PlatformVersion::latest().protocol_version);
    let mut bytes = f.response(V1, None).encode_to_vec();
    // Outer version tag 2 (V1) becomes 1 (V0); inner result tag 2 (Proof) becomes 4.
    assert_eq!(bytes[0], 0x12);
    bytes[0] = 0x0a;
    let inner = fields_start(&bytes);
    assert_eq!(bytes[inner], 0x12);
    bytes[inner] = 0x22;
    let response = GetIdentityByNonUniquePublicKeyHashResponse::decode(bytes.as_slice()).unwrap();
    let Some(Version::V0(v0)) = &response.version else {
        panic!("V0")
    };
    assert!(v0.result.is_none());
    assert!(matches!(
        *verify_as(V0, response, f.hash, None, PlatformVersion::latest()).unwrap_err(),
        Error::NoProofInResult
    ));
}
/// Offset of the first field inside the outer length-delimited version field.
fn fields_start(bytes: &[u8]) -> usize {
    let mut offset = 1;
    while bytes[offset] & 0x80 != 0 {
        offset += 1;
    }
    offset + 1
}

/// A version newer than this build is read with the newest known rules, still bound to
/// the signed root.
#[test]
fn should_read_nonunique_answers_signed_by_an_unknown_newer_version_with_newest_rules() {
    let newer = PlatformVersion::latest().protocol_version + 1;
    let f = Fixture::new(PlatformVersion::latest().protocol_version);
    for lookup in [V0, V1] {
        for after in [None, Some([0x10; 32]), Some([0x20; 32])] {
            assert_eq!(
                verify(f.response_signed_as(lookup, after, newer), f.hash, after)
                    .unwrap()
                    .as_ref(),
                f.expected(after),
                "{lookup:?} PV{newer}, after={after:?}"
            );
        }
        let mut forged = f.response_signed_as(lookup, None, newer);
        metadata(&mut forged).protocol_version = newer + 1;
        assert_invalid_signature(verify(forged, f.hash, None), &format!("{lookup:?}"));
    }
}

#[test]
fn should_reject_nonunique_wrong_hash_cursor_signature_and_metadata_tampering() {
    let latest = PlatformVersion::latest().protocol_version;
    for lookup in [V0, V1] {
        for signed in [13, latest] {
            let f = Fixture::new(signed);
            let context = format!("{lookup:?} PV{signed}");
            let response = f.response(lookup, None);
            assert_eq!(
                verify(response.clone(), f.hash, None).unwrap().as_ref(),
                f.expected(None),
                "{context}: untampered control"
            );
            assert_proof_refused(
                verify(response.clone(), [9; 20], None),
                &format!("{context}: hash"),
            );
            assert_proof_refused(
                verify(response.clone(), f.hash, Some([0x10; 32])),
                &format!("{context}: cursor"),
            );
            assert_proof_refused(
                verify(f.response(lookup, Some([0x10; 32])), f.hash, None),
                &format!("{context}: answer for another cursor"),
            );
            for field in 0..8 {
                let mut changed = response.clone();
                let m = metadata(&mut changed);
                match field {
                    0 => m.height += 1,
                    1 => m.time_ms += 1,
                    2 => m.core_chain_locked_height += 1,
                    // Another version building the same queries: only the signature refuses it.
                    3 => m.protocol_version = if signed == 13 { 12 } else { latest + 1 },
                    4 => m.chain_id.push('x'),
                    5 => m.protocol_version = u32::MAX,
                    6 => m.protocol_version = latest + 1,
                    _ => signed_proof(&mut changed).signature[0] ^= 1,
                }
                let context = format!("{context}, field {field}");
                let result = verify(changed, f.hash, None);
                if field == 7 {
                    // A flipped bit may not decode as a signature at all.
                    let error = result.expect_err(&context);
                    assert!(
                        matches!(
                            *error,
                            Error::InvalidSignature { .. }
                                | Error::InvalidSignatureFormat { .. }
                                | Error::SignatureVerificationError { .. }
                        ),
                        "{context}: {error}"
                    );
                } else {
                    assert_invalid_signature(result, &context);
                }
            }
            let mut unknown = response.clone();
            metadata(&mut unknown).protocol_version = 0;
            let error = verify(unknown, f.hash, None).expect_err("protocol version 0");
            assert!(
                matches!(&*error, Error::ResponseDecodeError { error } if error.contains("unsupported proof query protocol version 0")),
                "{lookup:?}: {error}"
            );
        }
    }
}

/// Moving an answer across the protocol 14 boundary needs the signature of the version
/// it claims: a two-proof answer made at 13 relabeled 14, and a single proof made at 14
/// relabeled 13.
#[test]
fn should_refuse_nonunique_answers_relabeled_across_protocol_14() {
    let latest = PlatformVersion::latest().protocol_version;
    for (lookup, made, claimed) in [
        (V0, 13, latest),
        (V0, latest, 13),
        (V1, latest, 13),
        (V1, 13, latest),
    ] {
        let f = Fixture::new(made);
        let mut response = f.response(lookup, None);
        metadata(&mut response).protocol_version = claimed;
        assert_invalid_signature(
            verify(response, f.hash, None),
            &format!("{lookup:?} made at {made}, claimed {claimed}"),
        );
    }
}

/// A true absence proof for a narrower range or another key cannot answer a wider
/// request: the request's own hash and cursor rebuild the query.
#[test]
fn should_refuse_a_substituted_nonunique_absence_proof() {
    let latest = PlatformVersion::latest().protocol_version;
    for lookup in [V0, V1] {
        for signed in [13, latest] {
            let f = Fixture::new(signed);
            let past_all = f.response(lookup, Some([0x20; 32]));
            assert_eq!(
                verify(past_all.clone(), f.hash, Some([0x20; 32])).unwrap(),
                None
            );
            for after in [None, Some([0x10; 32])] {
                assert_proof_refused(
                    verify(past_all.clone(), f.hash, after),
                    &format!("{lookup:?} PV{signed}: cursor 0x20 answer for {after:?}"),
                );
            }
            let mut other = Fixture::new(signed);
            other.hash = [9; 20];
            let other_key = other.response(lookup, None);
            assert_eq!(verify(other_key.clone(), [9; 20], None).unwrap(), None);
            // The other fixture holds the same identities, so its root signs this state too.
            assert_eq!(other.root(), f.root());
            for after in [None, Some([0x10; 32])] {
                assert_proof_refused(
                    verify(other_key.clone(), f.hash, after),
                    &format!("{lookup:?} PV{signed}: another key's absence for {after:?}"),
                );
            }
        }
    }
}

#[test]
fn should_reject_unbound_incomplete_and_malformed_two_proof_answers() {
    for signed in [
        13,
        PlatformVersion::latest().protocol_version,
        PlatformVersion::latest().protocol_version + 1,
    ] {
        let f = Fixture::new(producer_for(signed));
        let control = f.response_signed_as(V0, None, signed);
        assert_eq!(
            verify(control.clone(), f.hash, None).unwrap().as_ref(),
            f.expected(None),
            "PV{signed}: untampered control"
        );
        let first = f.identities[0].id().to_buffer();
        let second = f.identities[1].id().to_buffer();
        let full_identity = |id: &[u8; 32]| {
            f.prove(&Drive::full_identity_query(id, &f.producer.drive.grove_version).unwrap())
        };
        for case in 0..7 {
            let mut response = control.clone();
            let proved = two_proofs(&mut response);
            match case {
                // The index names an identity, but its proof is missing.
                0 => proved.identity_proof_bytes = None,
                // Unsupported or malformed identity proof envelopes.
                1 => proved.identity_proof_bytes = Some(vec![1]),
                2 => proved.identity_proof_bytes = Some(vec![0]),
                // Malformed index proof under the signature.
                3 => {
                    proved
                        .grovedb_identity_public_key_hash_proof
                        .as_mut()
                        .unwrap()
                        .grovedb_proof = vec![0]
                }
                // Another identity's complete proof.
                4 => proved.identity_proof_bytes = Some(full_identity(&second)),
                // A proof of only part of the identity.
                5 => {
                    proved.identity_proof_bytes =
                        Some(f.prove(&Drive::identity_balance_query(&first)))
                }
                // An identity proof of another state.
                _ => {
                    let mut later = Identity::random_identity(3, Some(16), f.producer).unwrap();
                    later.set_id(Identifier::new([0x30; 32]));
                    let g = Fixture::new(producer_for(signed));
                    g.drive
                        .add_new_identity(
                            later,
                            false,
                            &BlockInfo::default(),
                            true,
                            None,
                            g.producer,
                        )
                        .unwrap();
                    proved.identity_proof_bytes = Some(
                        g.prove(
                            &Drive::full_identity_query(&first, &g.producer.drive.grove_version)
                                .unwrap(),
                        ),
                    );
                }
            }
            let error =
                verify(response, f.hash, None).expect_err(&format!("PV{signed}, case {case}"));
            let refused_by = match &*error {
                Error::DriveError { error } | Error::GroveDBError { error, .. } => error.clone(),
                Error::UnsupportedGroveDBProofVersion { proof, version, .. } => {
                    format!("unsupported {proof} envelope {version}")
                }
                other => panic!("PV{signed}, case {case}: unexpected {other}"),
            };
            let expected = [
                "references an identity without its proof",
                "unable to decode proof",
                "unsupported full identity proof envelope 0",
                "unsupported proof envelope 0",
                "invalid proof",
                "invalid proof",
                "index and identity proof roots differ",
            ][case];
            assert!(
                refused_by.contains(expected),
                "PV{signed}, case {case}: {refused_by}"
            );
        }

        // An identity proof sent with an absence answer.
        let mut absence = f.response_signed_as(V0, Some([0xff; 32]), signed);
        assert_eq!(
            verify(absence.clone(), f.hash, Some([0xff; 32])).unwrap(),
            None
        );
        two_proofs(&mut absence).identity_proof_bytes = Some(full_identity(&first));
        assert_proof_refused(
            verify(absence, f.hash, Some([0xff; 32])),
            &format!("PV{signed}: absence with identity"),
        );

        // An index proof of more than the first entry after the cursor.
        let mut query = Drive::identity_id_by_non_unique_public_key_hash_query(f.hash, None);
        query.query.limit = Some(2);
        let extra = f.two_proof_answer(
            f.prove(&query),
            Some(full_identity(&first)),
            Fixture::metadata(signed),
        );
        let error = verify(extra, f.hash, None).expect_err("an extra index entry");
        assert!(
            matches!(&*error, Error::GroveDBError { error, .. } if error.contains("more data than limit")),
            "PV{signed}: {error}"
        );

        // An index proof that skips the first entry, paired with the second identity.
        let mut skipped =
            Drive::identity_id_by_non_unique_public_key_hash_query(f.hash, Some(first));
        skipped.query.limit = Some(1);
        let skipped = f.two_proof_answer(
            f.prove(&skipped),
            Some(full_identity(&second)),
            Fixture::metadata(signed),
        );
        assert_proof_refused(
            verify(skipped, f.hash, None),
            &format!("PV{signed}: skipped first entry"),
        );
    }
}

#[test]
fn should_verify_nonunique_hash_absence_in_both_versions() {
    for lookup in [V0, V1] {
        for signed in [13, PlatformVersion::latest().protocol_version] {
            let mut f = Fixture::new(signed);
            f.hash = [9; 20];
            f.identities.clear();
            let response = f.response(lookup, None);
            if lookup == V0 {
                assert_eq!(two_proofs(&mut response.clone()).identity_proof_bytes, None);
            }
            assert!(verify(response, f.hash, None).unwrap().is_none());
        }
    }
}

/// Callers replaying a verified answer get the signed proof back: for V0 that is the index
/// proof, which carries the quorum signature.
#[test]
fn should_return_the_signed_proof_of_either_version() {
    for lookup in [V0, V1] {
        let f = Fixture::new(PlatformVersion::latest().protocol_version);
        let mut response = f.response(lookup, None);
        let signed = signed_proof(&mut response).clone();
        let (identity, _, returned) = <Identity as FromProof<
            GetIdentityByNonUniquePublicKeyHashRequest,
        >>::maybe_from_proof_with_metadata(
            request(lookup, f.hash, None),
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &Provider,
        )
        .unwrap();
        assert_eq!(identity.as_ref(), Some(&f.identities[0]), "{lookup:?}");
        assert_eq!(returned, signed, "{lookup:?}");
    }
}

#[test]
fn should_not_read_two_proofs_packed_into_a_v1_proof() {
    let f = Fixture::new(PlatformVersion::latest().protocol_version);
    let mut response = f.response(V1, None);
    let mut two = f.response(V0, None);
    let packed = two_proofs(&mut two).encode_to_vec();
    signed_proof(&mut response).grovedb_proof = packed;
    // The packed message is not a GroveDB proof, so it does not decode as one.
    let error = verify(response, f.hash, None).expect_err("two proofs packed into V1");
    assert!(
        matches!(&*error, Error::GroveDBError { error, .. } if error.contains("unable to decode proof")),
        "{error}"
    );
}

/// Rewrites the committed vectors, so running the suite with `--ignored` skips it
/// unless `DASH_SDK_EXPORT_NONUNIQUE_VECTORS=1` is set.
#[test]
#[ignore = "requires DASH_SDK_EXPORT_NONUNIQUE_VECTORS=1; regenerates deterministic signed SDK compatibility fixtures"]
fn should_export_nonunique_sdk_fixtures() {
    if std::env::var("DASH_SDK_EXPORT_NONUNIQUE_VECTORS").as_deref() != Ok("1") {
        eprintln!("skipped: set DASH_SDK_EXPORT_NONUNIQUE_VECTORS=1 to rewrite the vectors");
        return;
    }
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/identity-discovery");
    std::fs::create_dir_all(&directory).unwrap();
    // The versions on either side of protocol 14, fixed so that re-running on a later
    // branch rewrites the same files the SDK tests read.
    for (lookup, versions) in [(V0, vec![12, 13, 14]), (V1, vec![14])] {
        for version in versions {
            let f = Fixture::new(version);
            std::fs::write(directory.join("key-hash.bin"), f.hash).unwrap();
            std::fs::write(
                directory.join("quorum-key.bin"),
                Provider.get_quorum_public_key(106, [2; 32], 0).unwrap(),
            )
            .unwrap();
            for (page, after) in [None, Some([0x10; 32]), Some([0x20; 32])]
                .into_iter()
                .enumerate()
            {
                let response = f.response(lookup, after);
                assert_eq!(
                    verify(response.clone(), f.hash, after).unwrap().as_ref(),
                    f.expected(after)
                );
                let name = format!("pv{version}-{}-{page}.bin", lookup_name(lookup));
                std::fs::write(directory.join(name), response.encode_to_vec()).unwrap();
            }
        }
    }
    // A version newer than any build, in each request version.
    let f = Fixture::new(14);
    for lookup in [V0, V1] {
        let response = f.response_signed_as(lookup, None, NEWER_PROTOCOL_VERSION);
        verify(response.clone(), f.hash, None).unwrap();
        std::fs::write(
            directory.join(format!("newer-{}-0.bin", lookup_name(lookup))),
            response.encode_to_vec(),
        )
        .unwrap();
    }
}
fn lookup_name(lookup: Lookup) -> &'static str {
    match lookup {
        V0 => "v0",
        V1 => "v1",
    }
}

#[path = "identity_discovery/mainnet.rs"]
mod mainnet;
