use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Proves the Identity's contract nonce from the backing store
    pub(super) fn prove_identity_contract_nonce_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let identity_contract_path = Self::identity_contract_nonce_query(identity_id, contract_id);
        self.grove_get_proved_path_query(
            &identity_contract_path,
            transaction,
            &mut vec![],
            drive_version,
        )
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    // NOTE: no proof-verifier roundtrip here. Unlike
    // `Drive::verify_address_info`, there is no public verifier helper for
    // "identity-contract-nonce" proofs — callers decode via the generic
    // `GroveDb::verify_*` API on the path_query used to produce the proof,
    // which requires reconstructing the query shape outside this module. The
    // present tests therefore exercise prove-path correctness by comparing
    // absent-vs-present proof bytes (they must differ) and asserting a
    // non-empty blob; a verifier roundtrip would be more meaningful but
    // belongs in a higher-level integration test that already owns the
    // verifier plumbing.
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use platform_version::version::PlatformVersion;

    /// Producing a proof for an identity/contract pair that has never been
    /// merged returns valid non-empty proof bytes (an absence proof).
    #[test]
    fn prove_absent_nonce_returns_proof_bytes() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::first();
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("init");
        let identity = Identity::random_identity(5, Some(42), platform_version).expect("rand id");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("insert");

        let proof = drive
            .prove_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                [0u8; 32],
                None,
                &platform_version.drive,
            )
            .expect("prove absent");
        assert!(!proof.is_empty());
    }

    /// After merging a nonce, the proof must be non-empty and different from
    /// the absence-proof case (ensures the prove path actually reads state).
    #[test]
    fn prove_present_nonce_differs_from_absent_proof() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::first();
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("init");
        let identity = Identity::random_identity(5, Some(42), platform_version).expect("rand id");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("insert");
        let contract_id = [5u8; 32];

        let proof_before = drive
            .prove_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                None,
                &platform_version.drive,
            )
            .expect("prove before merge");

        drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("merge");

        let proof_after = drive
            .prove_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                None,
                &platform_version.drive,
            )
            .expect("prove after merge");

        // After state change the proof must differ.
        assert_ne!(proof_before, proof_after);
    }
}
