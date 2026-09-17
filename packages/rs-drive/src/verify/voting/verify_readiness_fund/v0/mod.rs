use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_readiness_path_vec;
use crate::drive::votes::readiness::queries::readiness_fund_path_query;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_readiness_fund_v0(
        proof: &[u8],
        fund_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        let path_query = readiness_fund_path_query(fund_id);
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
                "expected one readiness fund",
            )));
        }
        let (path, key, maybe_element) = proved_key_values.remove(0);
        if path != prefunded_specialized_balances_for_readiness_path_vec() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we did not get back an element for the correct path in readiness funds"
                    .to_string(),
            )));
        }
        if key != fund_id {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we did not get back an element for the correct key in readiness funds"
                    .to_string(),
            )));
        }
        let balance = maybe_element
            .map(|element| {
                element
                    .as_sum_item_value()
                    .map_err(Error::from)?
                    .try_into()
                    .map_err(|_| {
                        Error::Proof(ProofError::IncorrectValueSize("value size is incorrect"))
                    })
            })
            .transpose()?;
        Ok((root_hash, balance))
    }
}
