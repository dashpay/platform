use crate::drive::Drive;
use grovedb::Element::Item;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use grovedb::GroveDb;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;

impl Drive {
    pub(super) fn verify_token_perpetual_distribution_last_paid_time_v0(
        proof: &[u8],
        token_id: [u8; 32],
        identity_id: [u8; 32],
        distribution_type: &RewardDistributionType,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<RewardDistributionMoment>), Error> {
        let path_query =
            Drive::perpetual_distribution_last_paid_moment_query(token_id, identity_id);
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
        if proved_key_values.len() == 1 {
            let proved_key_value = proved_key_values.remove(0);
            match proved_key_value.2 {
                Some(Item(value, ..)) => {
                    let moment = distribution_type.moment_from_bytes(&value).map_err(|e| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "Moment should be specific amount of bytes: {}",
                            e
                        )))
                    })?;
                    Ok((root_hash, Some(moment)))
                }
                None => Ok((root_hash, None)),
                _ => Err(Error::Proof(ProofError::IncorrectValueSize(
                    "proof did not point to an item",
                ))),
            }
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: 1,
                got: proved_key_values.len(),
            }))
        }
    }
}
