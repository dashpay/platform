use super::DataContractsByRangePage;
use crate::drive::contract::paths::{
    all_contracts_global_root_path, contract_root_path_vec, contract_storage_path_vec,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::bounded_decode::decode_proof_data_contract;
use crate::verify::RootHash;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};

/// The key under a contract subtree that holds the serialized contract, or, for a contract
/// that keeps history, the history subtree whose key `0` references the latest revision.
const CONTRACT_STORAGE_KEY: &[u8] = &[0];

/// One verified page row: the contract id and the contract (absent in ids-only pages).
type PageRow = (Identifier, Option<DataContract>);

impl Drive {
    /// Verifies one page of the contract enumeration.
    ///
    /// Rebuilds the exact path query the prover used (ids-only or full), verifies the
    /// GroveDB proof against it, then reads every proved row back into a contract id and,
    /// unless `ids_only`, the serialized contract. Rows must be present, in ascending id
    /// order and at or after the cursor; a full row must sit under the contract subtree
    /// named by the id it decodes to.
    #[inline(always)]
    pub(super) fn verify_contracts_by_range_v0(
        proof: &[u8],
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        ids_only: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DataContractsByRangePage), Error> {
        let path_query = if ids_only {
            Self::fetch_contract_ids_by_range_query(start_at, limit)
        } else {
            Self::fetch_contracts_by_range_query(start_at, limit)
        };

        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        if proved_key_values.len() > limit as usize {
            return Err(Error::Proof(ProofError::TooManyElements(
                "the contract page proof carries more rows than the requested limit",
            )));
        }

        let mut page: DataContractsByRangePage = Vec::with_capacity(proved_key_values.len());

        for (path, key, maybe_element) in proved_key_values {
            let element = maybe_element.ok_or_else(|| {
                Error::Proof(ProofError::CorruptedProof(
                    "the contract page proof carries an absent row; a range proof only \
                     carries present contracts"
                        .to_string(),
                ))
            })?;

            let row = if ids_only {
                ids_only_row(path, key, element)?
            } else {
                contract_row(path, key, element, platform_version)?
            };

            check_row_order(&page, &row.0, start_at)?;
            page.push(row);
        }

        Ok((root_hash, page))
    }
}

/// Rejects a row that breaks ascending contract id order or lands before the cursor.
fn check_row_order(
    page: &[PageRow],
    contract_id: &Identifier,
    start_at: Option<([u8; 32], bool)>,
) -> Result<(), Error> {
    match page.last() {
        Some((previous_id, _)) if previous_id >= contract_id => {
            Err(Error::Proof(ProofError::CorruptedProof(
                "the contract page rows are not in ascending contract id order".to_string(),
            )))
        }
        Some(_) => Ok(()),
        None => match start_at {
            Some((cursor, true)) if contract_id.to_buffer() < cursor => {
                Err(Error::Proof(ProofError::CorruptedProof(
                    "the first contract page row precedes the inclusive start cursor".to_string(),
                )))
            }
            Some((cursor, false)) if contract_id.to_buffer() <= cursor => {
                Err(Error::Proof(ProofError::CorruptedProof(
                    "the first contract page row does not follow the exclusive start cursor"
                        .to_string(),
                )))
            }
            _ => Ok(()),
        },
    }
}

/// Reads an ids-only row: the contract subtree keyed by its id, directly under the
/// contracts root.
fn ids_only_row(path: Vec<Vec<u8>>, key: Vec<u8>, element: Element) -> Result<PageRow, Error> {
    let [contracts_root] = all_contracts_global_root_path();
    let under_contracts_root =
        path.len() == 1 && path.first().map(Vec::as_slice) == Some(contracts_root);
    if !under_contracts_root {
        return Err(Error::Proof(ProofError::CorruptedProof(
            "an ids-only contract page row is not directly under the contracts root".to_string(),
        )));
    }
    if !element.is_any_tree() {
        return Err(Error::Proof(ProofError::CorruptedProof(
            "an ids-only contract page row is not a contract subtree".to_string(),
        )));
    }
    Ok((identifier_from_bytes(key)?, None))
}

