mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    #[allow(clippy::type_complexity)]
    /// Verifies an identity's balance/revision along with the balances of a set of input addresses.
    ///
    /// This helper is primarily used by identity top-up transitions that spend balances from
    /// Platform addresses. It ensures the proof contains all expected information and that the
    /// subset verifications share the same root hash.
    ///
    /// Returns a tuple consisting of:
    /// * `RootHash` – the GroveDB root hash shared by the identity and address proofs.
    /// * `Option<(Credits, u64)>` – the identity balance (credits) and revision if the identity exists.
    /// * `BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>` – the proved nonce/balance for
    ///   each queried address (or `None` if the address was absent).
    pub fn verify_identity_balance_revision_and_addresses_from_inputs<
        'a,
        I: IntoIterator<Item = &'a PlatformAddress>,
    >(
        proof: &[u8],
        identity_id: [u8; 32],
        addresses: I,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            RootHash,
            Option<(Credits, u64)>,
            BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
        ),
        Error,
    > {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_balance_revision_and_addresses_from_inputs
        {
            0 => Self::verify_identity_balance_revision_and_addresses_from_inputs_v0(
                proof,
                identity_id,
                addresses,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_balance_revision_and_addresses_from_inputs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_identity_balance_revision_and_addresses_from_inputs_unknown_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_balance_revision_and_addresses_from_inputs = 255;

        let addresses: Vec<PlatformAddress> = vec![];
        let result = Drive::verify_identity_balance_revision_and_addresses_from_inputs(
            &[],
            [0u8; 32],
            addresses.iter(),
            false,
            &platform_version,
        );

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_identity_balance_revision_and_addresses_from_inputs" && known_versions == vec![0] && received == 255
            )
        );
    }
}
