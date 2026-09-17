use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_readiness_path_vec;
use crate::drive::votes::paths::{
    readiness_contract_tree_path_vec, readiness_round_reports_tree_path_vec,
    readiness_round_tree_path_vec, READINESS_CURRENT_ROUND_POINTER_KEY,
    READINESS_ROUND_RECORD_KEY, READINESS_ROUND_REPORTS_TREE_KEY,
};
use grovedb::{PathQuery, Query, SizedQuery};

/// The path query proving one readiness fund.
pub fn readiness_fund_path_query(fund_id: [u8; 32]) -> PathQuery {
    let mut path_query = PathQuery::new_single_key(
        prefunded_specialized_balances_for_readiness_path_vec(),
        fund_id.to_vec(),
    );
    path_query.query.limit = Some(1);
    path_query
}

/// The path query proving a contract's current round pointer.
pub fn readiness_round_pointer_path_query(contract_id: [u8; 32]) -> PathQuery {
    let mut path_query = PathQuery::new_single_key(
        readiness_contract_tree_path_vec(contract_id),
        vec![READINESS_CURRENT_ROUND_POINTER_KEY as u8],
    );
    path_query.query.limit = Some(1);
    path_query
}

/// The path query proving a round's record and its reports count tree element (whose count
/// is the raw distinct report count).
pub fn readiness_round_path_query(contract_id: [u8; 32], round_id: [u8; 32]) -> PathQuery {
    let mut query = Query::new();
    query.insert_key(vec![READINESS_ROUND_RECORD_KEY]);
    query.insert_key(vec![READINESS_ROUND_REPORTS_TREE_KEY]);
    PathQuery::new(
        readiness_round_tree_path_vec(contract_id, round_id),
        SizedQuery::new(query, Some(2), None),
    )
}

/// The path query proving one report of a round.
pub fn readiness_report_path_query(
    contract_id: [u8; 32],
    round_id: [u8; 32],
    pro_tx_hash: [u8; 32],
) -> PathQuery {
    let mut path_query = PathQuery::new_single_key(
        readiness_round_reports_tree_path_vec(contract_id, round_id),
        pro_tx_hash.to_vec(),
    );
    path_query.query.limit = Some(1);
    path_query
}
