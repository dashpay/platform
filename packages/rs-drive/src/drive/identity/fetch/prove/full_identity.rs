use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use grovedb::TransactionArg;

impl Drive {
    /// Proves an identity with all its information from an identity id.
    pub fn prove_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let query = Self::full_identity_query(&identity_id)?;
        self.grove_get_proved_path_query(&query, transaction, &mut drive_operations)
    }

    /// Proves identities with all its information from an identity ids.
    pub fn proved_full_identities(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let query = Self::full_identities_query(identity_ids)?;
        self.grove_get_proved_path_query(&query, transaction, &mut drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::identity::fetch::full_identity::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::query_result_type::QueryResultType;
    use grovedb::QueryItem;
    use std::borrow::Borrow;
    use std::collections::BTreeMap;
    use std::ops::RangeFull;
    use dpp::identity::Identity;
    use grovedb::GroveDb;
    use crate::drive::block_info::BlockInfo;

    mod fetch_proved_full_identities {
        use super::*;



        #[test]
        fn test_proved_two_full_identities_query_no_tx() {
            let drive = setup_drive_with_initial_state_structure();

            let identities: BTreeMap<[u8; 32], Option<Identity>> =
                Identity::random_identities(2, 5, Some(14))
                    .into_iter()
                    .map(|identity| (identity.id.to_buffer(), Some(identity)))
                    .collect();

            for identity in identities.values() {
                drive
                    .add_new_identity(
                        identity.as_ref().unwrap().clone(),
                        &BlockInfo::default(),
                        true,
                        None,
                    )
                    .expect("expected to add an identity");
            }

            let path_query = Drive::full_identities_query(
                identities
                    .keys()
                    .copied()
                    .collect::<Vec<[u8; 32]>>()
                    .borrow(),
            )
            .expect("expected to get query");

            let (elements, _) = drive
                .grove
                .query_raw(
                    &path_query,
                    true,
                    QueryResultType::QueryPathKeyElementTrioResultType,
                    None,
                )
                .unwrap()
                .expect("expected to run the path query");
            assert_eq!(elements.len(), 14);

            let fetched_identities = drive
                .proved_full_identities(
                    identities
                        .keys()
                        .copied()
                        .collect::<Vec<[u8; 32]>>()
                        .borrow(),
                    None,
                )
                .expect("should fetch an identity");

            let (_hash, proof) = GroveDb::verify_query(fetched_identities.as_slice(), &path_query)
                .expect("expected to verify query");

            // We want to get a proof on the balance, the revision and 5 keys
            assert_eq!(proof.len(), 14);
        }

        #[test]
        fn test_proved_ten_full_identities_query_no_tx() {
            let drive = setup_drive_with_initial_state_structure();

            let identities: BTreeMap<[u8; 32], Option<Identity>> =
                Identity::random_identities(10, 5, Some(14))
                    .into_iter()
                    .map(|identity| (identity.id.to_buffer(), Some(identity)))
                    .collect();

            for identity in identities.values() {
                drive
                    .add_new_identity(
                        identity.as_ref().unwrap().clone(),
                        &BlockInfo::default(),
                        true,
                        None,
                    )
                    .expect("expected to add an identity");
            }

            let path_query = Drive::full_identities_query(
                identities
                    .keys()
                    .copied()
                    .collect::<Vec<[u8; 32]>>()
                    .borrow(),
            )
            .expect("expected to get query");

            let query = path_query.query.query.clone();
            assert_eq!(path_query.path, Vec::<Vec<u8>>::new()); // it splits at the root
            assert_eq!(query.items.len(), 2); // 32 (Identities) and 96 (Balances)

            let conditional_subqueries = query
                .conditional_subquery_branches
                .expect("expected to have conditional subqueries");

            assert_eq!(conditional_subqueries.len(), 2); // 32 (Identities) and 96 (Balances)

            let (_, identity_conditional_subquery) = conditional_subqueries.last().unwrap();
            let (_, balance_conditional_subquery) = conditional_subqueries.first().unwrap();

            // Lets start with balance
            // There should be no subquery path

            assert_eq!(balance_conditional_subquery.subquery_path, None);

            // There should be a subquery pointing to the identity id

            let balance_subquery = *balance_conditional_subquery
                .subquery
                .clone()
                .expect("expected balances to have a subquery");

            assert_eq!(balance_subquery.conditional_subquery_branches, None);

            assert_eq!(balance_subquery.items.len(), 10);
            assert_eq!(
                balance_subquery
                    .items
                    .into_iter()
                    .map(|query_item| {
                        query_item
                            .keys()
                            .expect("expected to get keys of query item")
                            .first()
                            .unwrap()
                            .clone()
                            .try_into()
                            .unwrap()
                    })
                    .collect::<Vec<[u8; 32]>>(),
                identities.keys().copied().collect::<Vec<[u8; 32]>>()
            );

            // Moving on to Identity subquery

            // The subquery path is our identity
            assert_eq!(identity_conditional_subquery.subquery_path, None,);

            // We should have 10 conditional subqueries

            let identities_subquery = *identity_conditional_subquery
                .subquery
                .clone()
                .expect("expected identities to have a subquery");

            assert_eq!(identities_subquery.items.len(), 10); // This query is for the 10 identities

            let identities_conditional_subqueries = identities_subquery
                .conditional_subquery_branches
                .expect("expected to have conditional subqueries");

            // We only subquery the keys
            assert_eq!(identities_conditional_subqueries.len(), 10);

            // Let's check one out
            let (_, identity_conditional_subquery) =
                identities_conditional_subqueries.first().unwrap();

            let identity_subquery = *identity_conditional_subquery
                .subquery
                .clone()
                .expect("expected identities to have a subquery");

            assert_eq!(identity_subquery.items.len(), 2); // This query is for the Revision 0 and Keys 1

            let identity_conditional_subqueries = identity_subquery
                .conditional_subquery_branches
                .expect("expected to have conditional subqueries");

            // We only subquery the keys
            assert_eq!(identity_conditional_subqueries.len(), 1);

            let (_, identity_keys_conditional_subquery) =
                identity_conditional_subqueries.first().unwrap();

            assert_eq!(identity_keys_conditional_subquery.subquery_path, None);

            // We are requesting all keys
            //todo: maybe we shouldn't be

            let identity_keys_subquery = *identity_keys_conditional_subquery
                .subquery
                .clone()
                .expect("expected identities to have a subquery");

            assert_eq!(
                identity_keys_subquery.items.first().unwrap(),
                &QueryItem::RangeFull(RangeFull)
            );

            let (elements, _) = drive
                .grove
                .query_raw(
                    &path_query,
                    true,
                    QueryResultType::QueryPathKeyElementTrioResultType,
                    None,
                )
                .unwrap()
                .expect("expected to run the path query");
            assert_eq!(elements.len(), 70);

            let fetched_identities = drive
                .proved_full_identities(
                    identities
                        .keys()
                        .copied()
                        .collect::<Vec<[u8; 32]>>()
                        .borrow(),
                    None,
                )
                .expect("should fetch an identity");

            let (_hash, proof) = GroveDb::verify_query(fetched_identities.as_slice(), &path_query)
                .expect("expected to verify query");

            // We want to get a proof on the balance, the revision and 5 keys
            assert_eq!(proof.len(), 70);
        }
    }

    mod fetch_proved_full_identity {
        use super::*;
        use crate::drive::block_info::BlockInfo;
        use grovedb::query_result_type::QueryResultType;
        use grovedb::QueryItem;
        use std::ops::RangeFull;

        #[test]
        fn test_full_identity_query_construction() {
            let identity = Identity::random_identity(5, Some(12345));
            let query = Drive::full_identity_query(identity.id.as_bytes())
                .expect("expected to make the query");
        }
        #[test]
        fn test_proved_full_identity_query_no_tx() {
            let drive = setup_drive_with_initial_state_structure();

            let identity = Identity::random_identity(5, Some(12345));

            drive
                .add_new_identity(identity.clone(), &BlockInfo::default(), true, None)
                .expect("expected to insert identity");

            let path_query = Drive::full_identity_query(identity.id.as_bytes())
                .expect("expected to make the query");

            // The query is querying
            //                     root
            //            /                     \
            //     identities                     balances
            //      /                                        \
            //                                             identity_id

            let query = path_query.query.query.clone();
            assert_eq!(path_query.path, Vec::<Vec<u8>>::new()); // it splits at the root
            assert_eq!(query.items.len(), 2); // 32 (Identities) and 96 (Balances)

            let conditional_subqueries = query
                .conditional_subquery_branches
                .expect("expected to have conditional subqueries");

            assert_eq!(conditional_subqueries.len(), 2); // 32 (Identities) and 96 (Balances)

            let (_, identity_conditional_subquery) = conditional_subqueries.last().unwrap();
            let (_, balance_conditional_subquery) = conditional_subqueries.first().unwrap();

            // Lets start with balance
            // There should be no subquery path

            assert_eq!(balance_conditional_subquery.subquery_path, None);

            // There should be a subquery pointing to the identity id

            let balance_subquery = *balance_conditional_subquery
                .subquery
                .clone()
                .expect("expected balances to have a subquery");

            assert_eq!(balance_subquery.conditional_subquery_branches, None);

            assert_eq!(balance_subquery.items.len(), 1);
            assert_eq!(
                balance_subquery.items.first().unwrap(),
                &QueryItem::Key(identity.id.to_buffer().to_vec())
            );

            // Moving on to Identity subquery

            // The subquery path is our identity
            assert_eq!(
                identity_conditional_subquery.subquery_path,
                Some(vec![identity.id.to_buffer().to_vec()])
            );

            let identity_subquery = *identity_conditional_subquery
                .subquery
                .clone()
                .expect("expected identities to have a subquery");

            assert_eq!(identity_subquery.items.len(), 2); // This query is for the Revision 0 and Keys 1

            let identity_conditional_subqueries = identity_subquery
                .conditional_subquery_branches
                .expect("expected to have conditional subqueries");

            // We only subquery the keys
            assert_eq!(identity_conditional_subqueries.len(), 1);

            let (_, identity_keys_conditional_subquery) =
                identity_conditional_subqueries.first().unwrap();

            assert_eq!(identity_keys_conditional_subquery.subquery_path, None);

            // We are requesting all keys
            //todo: maybe we shouldn't be

            let identity_keys_subquery = *identity_keys_conditional_subquery
                .subquery
                .clone()
                .expect("expected identities to have a subquery");

            assert_eq!(
                identity_keys_subquery.items.first().unwrap(),
                &QueryItem::RangeFull(RangeFull)
            );

            let (elements, _) = drive
                .grove
                .query_raw(
                    &path_query,
                    true,
                    QueryResultType::QueryPathKeyElementTrioResultType,
                    None,
                )
                .unwrap()
                .expect("expected to run the path query");
            assert_eq!(elements.len(), 7);

            let fetched_identity = drive
                .prove_full_identity(identity.id.to_buffer(), None)
                .expect("should fetch an identity");

            let (_hash, proof) = GroveDb::verify_query(fetched_identity.as_slice(), &path_query)
                .expect("expected to verify query");

            // We want to get a proof on the balance, the revision and 5 keys
            assert_eq!(proof.len(), 7);
        }
    }
}
