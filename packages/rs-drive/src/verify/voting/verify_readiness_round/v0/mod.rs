use crate::drive::votes::paths::{
    readiness_contract_tree_path_vec, readiness_round_tree_path_vec,
    READINESS_CURRENT_ROUND_POINTER_KEY, READINESS_ROUND_RECORD_KEY,
    READINESS_ROUND_REPORTS_TREE_KEY,
};
use crate::drive::votes::readiness::queries::{
    readiness_round_path_query, readiness_round_pointer_path_query,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::voting::verify_readiness_round::VerifiedReadinessRound;
use crate::verify::RootHash;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::voting::readiness::round::ReadinessRound;
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_readiness_round_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<VerifiedReadinessRound>), Error> {
        let grove_version = &platform_version.drive.grove_version;

        // The pointer first; a proof of a round without a round is a proof of absence. The
        // prover's query carries no limit (a limited query cannot land at a merged root), so
        // the absence check sets the one-key limit on its own copy. Whether or not the
        // caller verifies a subset, the pointer is verified as a subset: the full proof also
        // carries the round elements when the round exists, and those are pinned below.
        let mut pointer_query = readiness_round_pointer_path_query(contract_id);
        pointer_query.query.limit = Some(1);
        let _ = verify_subset_of_proof;
        let (root_hash, mut pointer_values) =
            GroveDb::verify_subset_query_with_absence_proof(proof, &pointer_query, grove_version)?;
        if pointer_values.len() != 1 {
            return Err(Error::Proof(ProofError::TooManyElements(
                "expected one readiness round pointer",
            )));
        }
        let (path, key, maybe_pointer) = pointer_values.remove(0);
        if path != readiness_contract_tree_path_vec(contract_id)
            || key != [READINESS_CURRENT_ROUND_POINTER_KEY as u8]
        {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we did not get back the readiness round pointer".to_string(),
            )));
        }
        let round_id: [u8; 32] = match maybe_pointer {
            None => return Ok((root_hash, None)),
            Some(Element::Item(bytes, _)) => bytes.try_into().map_err(|_| {
                Error::Proof(ProofError::CorruptedProof(
                    "readiness round pointer is not 32 bytes".to_string(),
                ))
            })?,
            Some(_) => {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "readiness round pointer was present but was not an item".to_string(),
                )))
            }
        };

        let round_query = readiness_round_path_query(contract_id, round_id);
        let (round_root_hash, round_values) =
            GroveDb::verify_subset_query(proof, &round_query, grove_version)?;
        if round_root_hash != root_hash {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "the readiness round proof does not share the pointer's root".to_string(),
            )));
        }
        let expected_path = readiness_round_tree_path_vec(contract_id, round_id);
        let mut round = None;
        let mut raw_count = None;
        for (path, key, maybe_element) in round_values {
            if path != expected_path {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct readiness round path"
                        .to_string(),
                )));
            }
            match (key.as_slice(), maybe_element) {
                ([READINESS_ROUND_RECORD_KEY], Some(Element::Item(bytes, _))) => {
                    round = Some(ReadinessRound::deserialize_from_bytes_untrusted(&bytes)?);
                }
                ([READINESS_ROUND_REPORTS_TREE_KEY], Some(Element::CountTree(_, count, _))) => {
                    raw_count = Some(count);
                }
                _ => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "unexpected element in the readiness round proof".to_string(),
                    )))
                }
            }
        }
        match (round, raw_count) {
            (Some(round), Some(raw_count)) => {
                if round.round_id() != round_id || round.contract_id().as_bytes() != &contract_id {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "the readiness round record does not match the pointer".to_string(),
                    )));
                }
                Ok((root_hash, Some(VerifiedReadinessRound { round, raw_count })))
            }
            _ => Err(Error::Proof(ProofError::CorruptedProof(
                "the readiness round proof lacks the record or the reports tree".to_string(),
            ))),
        }
    }
}
