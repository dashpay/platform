use super::DataContractsVersions;
use crate::drive::contract::paths::{
    all_contracts_global_root_path, CONTRACT_OTHER_KEY, CONTRACT_VERSION_KEY,
};
use crate::drive::contract::version_item::decode_contract_version;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};
use std::collections::BTreeSet;

impl Drive {
    /// Verifies the version item proof.
    ///
    /// Rebuilds the exact path query the prover used, verifies the proof against it with
    /// absence, then reads every proved row back. A row is either the version item (or its
    /// absence) at key `64` of the contract's other tree (`[64, id, 2]`), the absence of that
    /// tree under the contract's root subtree, or the absence of the contract subtree itself
    /// directly under the contracts root; every absence means there is no version for that id.
    /// Every distinct requested id must appear exactly once and nothing else may.
    #[inline(always)]
    pub(super) fn verify_contracts_versions_v0(
        proof: &[u8],
        contract_ids: &[[u8; 32]],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DataContractsVersions), Error> {
        if contract_ids.is_empty() {
            return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                "no contract ids to verify versions for",
            )));
        }
        if contract_ids.len() > u16::MAX as usize {
            return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                "at most {} contract versions can be verified at once, got {}",
                u16::MAX,
                contract_ids.len()
            ))));
        }

        let requested: BTreeSet<[u8; 32]> = contract_ids.iter().copied().collect();
        let path_query = Self::fetch_contracts_versions_query(contract_ids);

        let (root_hash, proved_key_values) = GroveDb::verify_query_with_absence_proof(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        )?;

        let [contracts_root] = all_contracts_global_root_path();
        let mut versions = DataContractsVersions::new();

        for (path, key, maybe_element) in proved_key_values {
            let (contract_id, version) = match path.as_slice() {
                [root, contract_id, other]
                    if root.as_slice() == contracts_root
                        && other.as_slice() == [CONTRACT_OTHER_KEY] =>
                {
                    if key.as_slice() != [CONTRACT_VERSION_KEY] {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "a contract version row is not keyed by the version key".to_string(),
                        )));
                    }
                    (
                        contract_id_from_bytes(contract_id)?,
                        maybe_element.map(version_from_element).transpose()?,
                    )
                }
                // A contract without its other tree (stored before the version item existed and
                // not migrated yet) proves the tree's key absent.
                [root, contract_id]
                    if root.as_slice() == contracts_root
                        && key.as_slice() == [CONTRACT_OTHER_KEY] =>
                {
                    if maybe_element.is_some() {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "a contract version proof carries a contract's other tree itself"
                                .to_string(),
                        )));
                    }
                    (contract_id_from_bytes(contract_id)?, None)
                }
                [root] if root.as_slice() == contracts_root => {
                    if maybe_element.is_some() {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "a contract version proof carries an element directly under the \
                             contracts root"
                                .to_string(),
                        )));
                    }
                    (contract_id_from_bytes(&key)?, None)
                }
                _ => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "a contract version row has an unexpected path".to_string(),
                    )))
                }
            };

            if !requested.contains(&contract_id) {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "the contract version proof carries a contract that was not requested"
                        .to_string(),
                )));
            }
            if versions.insert(contract_id, version).is_some() {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "the contract version proof carries a contract twice".to_string(),
                )));
            }
        }

        if versions.len() != requested.len() {
            return Err(Error::Proof(ProofError::WrongElementCount {
                expected: requested.len(),
                got: versions.len(),
            }));
        }

        Ok((root_hash, versions))
    }
}

/// Reads the version out of a proved version item.
fn version_from_element(element: Element) -> Result<u32, Error> {
    let bytes = element.into_item_bytes().map_err(Error::from)?;
    decode_contract_version(&bytes).ok_or_else(|| {
        Error::Proof(ProofError::CorruptedProof(
            "a contract version item is not four bytes".to_string(),
        ))
    })
}

