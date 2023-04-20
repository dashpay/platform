use crate::drive::balances::balance_path_vec;
use crate::drive::Drive;
use grovedb::PathQuery;

impl Drive {
    /// The query for proving the identities balance from an identity id.
    pub fn balance_for_identity_id_query(identity_id: [u8; 32]) -> PathQuery {
        let balance_path = balance_path_vec();
        PathQuery::new_single_key(balance_path, identity_id.to_vec())
    }

    /// The query for proving the identities balance and revision from an identity id.
    pub fn balance_and_revision_for_identity_id_query(identity_id: [u8; 32]) -> PathQuery {
        let balance_path_query = Self::balance_for_identity_id_query(identity_id);
        let revision_path_query = Self::identity_revision_query(&identity_id);
        //todo: lazy static this
        PathQuery::merge(vec![&balance_path_query, &revision_path_query]).unwrap()
    }
}
