use crate::drive::votes::paths::readiness_round_reports_tree_path_vec;
use crate::drive::votes::readiness::queries::readiness_report_path_query;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::voting::readiness::report_record::ReadinessReportRecord;
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_readiness_report_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        round_id: [u8; 32],
        pro_tx_hash: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<ReadinessReportRecord>), Error> {
        let path_query = readiness_report_path_query(contract_id, round_id, pro_tx_hash);
        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::TooManyElements(
                "expected one readiness report",
            )));
        }
        let (path, key, maybe_element) = proved_key_values.remove(0);
        if path != readiness_round_reports_tree_path_vec(contract_id, round_id) {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we did not get back an element for the correct path in readiness reports"
                    .to_string(),
            )));
        }
        if key != pro_tx_hash {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we did not get back an element for the correct key in readiness reports"
                    .to_string(),
            )));
        }
        let record = match maybe_element {
            None => None,
            Some(Element::Item(bytes, _)) => Some(
                ReadinessReportRecord::deserialize_from_bytes_untrusted(&bytes)?,
            ),
            Some(_) => {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "readiness report was present but was not an item".to_string(),
                )))
            }
        };
        Ok((root_hash, record))
    }
}
