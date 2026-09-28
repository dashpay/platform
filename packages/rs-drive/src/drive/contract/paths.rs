use crate::drive::RootTree;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::data_contract::document_type::action_fees::ContractFeePot;

use crate::drive::votes::paths::{ACTIVE_POLLS_TREE_KEY, CONTESTED_RESOURCE_TREE_KEY};
use dpp::data_contract::DataContract;

/// The various GroveDB paths underneath a contract
pub trait DataContractPaths {
    /// The root path, under this there should be the documents area and the contract itself
    fn root_path(&self) -> [&[u8]; 2];
    /// The documents path, under this you should have the various document types
    fn documents_path(&self) -> [&[u8]; 3];
    /// The document type path, this is based on the document type name
    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4];
    /// The contested document type path, this is based on the document type name
    fn contested_document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5];
    /// The document primary key path, this is under the document type
    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5];
    /// The contested document primary key path, this is under the document type
    fn contested_documents_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
    ) -> [&'a [u8]; 6];
    /// The underlying storage for documents that keep history
    fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6];
}

impl DataContractPaths for DataContract {
    fn root_path(&self) -> [&[u8]; 2] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
        ]
    }

    fn documents_path(&self) -> [&[u8]; 3] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
        ]
    }

    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
            document_type_name.as_bytes(),
        ]
    }

    fn contested_document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        [
            Into::<&[u8; 1]>::into(RootTree::Votes), // 1
            &[CONTESTED_RESOURCE_TREE_KEY as u8],    // 1
            &[ACTIVE_POLLS_TREE_KEY as u8],          // 1
            self.id_ref().as_bytes(),                // 32
            document_type_name.as_bytes(),
        ]
    }

    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
            document_type_name.as_bytes(),
            &[0],
        ]
    }

    fn contested_documents_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
    ) -> [&'a [u8]; 6] {
        [
            Into::<&[u8; 1]>::into(RootTree::Votes), // 1
            &[CONTESTED_RESOURCE_TREE_KEY as u8],    // 1
            &[ACTIVE_POLLS_TREE_KEY as u8],          // 1
            self.id_ref().as_bytes(),                // 32
            document_type_name.as_bytes(),
            &[0],
        ]
    }

    fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
            document_type_name.as_bytes(),
            &[0],
            id,
        ]
    }
}

/// The global root path for all contracts
pub fn all_contracts_global_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::DataContractDocuments)]
}

/// Takes a contract ID and returns the contract's root path.
pub fn contract_root_path(contract_id: &[u8]) -> [&[u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
    ]
}

/// Takes a contract ID and returns the contract's root path.
pub fn contract_root_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
    ]
}

/// Takes a contract ID and returns the contract's storage path (where it is stored).
pub fn contract_storage_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![0],
    ]
}

/// Takes a contract ID and returns the contract's storage history path.
pub fn contract_keeping_history_root_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![0],
    ]
}

/// Takes a contract ID and returns the contract's storage history path.
pub fn contract_keeping_history_root_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[0],
    ]
}

/// Takes a contract ID and an encoded timestamp and returns the contract's storage history path
/// for that timestamp.
pub fn contract_keeping_history_storage_time_reference_path(
    contract_id: &[u8],
    encoded_time: Vec<u8>,
) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![0],
        encoded_time,
    ]
}

/// The key under a contract's root subtree (`[64, id]`) of the contract's "other" tree, from
/// protocol version 14: everything a contract keeps beside itself (key `0`, the contract or
/// its history subtree) and its documents (key `1`). With three keys the root subtree's Merk
/// keeps `1`, the documents, on top, where every document proof and write goes through.
///
/// Inside, the keys are spread like the root tree's, so that the tree is balanced as it fills
/// and the most read entry sits on top: `128` the banlist, `64` the version item, `192` the
/// suspension list, `224` the warning list. A Merk built from one sorted batch roots at the
/// middle key (the upper middle of an even count), so `128` is on top whenever it is the
/// median of the keys created together; a key added later goes where it keeps `128` the
/// median in the most likely combinations, and what a document transition never reads may
/// sit a level down in the others.
pub const CONTRACT_OTHER_KEY: u8 = 2;

/// The key under a contract's other tree (`[64, id, 2]`) that holds the contract's version
/// number as a four-byte big-endian item, written beside the contract from protocol
/// version 14. Every contract has it.
pub const CONTRACT_VERSION_KEY: u8 = 64;

