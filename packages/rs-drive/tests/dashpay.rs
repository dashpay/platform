use dpp::block::block_info::BlockInfo;
use dpp::data_contracts::SystemDataContract;
use dpp::identifier::Identifier;
use dpp::system_data_contracts::load_system_data_contract;
use drive::query::DriveDocumentQuery;
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use platform_version::version::PlatformVersion;

mod contact_request {
    use super::*;

    #[test]
    fn test_user_id_query() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Dashpay, platform_version)
                .expect("failed to load system data contract");

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("should apply contract");

        let query = DriveDocumentQuery::from_sql_expr(
            &format!(
                "SELECT * FROM contactRequest WHERE toUserId == '{}'",
                Identifier::random()
            ),
            &data_contract,
            None,
        )
        .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }

    #[test]
    fn test_user_id_by_created_at_query() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Dashpay, platform_version)
                .expect("failed to load system data contract");

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("should apply contract");

        let query = DriveDocumentQuery::from_sql_expr(
            &format!(
                "SELECT * FROM contactRequest WHERE toUserId == '{}' AND $createdAt > 100 ORDER BY $createdAt asc",
                Identifier::random()
            ),
            &data_contract,
            None,
        )
            .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }

    #[test]
    fn test_owner_id_query() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Dashpay, platform_version)
                .expect("failed to load system data contract");

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("should apply contract");

        let query = DriveDocumentQuery::from_sql_expr(
            &format!(
                "SELECT * FROM contactRequest WHERE $ownerId == '{}'",
                Identifier::random()
            ),
            &data_contract,
            None,
        )
        .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }

    #[test]
    fn test_owner_id_by_created_at_query() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Dashpay, platform_version)
                .expect("failed to load system data contract");

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("should apply contract");

        let query = DriveDocumentQuery::from_sql_expr(
            &format!(
                "SELECT * FROM contactRequest WHERE $ownerId == '{}' AND $createdAt > 100 ORDER BY $createdAt asc",
                Identifier::random()
            ),
            &data_contract,
            None,
        )
        .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }

    #[test]
    fn test_owner_id_and_to_user_id_query() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Dashpay, platform_version)
                .expect("failed to load system data contract");

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("should apply contract");

        let query = DriveDocumentQuery::from_sql_expr(
            &format!(
                "SELECT * FROM contactRequest WHERE $ownerId == '{}' AND toUserId == '{}'",
                Identifier::random(),
                Identifier::random()
            ),
            &data_contract,
            None,
        )
        .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }
}