/// Reads a full row. Two shapes are valid, one per storage layout:
///
/// - a contract that does not keep history: path `[contracts root, id]`, key `0`, the
///   serialized contract item;
/// - a contract that keeps history: path `[contracts root, id, 0]`, key `0`, the latest
///   revision item the proof dereferenced.
///
/// The decoded contract must carry the id of the subtree it was proved under.
fn contract_row(
    path: Vec<Vec<u8>>,
    key: Vec<u8>,
    element: Element,
    platform_version: &PlatformVersion,
) -> Result<PageRow, Error> {
    if key.as_slice() != CONTRACT_STORAGE_KEY {
        return Err(Error::Proof(ProofError::CorruptedProof(
            "a contract page row is not keyed by the contract storage key".to_string(),
        )));
    }

    let contract_id_bytes = path.get(1).ok_or_else(|| {
        Error::Proof(ProofError::CorruptedProof(
            "a contract page row path has no contract id component".to_string(),
        ))
    })?;

    let expected_path = match path.len() {
        2 => contract_root_path_vec(contract_id_bytes),
        3 => contract_storage_path_vec(contract_id_bytes),
        _ => {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "a contract page row path has an unexpected depth".to_string(),
            )))
        }
    };
    if path != expected_path {
        return Err(Error::Proof(ProofError::CorruptedProof(
            "a contract page row is not under a contract subtree of the contracts root".to_string(),
        )));
    }

    let contract_id = identifier_from_bytes(contract_id_bytes.clone())?;

    let serialized_contract = element.into_item_bytes().map_err(Error::from)?;
    let contract = decode_proof_data_contract(&serialized_contract, platform_version)?;

    if contract.id() != contract_id {
        return Err(Error::Proof(ProofError::CorruptedProof(
            "a proved contract does not carry the id of the subtree it was proved under"
                .to_string(),
        )));
    }

    Ok((contract_id, Some(contract)))
}

