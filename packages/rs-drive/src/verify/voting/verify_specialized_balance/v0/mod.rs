use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_voting_path_vec;
use grovedb::{GroveDb, PathQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the balance of an identity by their identity ID.
    ///
    /// `verify_subset_of_proof` is used to indicate if we want to verify a subset of a bigger proof.
    /// For example, if the proof can prove the balance and the revision, but here we are only interested
    /// in verifying the balance.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    /// - `verify_subset_of_proof`: A boolean indicating whether we are verifying a subset of a larger proof.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option<u64>`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<u64>` represents the balance of the user's identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid balance.
    /// - The proved key value is not for the correct path or key in balances.
    /// - More than one balance is found.
    ///
    pub(super) fn verify_specialized_balance_v0(
        proof: &[u8],
        balance_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        let balance_path = prefunded_specialized_balances_for_voting_path_vec();
        let mut path_query = PathQuery::new_single_key(balance_path.clone(), balance_id.to_vec());
        path_query.query.limit = Some(1);
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
            let (path, key, maybe_element) = &proved_key_values.remove(0);
            if path != &balance_path {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path in balances".to_string(),
                )));
            }
            if key != &balance_id {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct key in balances".to_string(),
                )));
            }

            let signed_balance = maybe_element
                .as_ref()
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
            Ok((root_hash, signed_balance))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one specialized balance",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_specialized_balance_present() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let balance_id = Identifier::random();
        let amount = 500u64;

        drive
            .add_prefunded_specialized_balance(
                balance_id,
                amount,
                None,
                platform_version,
            )
            .expect("expected to add prefunded specialized balance");

        let proof = drive
            .prove_prefunded_specialized_balance(
                balance_id.to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to get proof");

        let (_, balance) = Drive::verify_specialized_balance(
            proof.as_slice(),
            balance_id.to_buffer(),
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(balance, Some(amount));
    }

    #[test]
    fn should_prove_and_verify_absent_specialized_balance() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let non_existent_balance_id = [0u8; 32];

        let proof = drive
            .prove_prefunded_specialized_balance(
                non_existent_balance_id,
                None,
                platform_version,
            )
            .expect("expected to get proof");

        let (_, balance) = Drive::verify_specialized_balance(
            proof.as_slice(),
            non_existent_balance_id,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(balance, None);
    }
}
