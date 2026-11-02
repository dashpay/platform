use crate::drive::contract_groups::paths::{
    contract_groups_root_path, CONTRACT_GROUPS_GROUPS_KEY, CONTRACT_GROUPS_MEMBERS_KEY,
};
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Inserts the `ContractGroups` root tree and its two subtrees, `Groups` and `Members`.
    ///
    /// CONSENSUS-CRITICAL: this is the single source of truth for the shape of the
    /// `[ContractGroups]` subtree. Both the fresh-genesis path
    /// (`Drive::create_initial_state_structure_v4`) and the in-place upgrade path
    /// (`Platform::transition_to_version_14`) call this helper so a node started at protocol
    /// version 14 and a node upgraded to it build a byte-identical subtree. Do not create these
    /// trees anywhere else.
    ///
    /// Both subtrees are created up front so a registration only writes under `Groups` and a
    /// membership only writes under `Groups` and `Members`. Every insert is `if not exists`, so
    /// the helper is safe to call on a tree that already has the structure.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `platform_version`: The platform version used to select grove method versions.
    ///
    /// # Returns
    ///
    /// * `Ok(())` when the three trees exist afterwards.
    /// * `Err(Error)` when GroveDB refuses an insert.
    pub fn insert_contract_groups_structure(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.grove_insert_if_not_exists(
            SubtreePath::empty(),
            &[RootTree::ContractGroups as u8],
            Element::empty_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;
        for subtree_key in [CONTRACT_GROUPS_GROUPS_KEY, CONTRACT_GROUPS_MEMBERS_KEY] {
            self.grove_insert_if_not_exists(
                contract_groups_root_path().as_slice().into(),
                subtree_key,
                Element::empty_tree(),
                transaction,
                None,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}
