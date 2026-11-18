mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use crate::verify::RootHash;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of one asset-lock outpoint, including a not-present result.
    pub fn verify_asset_lock_outpoint_info(
        proof: &[u8],
        outpoint: &Bytes36,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StoredAssetLockInfo), Error> {
        require_supported_grovedb_proof_envelope(proof, "asset-lock outpoint proof")?;
        match platform_version
            .drive
            .methods
            .verify
            .asset_lock
            .verify_asset_lock_outpoint_info
        {
            0 => Self::verify_asset_lock_outpoint_info_v0(proof, outpoint, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_asset_lock_outpoint_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::platform_serialization::PLATFORM_BINCODE_CONFIG;
    use dpp::platform_value::Bytes36;

    #[test]
    fn should_reject_legacy_grovedb_proof_envelopes_at_public_entry_point() {
        let proof = bincode::encode_to_vec(0u32, PLATFORM_BINCODE_CONFIG)
            .expect("encode legacy proof envelope");
        let error = Drive::verify_asset_lock_outpoint_info(
            &proof,
            &Bytes36::from([0; 36]),
            &PlatformVersion::latest(),
        )
        .expect_err("legacy proof envelope must be rejected");

        assert!(matches!(
            error,
            Error::Proof(
                crate::error::proof::ProofError::UnsupportedGroveDBProofEnvelopeVersion {
                    version: 0,
                    ..
                }
            )
        ));
    }
}