fn contract_id_from_bytes(bytes: &[u8]) -> Result<[u8; 32], Error> {
    bytes.try_into().map_err(|_| {
        Error::Proof(ProofError::CorruptedProof(format!(
            "a contract id in the contract version proof has {} bytes, expected 32",
            bytes.len()
        )))
    })
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::DataContract;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use grovedb_epoch_based_storage_flags::StorageFlags;

    const PLAIN_CONTRACT: &str = "tests/supporting_files/contract/family/family-contract.json";
    const HISTORY_CONTRACT: &str =
        "tests/supporting_files/contract/references/references_with_contract_history.json";

    /// Three contracts with ids `[n; 32]`: two plain, one keeping history.
    const LAYOUTS: &[(u8, bool)] = &[(1, false), (2, true), (3, false)];

    fn block(time_ms: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            height: 100,
            core_height: 10,
            epoch: Default::default(),
        }
    }

    fn apply(drive: &Drive, contract: &DataContract, time_ms: u64) {
        drive
            .apply_contract(
                contract,
                block(time_ms),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to apply the contract");
    }

    fn setup_contracts() -> (Drive, Vec<DataContract>) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contracts = LAYOUTS
            .iter()
            .map(|(seed, keeps_history)| {
                let fixture = if *keeps_history {
                    HISTORY_CONTRACT
                } else {
                    PLAIN_CONTRACT
                };
                let mut contract = json_document_to_contract(fixture, false, platform_version)
                    .expect("expected to load the contract fixture");
                contract.set_id([*seed; 32].into());
                if *keeps_history {
                    contract.config_mut().set_keeps_history(true);
                    contract.config_mut().set_readonly(false);
                }
                apply(&drive, &contract, 1000);
                contract
            })
            .collect();
        (drive, contracts)
    }

    fn prove_and_verify(drive: &Drive, ids: &[[u8; 32]]) -> super::DataContractsVersions {
        let platform_version = PlatformVersion::latest();
        let proof = drive
            .prove_contracts_versions(ids, None, platform_version)
            .expect("expected to prove the versions");
        let (_root_hash, versions) =
            Drive::verify_contracts_versions(&proof, ids, platform_version)
                .expect("expected to verify the versions");
        versions
    }

    #[test]
    fn should_prove_and_verify_versions_of_both_layouts_and_absence() {
        let (drive, contracts) = setup_contracts();
        let missing = [9; 32];
        let ids: Vec<[u8; 32]> = contracts
            .iter()
            .map(|contract| contract.id().to_buffer())
            .chain([missing])
            .collect();

        let versions = prove_and_verify(&drive, &ids);

        assert_eq!(versions.len(), ids.len());
        for contract in &contracts {
            assert_eq!(
                versions.get(&contract.id().to_buffer()),
                Some(&Some(contract.version())),
                "every stored contract proves its version"
            );
        }
        assert_eq!(
            versions.get(&missing),
            Some(&None),
            "a missing id proves absent"
        );
    }

    #[test]
    fn should_prove_the_version_after_an_update_and_fold_duplicate_ids() {
        let (drive, mut contracts) = setup_contracts();
        let contract = &mut contracts[1];
        contract.increment_version();
        apply(&drive, contract, 2000);
        let id = contract.id().to_buffer();

        let versions = prove_and_verify(&drive, &[id, id]);

        assert_eq!(versions.len(), 1);
        assert_eq!(versions.get(&id), Some(&Some(contract.version())));
    }

    #[test]
    fn should_reject_a_proof_verified_against_different_ids() {
        let (drive, contracts) = setup_contracts();
        let platform_version = PlatformVersion::latest();
        let proved_ids = [contracts[0].id().to_buffer()];
        let proof = drive
            .prove_contracts_versions(&proved_ids, None, platform_version)
            .expect("expected to prove the versions");

        let other_ids = [contracts[1].id().to_buffer()];
        assert!(
            Drive::verify_contracts_versions(&proof, &other_ids, platform_version).is_err(),
            "a proof of one id must not verify for another"
        );

        let more_ids = [contracts[0].id().to_buffer(), contracts[1].id().to_buffer()];
        assert!(
            Drive::verify_contracts_versions(&proof, &more_ids, platform_version).is_err(),
            "a proof of one id must not verify for two"
        );
    }

    #[test]
    fn should_reject_a_tampered_proof() {
        let (drive, contracts) = setup_contracts();
        let platform_version = PlatformVersion::latest();
        let ids: Vec<[u8; 32]> = contracts.iter().map(|c| c.id().to_buffer()).collect();
        let mut proof = drive
            .prove_contracts_versions(&ids, None, platform_version)
            .expect("expected to prove the versions");
        let middle = proof.len() / 2;
        proof[middle] ^= 0xff;

        assert!(matches!(
            Drive::verify_contracts_versions(&proof, &ids, platform_version),
            Err(Error::GroveDB(_)) | Err(Error::Proof(_))
        ));
    }

    #[test]
    fn should_reject_more_ids_than_a_query_limit_can_hold() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let ids: Vec<[u8; 32]> = (0..=u16::MAX as u32)
            .map(|i| {
                let mut id = [0u8; 32];
                id[..4].copy_from_slice(&i.to_be_bytes());
                id
            })
            .collect();

        assert!(drive
            .prove_contracts_versions(&ids, None, platform_version)
            .is_err());
        assert!(Drive::verify_contracts_versions(&[], &ids, platform_version).is_err());
    }

    #[test]
    fn should_reject_empty_ids() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        assert!(drive
            .prove_contracts_versions(&[], None, platform_version)
            .is_err());
        assert!(Drive::verify_contracts_versions(&[], &[], platform_version).is_err());
    }

    #[test]
    fn should_prove_the_versions_far_smaller_than_the_contracts() {
        let (drive, contracts) = setup_contracts();
        let platform_version = PlatformVersion::latest();
        let ids: Vec<[u8; 32]> = contracts.iter().map(|c| c.id().to_buffer()).collect();

        let versions_proof = drive
            .prove_contracts_versions(&ids, None, platform_version)
            .expect("expected to prove the versions");
        let contracts_proof = drive
            .prove_contracts(&ids, None, platform_version)
            .expect("expected to prove the contracts");

        assert!(
            versions_proof.len() * 4 < contracts_proof.len(),
            "the version proof ({} bytes) must be well under a quarter of the contract proof ({} bytes)",
            versions_proof.len(),
            contracts_proof.len()
        );
    }
}
