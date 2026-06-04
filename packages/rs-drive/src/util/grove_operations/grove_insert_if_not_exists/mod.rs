mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Inserts an element into groveDB only if the specified path and key do not exist.
    /// This operation costs are then stored in `drive_operations`.
    ///
    /// # Parameters
    /// * `path`: The groveDB hierarchical authenticated structure path where the new element is to be inserted.
    /// * `key`: The key where the new element should be inserted in the subtree.
    /// * `element`: The element to be inserted.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation. In this case,
    ///   it collects the cost of this insert operation if the path and key did not exist.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(true)` if the insertion was successful.
    /// * `Ok(false)` if the path and key already existed.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the platform version does not match known versions.
    pub fn grove_insert_if_not_exists<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        element: Element,
        transaction: TransactionArg,
        drive_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version.grove_methods.basic.grove_insert_if_not_exists {
            0 => self.grove_insert_if_not_exists_v0(
                path,
                key,
                element,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_insert_if_not_exists".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod v11_consensus_regression_tests {
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::version::PlatformVersion;
    use grovedb::Element;
    use grovedb_path::SubtreePath;

    /// Protocol-v11 (`GROVE_V2`) consensus regression guard — testnet block 245,344.
    ///
    /// This models the v11 AddressBalances tree-set built by `transition_to_version_11`: an
    /// `empty_sum_tree` root at `[56]` (the control) plus an `empty_provable_count_sum_tree`
    /// (CLEAR_ADDRESS_POOL, `[56,'c']`) under it, both via `grove_insert_if_not_exists`. Under the
    /// v11 grove version (`GROVE_V2`) the provable-count-sum tree must be inserted as a plain value
    /// (`Op::Put`), NOT a layered subtree (`Op::PutLayeredReference`): the latter folds the child
    /// root into the parent node's `value_hash`, changing the grovedb root and breaking consensus on
    /// replay (a beta.2 node computed `98DD9B…` instead of the canonical `29B639…` and stalled at
    /// block 245,344).
    ///
    /// grovedb #759 version-gates this dispatch: `GROVE_V1`/`GROVE_V2` keep `Op::Put`
    /// (slot v0); `GROVE_V3` (protocol v12) adopts the layered subtree (slot v1),
    /// consistent with the batch path. This test pins BOTH sides of the gate so neither
    /// can silently flip: v11/`GROVE_V2` → the canonical `Op::Put` roots, and
    /// v12/`GROVE_V3` → a *different* `provable_count_sum_tree` root (intentional
    /// layered behaviour). `empty_sum_tree` is the unchanged control (layered always).
    #[test]
    fn provable_count_sum_tree_insert_preserves_v11_consensus_root() {
        // grovedb v4.1.0 (`Op::Put`) golden roots — the canonical protocol-v11 chain.
        const GOLDEN_1: [u8; 32] = [
            193, 62, 168, 151, 156, 164, 202, 8, 147, 137, 134, 209, 196, 32, 2, 85, 18, 100, 97,
            227, 62, 160, 254, 196, 250, 171, 84, 176, 58, 38, 16, 116,
        ];
        const GOLDEN_2: [u8; 32] = [
            35, 99, 15, 178, 25, 57, 206, 47, 187, 195, 100, 28, 97, 85, 113, 230, 135, 22, 34,
            126, 72, 125, 158, 90, 116, 94, 214, 136, 96, 195, 235, 46,
        ];

        // Insert empty_sum_tree at [56] (AddressBalances, the control) then
        // empty_provable_count_sum_tree at [56,'c'] (CLEAR_ADDRESS_POOL, the regressed
        // op) under `pv`, returning the grovedb root after each insert.
        fn insert_v11_address_trees(pv: &PlatformVersion) -> ([u8; 32], [u8; 32]) {
            let drive = setup_drive(None);
            drive
                .grove_insert_if_not_exists(
                    SubtreePath::empty(),
                    &[56u8],
                    Element::empty_sum_tree(),
                    None,
                    None,
                    &pv.drive,
                )
                .expect("insert sum_tree at [56]");
            let root_1 = drive
                .grove
                .root_hash(None, &pv.drive.grove_version)
                .unwrap()
                .unwrap();

            let pcs_path: Vec<Vec<u8>> = vec![vec![56u8]];
            drive
                .grove_insert_if_not_exists(
                    pcs_path.as_slice().into(),
                    b"c",
                    Element::empty_provable_count_sum_tree(),
                    None,
                    None,
                    &pv.drive,
                )
                .expect("insert provable_count_sum_tree at [56,'c']");
            let root_2 = drive
                .grove
                .root_hash(None, &pv.drive.grove_version)
                .unwrap()
                .unwrap();
            (root_1, root_2)
        }

        // v11 / GROVE_V2 — the consensus-locked path: must be `Op::Put` (golden).
        let (v11_root_1, v11_root_2) =
            insert_v11_address_trees(PlatformVersion::get(11).expect("protocol v11"));
        eprintln!("v11 root_1 (control sum_tree)        = {v11_root_1:?}");
        eprintln!("v11 root_2 (provable_count_sum_tree) = {v11_root_2:?}");
        assert_eq!(
            v11_root_1, GOLDEN_1,
            "v11 control sum_tree root changed unexpectedly"
        );
        assert_eq!(
            v11_root_2, GOLDEN_2,
            "ProvableCountSumTree insert under GROVE_V2 no longer matches grovedb v4.1.0 \
             (Op::Put) — protocol-v11 consensus regression (testnet block 245,344)"
        );

        // v12 / GROVE_V3 — intentionally layered (grovedb #759 version gate): the
        // provable_count_sum_tree root MUST differ from the v11 Op::Put golden, while the
        // empty_sum_tree control MUST stay on GOLDEN_1 — the gate is scoped to
        // CountSumTree / ProvableCount[Sum]Tree only, never plain sum_tree.
        let (v12_root_1, v12_root_2) =
            insert_v11_address_trees(PlatformVersion::get(12).expect("protocol v12"));
        eprintln!("v12 root_1 (control sum_tree)        = {v12_root_1:?}");
        eprintln!("v12 root_2 (provable_count_sum_tree) = {v12_root_2:?}");
        assert_eq!(
            v12_root_1, GOLDEN_1,
            "v12 control sum_tree root changed — grovedb #759's version gate must affect only \
             CountSumTree / ProvableCount[Sum]Tree, never plain empty_sum_tree"
        );
        assert_ne!(
            v12_root_2, GOLDEN_2,
            "GROVE_V3 (protocol v12) must use the layered-subtree dispatch (grovedb #759) — \
             a root matching the v11 Op::Put value means the version gate was lost"
        );
    }
}
