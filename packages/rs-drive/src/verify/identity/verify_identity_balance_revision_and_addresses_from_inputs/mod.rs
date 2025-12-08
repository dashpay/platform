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
