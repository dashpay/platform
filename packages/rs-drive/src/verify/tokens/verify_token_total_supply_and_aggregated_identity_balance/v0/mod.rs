use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
use dpp::prelude::Identifier;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_token_total_supply_and_aggregated_identity_balance_v0(
        proof: &[u8],
        token_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, TotalSingleTokenBalance), Error> {
        let path_query = Self::token_total_supply_and_aggregated_identity_balances_query(
            token_id,
            platform_version,
        )?;
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
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
        if proved_key_values.len() == 2 {
            let (
                _aggregated_identity_balances_path,
                _aggregated_identity_balances_key,
                Some(aggregated_identity_balances_element),
            ) = proved_key_values.first().unwrap()
            else {
                return Err(Error::Proof(ProofError::UnexpectedResultProof(format!(
                    "Token {} most likely does not exist",
                    Identifier::new(token_id)
                ))));
            };
            let (_total_supply_path, _total_supply_key, Some(total_supply_element)) =
                proved_key_values.get(1).unwrap()
            else {
                return Err(Error::Proof(ProofError::UnexpectedResultProof(format!(
                    "Token {} has no known supply",
                    Identifier::new(token_id)
                ))));
            };

            Ok((
                root_hash,
                TotalSingleTokenBalance {
                    token_supply: total_supply_element.as_sum_item_value()?,
                    aggregated_token_account_balances: aggregated_identity_balances_element
                        .as_sum_tree_value()?,
                },
            ))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: 2,
                got: proved_key_values.len(),
            }))
        }
    }
}
