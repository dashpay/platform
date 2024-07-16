use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;

mod v0;

impl Drive {
    /// Verifies that the contract is included in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `contract_known_keeps_history`: An optional boolean indicating whether the contract keeps a history.
    /// - `is_proof_subset`: A boolean indicating whether to verify a subset of a larger proof.
    /// - `in_multiple_contract_proof_form`: If the contract proof was made by proving many contracts, the form
    /// of the proof will be different. We will be querying the contract id with a translation to 0 for non
    /// historical and 0/0 for historical contracts. When you query a single contract you query directly on the item
    /// 0 under the contract id you care about.
    /// - `contract_id`: The contract's unique identifier.
    /// - `platform_version`: the platform version,
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<DataContract>`. The `Option<DataContract>`
    /// represents the verified contract if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    pub fn verify_contract(
        proof: &[u8],
        contract_known_keeps_history: Option<bool>,
        is_proof_subset: bool,
        in_multiple_contract_proof_form: bool,
        contract_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<DataContract>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract
            .verify_contract
        {
            0 => Drive::verify_contract_v0(
                proof,
                contract_known_keeps_history,
                is_proof_subset,
                in_multiple_contract_proof_form,
                contract_id,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