/// The key under a contract's other tree (`[64, id, 2]`) of the banlist a moderated contract
/// keeps (protocol version 14): `identity id -> Item(reason)`. Present only when the contract's
/// config declares a banlist. Read by every document transition on the contract, hence on
/// top.
pub const CONTRACT_BANLIST_KEY: u8 = 128;

/// The key under a contract's other tree (`[64, id, 2]`) of the suspension list a moderated
/// contract keeps (protocol version 14): `identity id -> Item(until, u64 big-endian
/// milliseconds, then the reason)`. Present only when the contract's config declares a
/// suspension list.
pub const CONTRACT_SUSPENSIONS_KEY: u8 = 192;

/// The key under a contract's other tree (`[64, id, 2]`) of the warning list a moderated
/// contract keeps (protocol version 14): `identity id -> Item(one or more of: warned at as a
/// u64 big-endian milliseconds, the reason's length as a u16 big-endian, the reason)`, oldest
/// warning first. Present only when the contract's config declares a warning list. A warning
/// bars nothing, so no document transition reads it, and its depth does not matter: above
/// `192`, it leaves the banlist on top of the other tree for a contract that keeps the
/// banlist and a warning list, with or without removal records and with both when it keeps
/// a suspension list too; only a contract keeping all three lists and no removal records has
/// the suspension list on top and the banlist one level down.
pub const CONTRACT_WARNINGS_KEY: u8 = 224;

/// The key under a contract's other tree (`[64, id, 2]`) of the records of the documents the
/// contract's moderators deleted (protocol version 14): `document type name -> document id ->
/// Item(document owner id, moderator id, removed at, reason)`. Present when the contract has a
/// document type that sets `canBeDeletedByModerators`, with one subtree per such type,
/// created with the type. Written by a moderator's document deletion and read by clients,
/// never by a document transition, so it sorts below `128`: created together with both lists
/// it leaves the banlist on top.
pub const CONTRACT_DOCUMENT_REMOVALS_KEY: u8 = 16;

/// The key under a contract's other tree (`[64, id, 2]`) of the moderation action counts of a
/// contract that declares elected moderation (protocol version 14): `identity id -> Item(count,
/// u32 big-endian)`, one per member of the seated team who signed a counted moderation action
/// (a ban, a suspension, a warning or a document deletion) since the moderators pot was last
/// settled. Every settle, a claim or a change of the team, pays the pot out by them and
/// deletes them. Created with an elected contract, the only kind that has a seated team.
///
/// Read by the moderation actions of the team and by a settle, never by a document transition,
/// so it sorts below `64`: created together with the lists of an elected contract it keeps the
/// banlist on top when the contract keeps two or three lists, with or without removal records
/// beside them (the combination of every ability included), where a key above `128` would push
/// it down in most of those. It costs the banlist a level for a contract keeping the banlist
/// alone, or the banlist and one other list beside removal records.
pub const CONTRACT_MODERATION_ACTION_COUNTS_KEY: u8 = 48;

/// `[64, contract id, 2]`: the contract's other tree.
pub fn contract_other_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[CONTRACT_OTHER_KEY],
    ]
}

/// `[64, contract id, 2]`: the contract's other tree.
pub fn contract_other_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![CONTRACT_OTHER_KEY],
    ]
}

/// The tree key of a moderation list.
pub fn contract_moderation_list_key(list: ContractModerationList) -> &'static [u8; 1] {
    match list {
        ContractModerationList::Banlist => &[CONTRACT_BANLIST_KEY],
        ContractModerationList::Suspensions => &[CONTRACT_SUSPENSIONS_KEY],
        ContractModerationList::Warnings => &[CONTRACT_WARNINGS_KEY],
    }
}

/// `[64, contract id, 2, 128]`, `[64, contract id, 2, 192]` or `[64, contract id, 2, 224]`:
/// the tree of one moderation list.
pub fn contract_moderation_list_path(
    contract_id: &[u8],
    list: ContractModerationList,
) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[CONTRACT_OTHER_KEY],
        contract_moderation_list_key(list),
    ]
}

/// `[64, contract id, 2, 128]`, `[64, contract id, 2, 192]` or `[64, contract id, 2, 224]`:
/// the tree of one moderation list.
pub fn contract_moderation_list_path_vec(
    contract_id: &[u8],
    list: ContractModerationList,
) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![CONTRACT_OTHER_KEY],
        contract_moderation_list_key(list).to_vec(),
    ]
}

/// `[64, contract id, 2, 48]`: the moderation action counts of an elected contract.
pub fn contract_moderation_action_counts_path(contract_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[CONTRACT_OTHER_KEY],
        &[CONTRACT_MODERATION_ACTION_COUNTS_KEY],
    ]
}

