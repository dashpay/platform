use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    #[allow(clippy::type_complexity)]
    pub(super) fn verify_identity_balance_revision_and_addresses_from_inputs_v0<
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
        let (root_hash_identity, balance_revision) =
            Self::verify_identity_balance_and_revision_for_identity_id(
                proof,
                identity_id,
                verify_subset_of_proof,
                platform_version,
            )?;

        let (root_hash_addresses, address_balances): (
            RootHash,
            BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
        ) = Self::verify_addresses_infos(proof, addresses, verify_subset_of_proof, platform_version)?;

        if root_hash_identity != root_hash_addresses {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "proof is expected to have same root hash for identity balance and address infos"
                    .to_string(),
            )));
        }

        Ok((root_hash_identity, balance_revision, address_balances))
    }
}
