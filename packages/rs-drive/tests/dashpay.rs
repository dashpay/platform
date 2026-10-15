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
        let drive = setup_drive_with_initial_state_structure(None);

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
            platform_version,
        )
        .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }

    #[test]
    fn test_user_id_by_created_at_query() {
        let drive = setup_drive_with_initial_state_structure(None);

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
            platform_version,
        )
            .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }

    #[test]
    fn test_owner_id_query() {
        let drive = setup_drive_with_initial_state_structure(None);

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
            platform_version,
        )
        .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }

    #[test]
    fn test_owner_id_by_created_at_query() {
        let drive = setup_drive_with_initial_state_structure(None);

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
            platform_version,
        )
        .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }

    #[test]
    fn test_owner_id_and_to_user_id_query() {
        let drive = setup_drive_with_initial_state_structure(None);

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
            platform_version,
        )
        .expect("failed to create query");

        drive
            .query_documents(query, None, false, None, None)
            .expect("failed to query documents");
    }
}

#[path = "support/contact_request_blocks.rs"]
mod contact_request_blocks;

/// Proved contact-request pages through blocks that a page boundary cuts:
/// the shapes the SDK's contact-request sweep must read completely.
mod contact_request_paging {
    use super::contact_request_blocks::{
        populate, ContactBlocks, BLOCK_OF_100, BLOCK_OF_150, BLOCK_OF_7, DIRECTIONS,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::DocumentV0Getters;
    use dpp::identifier::Identifier;
    use dpp::platform_value::platform_value;
    use dpp::prelude::DataContract;
    use dpp::version::{PlatformVersion, ProtocolVersion, LATEST_VERSION};
    use drive::drive::Drive;
    use drive::query::{DriveDocumentQuery, OrderClause, WhereClause, WhereOperator};
    use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    const VERSIONS: [ProtocolVersion; 3] = [13, 14, LATEST_VERSION];

    /// The sets read, each with a page size that cuts or ends at one of its
    /// blocks, and how many requests a `startAfter` sweep loses at protocol
    /// version 13.
    const PAGED: [(&ContactBlocks, u16, usize); 3] = [
        (&BLOCK_OF_150, 100, 55),
        (&BLOCK_OF_100, 100, 0),
        (&BLOCK_OF_7, 2, 6),
    ];

    struct Fixture {
        drive: Drive,
        contract: DataContract,
        version: &'static PlatformVersion,
    }

    impl Fixture {
        fn new(protocol_version: ProtocolVersion) -> Self {
            let version = PlatformVersion::get(protocol_version).expect("protocol version");
            let drive = setup_drive_with_initial_state_structure(Some(version));
            let contract = populate(&drive, version);
            Fixture {
                drive,
                contract,
                version,
            }
        }

        /// One page of `field == identity`, ordered by `$createdAt`, proved
        /// by Drive and verified as a client verifies it: `(created_at, id)`
        /// of each row in proof order.
        fn proved_page(
            &self,
            field: &str,
            identity: Identifier,
            after: Option<u64>,
            start: Option<(Identifier, bool)>,
            limit: u16,
        ) -> Vec<(u64, Identifier)> {
            let mut where_clauses = vec![WhereClause {
                field: field.to_string(),
                operator: WhereOperator::Equal,
                value: platform_value!(identity),
            }];
            if let Some(after) = after {
                where_clauses.push(WhereClause {
                    field: "$createdAt".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: platform_value!(after),
                });
            }
            let query = DriveDocumentQuery::from_typed_clauses(
                where_clauses,
                vec![OrderClause {
                    field: "$createdAt".to_string(),
                    ascending: true,
                }],
                Some(limit),
                start.map(|(cursor, _)| cursor.to_buffer()),
                start.is_none_or(|(_, included)| included),
                None,
                &self.contract,
                self.contract
                    .document_type_for_name("contactRequest")
                    .expect("contactRequest type"),
                &self.drive.config,
                self.version,
            )
            .expect("contact-request query");
            let (proof, _) = query
                .clone()
                .execute_with_proof(&self.drive, None, None, self.version)
                .expect("proved page");
            let (root_hash, documents) = query
                .verify_proof(&proof, self.version)
                .expect("verified page");
            let state_root = self
                .drive
                .grove
                .root_hash(None, &self.version.drive.grove_version)
                .unwrap()
                .expect("state root");
            assert_eq!(root_hash, state_root);
            documents
                .iter()
                .map(|document| (document.created_at().expect("$createdAt"), document.id()))
                .collect()
        }

        /// Every request of `field == identity` (created after `after`), read
        /// as the SDK sweep reads them: continue with `startAt` the previous
        /// page's last row and no `$createdAt` clause, check and drop the
        /// cursor row, stop on a raw page shorter than `limit`.
        fn sweep_start_at(
            &self,
            field: &str,
            identity: Identifier,
            after: Option<u64>,
            limit: u16,
        ) -> Vec<Identifier> {
            let mut ids = Vec::new();
            let mut cursor = None;
            loop {
                let after = if cursor.is_none() { after } else { None };
                let page = self.proved_page(field, identity, after, cursor, limit);
                let mut rows = page.iter().map(|(_, id)| *id);
                if let Some((cursor, _)) = cursor {
                    assert_eq!(
                        rows.next(),
                        Some(cursor),
                        "a startAt page starts with its cursor"
                    );
                }
                ids.extend(rows);
                match page.last() {
                    Some((_, last)) if page.len() == limit as usize => cursor = Some((*last, true)),
                    _ => return ids,
                }
            }
        }

        /// The same sweep continued with `startAfter`: the shape the SDK sent
        /// before it moved to `startAt`.
        fn sweep_start_after(
            &self,
            field: &str,
            identity: Identifier,
            limit: u16,
        ) -> Vec<Identifier> {
            let mut ids = Vec::new();
            let mut cursor = None;
            loop {
                let page = self.proved_page(field, identity, None, cursor, limit);
                ids.extend(page.iter().map(|(_, id)| *id));
                match page.last() {
                    Some((_, last)) if page.len() == limit as usize => {
                        cursor = Some((*last, false))
                    }
                    _ => return ids,
                }
            }
        }
    }

    #[test]
    fn should_read_every_contact_request_through_cut_blocks_with_a_start_at_cursor() {
        for protocol_version in VERSIONS {
            let fixture = Fixture::new(protocol_version);
            for (set, limit, _) in PAGED {
                for field in DIRECTIONS {
                    let identity = set.identity(field);
                    let context = format!("protocol {protocol_version}, {field}, limit {limit}");
                    assert_eq!(
                        fixture.sweep_start_at(field, identity, None, limit),
                        set.ids_after(0),
                        "{context}"
                    );
                    // A first page after a high-water mark, then continuations
                    // without the `$createdAt` clause.
                    let after = set.blocks[0].0;
                    assert_eq!(
                        fixture.sweep_start_at(field, identity, Some(after), limit),
                        set.ids_after(after),
                        "{context}, after {after}"
                    );
                }
            }
        }
    }

    /// The control: up to protocol version 13 a `startAfter` continuation
    /// skips the rest of the cursor's block, the requests the SDK sweep lost;
    /// from 14 Drive bounds it by the cursor's id as well.
    #[test]
    fn should_skip_a_cut_block_with_a_start_after_cursor_only_up_to_protocol_version_13() {
        for protocol_version in VERSIONS {
            let fixture = Fixture::new(protocol_version);
            for (set, limit, lost) in PAGED {
                for field in DIRECTIONS {
                    let rows = set.rows();
                    let swept = fixture.sweep_start_after(field, set.identity(field), limit);
                    let context = format!("protocol {protocol_version}, {field}, limit {limit}");
                    if protocol_version > 13 {
                        assert_eq!(swept, set.ids_after(0), "{context}");
                        continue;
                    }
                    // Each page ends at a cut block; the next page resumes
                    // at the first block after it.
                    let mut expected = Vec::new();
                    let mut rest = rows.as_slice();
                    while !rest.is_empty() {
                        let (page, after) = rest.split_at(rest.len().min(limit as usize));
                        expected.extend(page.iter().map(|(_, id)| *id));
                        let cut = page.last().expect("page").0;
                        rest =
                            &after[after.partition_point(|(created_at, _)| *created_at == cut)..];
                    }
                    assert_eq!(swept, expected, "{context}");
                    assert_eq!(rows.len() - swept.len(), lost, "{context}");
                }
            }
        }
    }
}