fn identifier_from_bytes(bytes: Vec<u8>) -> Result<Identifier, Error> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        Error::Proof(ProofError::CorruptedProof(format!(
            "a contract id in the contract page proof has {} bytes, expected 32",
            bytes.len()
        )))
    })?;
    Ok(Identifier::new(bytes))
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::{
        DataContractConfigGettersV0, DataContractConfigSettersV0,
    };
    use dpp::data_contract::DataContract;
    use dpp::prelude::Identifier;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use grovedb_epoch_based_storage_flags::StorageFlags;

    const PLAIN_CONTRACT: &str = "tests/supporting_files/contract/family/family-contract.json";
    const HISTORY_CONTRACT: &str =
        "tests/supporting_files/contract/references/references_with_contract_history.json";

    /// Five contracts with ids `[n; 32]`, alternating between the plain layout and the
    /// history-keeping layout (a history subtree plus a reference to the latest revision).
    const MIXED_LAYOUTS: &[(u8, bool)] =
        &[(1, false), (2, true), (3, false), (4, true), (5, false)];

    /// Inserts one contract per `(id byte, keeps_history)` pair and returns the drive with
    /// the inserted ids in ascending order.
    fn setup_contracts(layouts: &[(u8, bool)]) -> (Drive, Vec<[u8; 32]>) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut ids = Vec::new();
        for (seed, keeps_history) in layouts {
            let fixture = if *keeps_history {
                HISTORY_CONTRACT
            } else {
                PLAIN_CONTRACT
            };
            let mut contract = json_document_to_contract(fixture, false, platform_version)
                .expect("expected to load the contract fixture");
            let id = [*seed; 32];
            contract.set_id(id.into());
            if *keeps_history {
                contract.config_mut().set_keeps_history(true);
                contract.config_mut().set_readonly(false);
            }
            drive
                .apply_contract(
                    &contract,
                    BlockInfo {
                        time_ms: 1000,
                        height: 100,
                        core_height: 10,
                        epoch: Default::default(),
                    },
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply the contract");
            ids.push(id);
        }
        ids.sort();
        (drive, ids)
    }

    fn prove_and_verify(
        drive: &Drive,
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        ids_only: bool,
    ) -> Vec<(Identifier, Option<DataContract>)> {
        let platform_version = PlatformVersion::latest();
        let proof = drive
            .prove_contracts_by_range(start_at, limit, ids_only, None, platform_version)
            .expect("expected to prove the page");
        let (_root_hash, page) =
            Drive::verify_contracts_by_range(&proof, start_at, limit, ids_only, platform_version)
                .expect("expected to verify the page");
        page
    }

    fn page_ids(page: &[(Identifier, Option<DataContract>)]) -> Vec<[u8; 32]> {
        page.iter().map(|(id, _)| id.to_buffer()).collect()
    }

    #[test]
    fn should_prove_and_verify_first_page_with_mixed_history_layouts() {
        let (drive, ids) = setup_contracts(MIXED_LAYOUTS);

        let page = prove_and_verify(&drive, None, 100, false);

        assert_eq!(page_ids(&page), ids);
        for (id, contract) in &page {
            let contract = contract
                .as_ref()
                .expect("a full page carries every contract");
            assert_eq!(contract.id(), *id);
            let expected_keeps_history = MIXED_LAYOUTS
                .iter()
                .find(|(seed, _)| [*seed; 32] == id.to_buffer())
                .map(|(_, keeps_history)| *keeps_history)
                .expect("every proved id was inserted");
            assert_eq!(contract.config().keeps_history(), expected_keeps_history);
        }
    }

    #[test]
    fn should_page_with_start_after_and_start_at() {
        let (drive, ids) = setup_contracts(MIXED_LAYOUTS);

        let page1 = page_ids(&prove_and_verify(&drive, None, 2, false));
        assert_eq!(page1, ids[..2]);

        let page2 = page_ids(&prove_and_verify(&drive, Some((page1[1], false)), 2, false));
        assert_eq!(page2, ids[2..4]);

        let page3 = page_ids(&prove_and_verify(&drive, Some((page2[1], false)), 2, false));
        assert_eq!(page3, ids[4..], "the last page is short");

        let from_fourth = page_ids(&prove_and_verify(&drive, Some((ids[3], true)), 2, false));
        assert_eq!(from_fourth, ids[3..], "start_at includes the cursor");

        let past_end = prove_and_verify(&drive, Some((ids[4], false)), 2, false);
        assert!(
            past_end.is_empty(),
            "start_after the last id is an empty page"
        );
    }

    #[test]
    fn should_prove_and_verify_ids_only_pages() {
        let (drive, ids) = setup_contracts(MIXED_LAYOUTS);

        let page = prove_and_verify(&drive, None, 100, true);
        assert_eq!(page_ids(&page), ids);
        assert!(page.iter().all(|(_, contract)| contract.is_none()));

        let page2 = prove_and_verify(&drive, Some((ids[1], false)), 2, true);
        assert_eq!(page_ids(&page2), ids[2..4]);
    }

    #[test]
    fn should_verify_empty_pages_on_empty_state() {
        let drive = setup_drive_with_initial_state_structure(None);

        assert!(prove_and_verify(&drive, None, 10, false).is_empty());
        assert!(prove_and_verify(&drive, None, 10, true).is_empty());
    }

    #[test]
    fn should_reject_proof_verified_against_different_page_parameters() {
        let (drive, ids) = setup_contracts(MIXED_LAYOUTS);
        let platform_version = PlatformVersion::latest();

        let proof = drive
            .prove_contracts_by_range(None, 2, false, None, platform_version)
            .expect("expected to prove the page");

        assert!(
            Drive::verify_contracts_by_range(&proof, None, 3, false, platform_version).is_err(),
            "a larger limit must not verify a shorter page"
        );
        assert!(
            Drive::verify_contracts_by_range(
                &proof,
                Some((ids[0], false)),
                2,
                false,
                platform_version
            )
            .is_err(),
            "a different cursor must not verify"
        );
        assert!(
            Drive::verify_contracts_by_range(&proof, None, 2, true, platform_version).is_err(),
            "an ids-only query must not verify a full-page proof"
        );
    }

    #[test]
    fn should_reject_tampered_proof() {
        let (drive, _ids) = setup_contracts(MIXED_LAYOUTS);
        let platform_version = PlatformVersion::latest();

        let mut proof = drive
            .prove_contracts_by_range(None, 100, false, None, platform_version)
            .expect("expected to prove the page");
        let middle = proof.len() / 2;
        proof[middle] ^= 0xff;

        assert!(matches!(
            Drive::verify_contracts_by_range(&proof, None, 100, false, platform_version),
            Err(Error::GroveDB(_)) | Err(Error::Proof(_))
        ));
    }
}