/// `[64, contract id, 2, 48]`: the moderation action counts of an elected contract.
pub fn contract_moderation_action_counts_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![CONTRACT_OTHER_KEY],
        vec![CONTRACT_MODERATION_ACTION_COUNTS_KEY],
    ]
}

/// The key under the prefunded specialized balances tree (`[40]`) of the sum tree holding every
/// contract's owner fee pot (protocol version 14): `contract id -> SumItem(credits)`. The
/// `owner` parts of the contract's document action fees accumulate there until the owner
/// claims them. With the voting balances at `128` on top, `64` and `192` keep the tree
/// balanced.
pub const PREFUNDED_BALANCES_FOR_CONTRACT_OWNER_FEES: u8 = 64;

/// The key under the prefunded specialized balances tree (`[40]`) of the sum tree holding every
/// contract's moderators fee pot (protocol version 14): `contract id -> SumItem(credits)`. The
/// `moderators` parts of the contract's document action fees accumulate there until a member
/// of the moderation team claims them for the team.
pub const PREFUNDED_BALANCES_FOR_CONTRACT_MODERATOR_FEES: u8 = 192;

/// The key under a contract's other tree (`[64, id, 2]`) of the last claim of its owner fee pot,
/// a 42 byte item: the epoch, the block time and the claimant (protocol version 14). Absent
/// until the first claim. Below `128`, so the banlist stays on top of the other tree.
pub const CONTRACT_LAST_OWNER_FEE_CLAIM_KEY: u8 = 32;

/// The key under a contract's other tree (`[64, id, 2]`) of the last claim of its moderators
/// fee pot, a 42 byte item: the epoch, the block time and the claimant (protocol version 14).
/// Absent until the first claim. Below `128`, so the banlist stays on top of the other tree.
pub const CONTRACT_LAST_MODERATORS_FEE_CLAIM_KEY: u8 = 96;

/// The key, under the prefunded specialized balances tree, of the sum tree of a kind of pot.
pub fn contract_fee_pots_key(pot: ContractFeePot) -> &'static [u8; 1] {
    match pot {
        ContractFeePot::Owner => &[PREFUNDED_BALANCES_FOR_CONTRACT_OWNER_FEES],
        ContractFeePot::Moderators => &[PREFUNDED_BALANCES_FOR_CONTRACT_MODERATOR_FEES],
    }
}

/// `[40, 64]` or `[40, 192]`: the sum tree holding every contract's pot of one kind.
pub fn contract_fee_pots_path(pot: ContractFeePot) -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::PreFundedSpecializedBalances),
        contract_fee_pots_key(pot),
    ]
}

/// `[40, 64]` or `[40, 192]`: the sum tree holding every contract's pot of one kind.
pub fn contract_fee_pots_path_vec(pot: ContractFeePot) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::PreFundedSpecializedBalances).to_vec(),
        contract_fee_pots_key(pot).to_vec(),
    ]
}

/// The key, under a contract's other tree, of the last claim of a pot.
pub fn contract_last_fee_claim_key(pot: ContractFeePot) -> &'static [u8; 1] {
    match pot {
        ContractFeePot::Owner => &[CONTRACT_LAST_OWNER_FEE_CLAIM_KEY],
        ContractFeePot::Moderators => &[CONTRACT_LAST_MODERATORS_FEE_CLAIM_KEY],
    }
}

/// `[64, contract id, 2, 16]`: the tree of the contract's document removal records, one subtree
/// per document type moderators may delete documents of.
pub fn contract_document_removals_path(contract_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[CONTRACT_OTHER_KEY],
        &[CONTRACT_DOCUMENT_REMOVALS_KEY],
    ]
}

/// `[64, contract id, 2, 16]`: the tree of the contract's document removal records.
pub fn contract_document_removals_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![CONTRACT_OTHER_KEY],
        vec![CONTRACT_DOCUMENT_REMOVALS_KEY],
    ]
}

/// `[64, contract id, 2, 16, document type name]`: the removal records of one document type,
/// keyed by document id.
pub fn contract_document_type_removals_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[CONTRACT_OTHER_KEY],
        &[CONTRACT_DOCUMENT_REMOVALS_KEY],
        document_type_name.as_bytes(),
    ]
}

/// `[64, contract id, 2, 16, document type name]`: the removal records of one document type.
pub fn contract_document_type_removals_path_vec(
    contract_id: &[u8],
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![CONTRACT_OTHER_KEY],
        vec![CONTRACT_DOCUMENT_REMOVALS_KEY],
        document_type_name.as_bytes().to_vec(),
    ]
}
