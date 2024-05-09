use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves an identity with all its information from an identity id.
    pub(super) fn prove_full_identity_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let query = Self::full_identity_query(&identity_id)?;
        self.grove_get_proved_path_query(
            &query,
            false,
            transaction,
            &mut drive_operations,
            drive_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;

    use crate::drive::Drive;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;

    use dpp::version::PlatformVersion;
    use grovedb::query_result_type::QueryResultType;
    use grovedb::GroveDb;
    use grovedb::QueryItem;

    use std::ops::RangeFull;

    #[test]
    fn should_prove_full_identity_query_no_tx() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(5, Some(14), platform_version)
            .expect("expected a random identity");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert identity");

        let path_query = Drive::full_identity_query(identity.id().as_bytes())
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
            &QueryItem::Key(identity.id().to_buffer().to_vec())
        );

        // Moving on to Identity subquery

        // The subquery path is our identity
        assert_eq!(
            identity_conditional_subquery.subquery_path,
            Some(vec![identity.id().to_buffer().to_vec()])
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
                true,
                true,
                QueryResultType::QueryPathKeyElementTrioResultType,
                None,
            )
            .unwrap()
            .expect("expected to run the path query");
        assert_eq!(elements.len(), 7);

        let fetched_identity = drive
            .prove_full_identity_v0(identity.id().to_buffer(), None, &platform_version.drive)
            .expect("should fetch an identity");

        let (_hash, proof) = GroveDb::verify_query(fetched_identity.as_slice(), &path_query)
            .expect("expected to verify query");

        // We want to get a proof on the balance, the revision and 5 keys
        assert_eq!(proof.len(), 7);
    }
}
