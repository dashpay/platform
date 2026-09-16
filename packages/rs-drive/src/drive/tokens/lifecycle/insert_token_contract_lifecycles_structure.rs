use crate::drive::tokens::lifecycle::encode_destroyed_supply;
use crate::drive::tokens::paths::{
    token_contract_lifecycles_root_path, tokens_root_path, TOKEN_CONTRACT_LIFECYCLES_KEY,
    TOKEN_DESTROYED_SUPPLY_KEY, TOKEN_LIFECYCLE_CLEANUP_QUEUE_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Inserts the token contract lifecycle ledger under an already existing `[Tokens]` tree:
    /// the ledger tree itself, the zero destroyed supply scalar and the empty cleanup queue.
    ///
    /// CONSENSUS-CRITICAL: this is the single source of truth for the ledger's shape. Both the
    /// genesis path of the protocol version that introduces it and the in-place upgrade path
    /// call this helper, so a fresh node and an upgraded node hold byte-identical elements.
    /// The construction is sequential, one insert-if-not-exists per element in this order,
    /// which is what fixes the Merk shape of the ledger's two one-byte keys; every insert is
    /// idempotent, so calling it on a state that already holds the ledger changes nothing.
    ///
    /// # Parameters
    ///
    /// * `transaction` - The groveDB transaction associated with this operation.
    /// * `platform_version` - The platform version used to select grove method versions.
    pub fn insert_token_contract_lifecycles_structure(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let tokens_path = tokens_root_path();
        self.grove_insert_if_not_exists(
            (&tokens_path).into(),
            &[TOKEN_CONTRACT_LIFECYCLES_KEY],
            Element::empty_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;

        let lifecycles_path = token_contract_lifecycles_root_path();
        self.grove_insert_if_not_exists(
            (&lifecycles_path).into(),
            &TOKEN_DESTROYED_SUPPLY_KEY,
            Element::new_item(encode_destroyed_supply(0)),
            transaction,
            None,
            &platform_version.drive,
        )?;

        self.grove_insert_if_not_exists(
            (&lifecycles_path).into(),
            &TOKEN_LIFECYCLE_CLEANUP_QUEUE_KEY,
            Element::empty_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;

        Ok(())
    }
}
