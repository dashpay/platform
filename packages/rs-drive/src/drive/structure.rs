use crate::drive::address_funds::structure::structure as address_balances;
use crate::drive::asset_lock::structure::structure as spent_asset_locks;
use crate::drive::balances::structure::structure as balances;
use crate::drive::contract::structure::structure as contracts_and_documents;
use crate::drive::contract_groups::structure::structure as contract_groups;
use crate::drive::credit_pools::structure::structure as pools;
use crate::drive::group::structure::structure as group_actions;
use crate::drive::identity::structure::{
    non_unique_key_hashes_structure, structure as identities, unique_key_hashes_structure,
};
use crate::drive::identity::withdrawals::structure::structure as withdrawals;
use crate::drive::prefunded_specialized_balances::structure::structure as prefunded_balances;
use crate::drive::protocol_upgrade::structure::structure as versions;
use crate::drive::saved_block_transactions::structure::structure as saved_block_transactions;
use crate::drive::shielded::structure::structure as shielded_balances;
use crate::drive::system::structure::structure as misc;
use crate::drive::tokens::structure::structure as tokens;
use crate::drive::votes::structure::structure as votes;
use crate::structure::StructureNode;

/// The root layer: one child per `RootTree`
/// variant, each declared by its area.
///
/// The root keys are spread over the byte range so the root Merk stays
/// shallow for the trees read most. Where a new key lands in that Merk
/// matters: every write below a node rewrites its ancestors, so a root key
/// should hang below a tree that fee bearing transitions do not write to.
pub(crate) fn root_structure() -> StructureNode {
    StructureNode::root().children(vec![
        non_unique_key_hashes_structure(),
        tokens(),
        unique_key_hashes_structure(),
        identities(),
        saved_block_transactions(),
        prefunded_balances(),
        pools(),
        shielded_balances(),
        address_balances(),
        contracts_and_documents(),
        spent_asset_locks(),
        withdrawals(),
        group_actions(),
        balances(),
        misc(),
        votes(),
        versions(),
        contract_groups(),
    ])
}
