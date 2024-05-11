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

pub const VOTE_DECISIONS_TREE_KEY: char = 'd';

pub const CONTESTED_RESOURCE_TREE_KEY: char = 'c';

pub const END_DATE_QUERIES_TREE_KEY: char = 'e';

pub const IDENTITY_VOTES_TREE_KEY: char = 'i';

pub(in crate::drive::votes) fn vote_root_path<'a>() -> [&'a [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Votes)]
}

pub(in crate::drive::votes) fn vote_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Votes as u8]]
}

pub(in crate::drive::votes) fn vote_decisions_tree_path<'a>() -> [&'a [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[VOTE_DECISIONS_TREE_KEY as u8],
    ]
}

pub(in crate::drive::votes) fn vote_decisions_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![VOTE_DECISIONS_TREE_KEY as u8],
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_tree_path<'a>() -> [&'a [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_end_date_queries_tree_path<'a>(
) -> [&'a [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[END_DATE_QUERIES_TREE_KEY as u8],
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_end_date_queries_tree_path_vec(
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![END_DATE_QUERIES_TREE_KEY as u8],
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_end_date_queries_at_time_tree_path_vec(
    time: TimestampMillis,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![END_DATE_QUERIES_TREE_KEY as u8],
        time.to_be_bytes().to_vec(),
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_identity_votes_tree_path<'a>(
) -> [&'a [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[IDENTITY_VOTES_TREE_KEY as u8],
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_identity_votes_tree_path_vec() -> Vec<Vec<u8>>
{
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![IDENTITY_VOTES_TREE_KEY as u8],
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_identity_votes_tree_path_for_identity(
    identity_id: &[u8; 32],
) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[IDENTITY_VOTES_TREE_KEY as u8],
        identity_id,
    ]
}

pub(in crate::drive::votes) fn vote_contested_resource_identity_votes_tree_path_for_identity_vec(
    identity_id: &[u8; 32],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![IDENTITY_VOTES_TREE_KEY as u8],
        identity_id.to_vec(),
    ]
}
