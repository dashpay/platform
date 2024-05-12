use crate::drive::RootTree;
use dpp::identity::TimestampMillis;

/// The votes tree structure looks like this
///
///     Votes
///
///     |- Decisions [key: "d"]
///     |- Contested Resource [key: "c"]
///        |- End date Queries [key: "e"]
///        |- Identifier Votes Query [key: "i"]
///
///

/// A subtree made for polls to the network that represent decisions.
pub const VOTE_DECISIONS_TREE_KEY: char = 'd';

/// A subtree made for contested resources that will be voted on.
pub const CONTESTED_RESOURCE_TREE_KEY: char = 'c';

/// A subtree made for being able to query the end date of votes
pub const END_DATE_QUERIES_TREE_KEY: char = 'e';

/// A subtree made for being able to query votes that an identity has made
pub const IDENTITY_VOTES_TREE_KEY: char = 'i';

/// the root path of the voting branch
pub fn vote_root_path<'a>() -> [&'a [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Votes)]
}

/// the root path of the voting branch as a vec
pub fn vote_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Votes as u8]]
}

/// the decisions path of the voting branch
pub fn vote_decisions_tree_path<'a>() -> [&'a [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[VOTE_DECISIONS_TREE_KEY as u8],
    ]
}

/// the decisions path of the voting branch as a vec
pub fn vote_decisions_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![VOTE_DECISIONS_TREE_KEY as u8],
    ]
}

/// the contested resource tree path of the voting branch
pub fn vote_contested_resource_tree_path<'a>() -> [&'a [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
    ]
}

/// the contested resource tree path of the voting branch as a vec
pub fn vote_contested_resource_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
    ]
}

/// the end dates of contested resources
pub fn vote_contested_resource_end_date_queries_tree_path<'a>() -> [&'a [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[END_DATE_QUERIES_TREE_KEY as u8],
    ]
}

/// the end dates of contested resources as a vec
pub fn vote_contested_resource_end_date_queries_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![END_DATE_QUERIES_TREE_KEY as u8],
    ]
}

/// the specific end date path query of a contested resources as a vec
/// there is no need to ever add a key to this
pub fn vote_contested_resource_end_date_queries_at_time_tree_path_vec(
    time: TimestampMillis,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![END_DATE_QUERIES_TREE_KEY as u8],
        time.to_be_bytes().to_vec(),
    ]
}

/// the identity votes of contested resources
pub fn vote_contested_resource_identity_votes_tree_path<'a>() -> [&'a [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[IDENTITY_VOTES_TREE_KEY as u8],
    ]
}

/// the identity votes of contested resources as a vec
pub fn vote_contested_resource_identity_votes_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![IDENTITY_VOTES_TREE_KEY as u8],
    ]
}

/// the qualified identity vote of contested resources
/// this is with the identity id at the end already
pub fn vote_contested_resource_identity_votes_tree_path_for_identity(
    identity_id: &[u8; 32],
) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[IDENTITY_VOTES_TREE_KEY as u8],
        identity_id,
    ]
}

/// the qualified identity vote of contested resources as a vec
/// this is with the identity id at the end already
pub fn vote_contested_resource_identity_votes_tree_path_for_identity_vec(
    identity_id: &[u8; 32],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![IDENTITY_VOTES_TREE_KEY as u8],
        identity_id.to_vec(),
    ]
}
