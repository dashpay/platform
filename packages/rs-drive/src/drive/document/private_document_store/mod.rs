//! The store of a private document type (protocol version 15). A private type's entries are
//! fixed-size encrypted blobs the chain never reads, appended by an anonymous transition paid
//! from the shielded pool. They live in a grovedb `PrivateDocumentStore` at the type's key
//! `[0]`, where every other type keeps its primary-key tree, and the type has no index trees.

#[cfg(feature = "server")]
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::util::grove_operations::DirectQueryType;
#[cfg(feature = "server")]
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{DocumentTypeRef, PrivateDocumentConfig};
use dpp::version::PlatformVersion;
#[cfg(feature = "server")]
use grovedb::{Element, TransactionArg};
#[cfg(feature = "server")]
use v0::insert_empty_private_document_store_operation_v0;
use v0::private_document_store_chunk_power_v0;

#[cfg(feature = "server")]
mod fetch_private_document_entries;
#[cfg(feature = "server")]
mod prove_private_document_entries;
mod v0;

/// The bytes Drive writes before each entry a client appends: the block height (u64, little
/// endian) and the block time in milliseconds (u64, little endian). They give the recipient a
/// time it can trust, which nothing written inside a ciphertext could be, and leak nothing:
/// the block's height and time are public already. The size of a layout, not a tunable
/// number: a different header is a different entry format, with its own append generation.
pub const PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE: u32 = 16;

/// The key of a private type's store under its document type tree: the key every other type
/// keeps its primary-key tree at.
pub const PRIVATE_DOCUMENT_STORE_KEY: u8 = 0;

/// The bytes each stored entry of a private type takes: Drive's header and the client's entry.
pub fn private_document_stored_entry_size(
    private_document: PrivateDocumentConfig,
) -> Result<u32, Error> {
    private_document
        .entry_size()
        .checked_add(PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE)
        .ok_or_else(|| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "a private document type declares entries of {} bytes, more than a store holds",
                private_document.entry_size()
            )))
        })
}

/// One entry of a private type's store as a page returns it: its position in append order,
/// Drive's header split out, and the client's entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateDocumentEntry {
    /// The entry's position in the store, from 0 in append order.
    pub position: u64,
    /// The height of the block that appended the entry, from Drive's header.
    pub block_height: u64,
    /// The time in milliseconds of the block that appended the entry, from Drive's header.
    pub block_time_ms: u64,
    /// The client's entry: `entrySize` bytes the chain never reads.
    pub data: Vec<u8>,
}

/// The bytes a private type's store holds for an entry: Drive's header, the height and then
/// the time in milliseconds of the block appending the entry (each a u64 little endian), and
/// the client's entry. The one layout the append writes and
/// [`PrivateDocumentEntry::from_stored`] reads.
pub fn encode_stored_private_document_entry(
    block_height: u64,
    block_time_ms: u64,
    entry: &[u8],
) -> Vec<u8> {
    let mut stored = Vec::with_capacity(
        (PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE as usize).saturating_add(entry.len()),
    );
    stored.extend_from_slice(&block_height.to_le_bytes());
    stored.extend_from_slice(&block_time_ms.to_le_bytes());
    stored.extend_from_slice(entry);
    stored
}

impl PrivateDocumentEntry {
    /// Splits an entry as the store holds it (`encode_stored_private_document_entry`) into
    /// Drive's header and the client's entry. The header is removed in place, so the entry
    /// keeps the stored bytes' allocation. `None` when the stored bytes are not
    /// `16 + entrySize` long.
    pub(crate) fn from_stored(
        position: u64,
        mut stored: Vec<u8>,
        private_document: PrivateDocumentConfig,
    ) -> Option<Self> {
        let header_size = PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE as usize;
        if stored.len() != header_size.checked_add(private_document.entry_size() as usize)? {
            return None;
        }
        let block_height = u64::from_le_bytes(stored[..8].try_into().ok()?);
        let block_time_ms = u64::from_le_bytes(stored[8..header_size].try_into().ok()?);
        stored.drain(..header_size);
        Some(Self {
            position,
            block_height,
            block_time_ms,
            data: stored,
        })
    }
}

/// A page of a private type's store: its entries in append order, and how many entries the
/// store holds, so a page shorter than asked for ends at the store's last entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivateDocumentEntries {
    /// The page's entries, in append order.
    pub entries: Vec<PrivateDocumentEntry>,
    /// How many entries the store holds.
    pub total_count: u64,
}

/// The private document declaration of `document_type`, which the entries of a store are read
/// for; a query naming a type that is not private is refused.
pub(crate) fn private_document_of(
    document_type: DocumentTypeRef,
) -> Result<PrivateDocumentConfig, Error> {
    document_type.private_document().ok_or_else(|| {
        Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "\"{}\" is not a private document type: it has no store of entries",
            document_type.name()
        )))
    })
}

/// How many entries a page of a private type's store reads from `start_position`: the rest of
/// the chunk holding it, to the chunk's end (`2^chunk_power` entries to a chunk). A page so
/// reads at most one compacted chunk, and a reader reads from its cursor wherever it is. The
/// server reading or proving a page and a client verifying it both settle the page here, from
/// the chunk power the store committed when its contract created it: the server reads it from
/// the store element and a client from the element the proof binds, since a later protocol
/// version may size new stores differently but a store keeps its chunks.
///
/// A proof of a page in the store's buffered tail, or past its end, also carries the store's
/// first compacted chunk, which GroveDB proves to anchor the root of its chunks, so such a
/// proof can reach about two chunks.
pub(crate) fn private_document_entries_page_limit(
    chunk_power: u8,
    start_position: u64,
) -> Result<u16, Error> {
    // GroveDB refuses a store whose chunk power is not 1 to 16
    let chunk_size = 1u64.checked_shl(u32::from(chunk_power)).ok_or_else(|| {
        Error::Drive(DriveError::CorruptedDriveState(format!(
            "a store's chunk power is at most 16, not {chunk_power}"
        )))
    })?;
    let limit = chunk_size - start_position % chunk_size;
    // A page reads at most u16::MAX entries; the store limits keep every chunk within that
    // (`should_keep_a_whole_chunk_within_one_page`)
    u16::try_from(limit).map_err(|_| {
        Error::Drive(DriveError::CorruptedDriveState(format!(
            "a page reads at most {} entries, not the {limit} left of a chunk of {chunk_size}",
            u16::MAX
        )))
    })
}

#[cfg(feature = "server")]
impl Drive {
    /// The chunk power the store of the private type at `type_path` committed when its
    /// contract created it, which settles a page of the store
    /// (`private_document_entries_page_limit`).
    fn committed_private_document_store_chunk_power(
        &self,
        type_path: [&[u8]; 4],
        document_type_name: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u8, Error> {
        let element = self.grove_get_raw(
            type_path.as_slice().into(),
            &[PRIVATE_DOCUMENT_STORE_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        match element.as_ref().map(Element::underlying) {
            Some(Element::PrivateDocumentStore(_, _, chunk_power, _)) => Ok(*chunk_power),
            _ => Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "the private document type \"{document_type_name}\" has no store of entries"
            )))),
        }
    }
}

/// The chunk power of a private type's store whose stored entries take `stored_entry_size`
/// bytes, as the generation of `insert_empty_private_document_store_operation` that
/// `platform_version` selects sizes it when it creates the store. Drive's layout of a private
/// type reads it too.
pub fn private_document_store_chunk_power(
    stored_entry_size: u32,
    platform_version: &PlatformVersion,
) -> Result<u8, Error> {
    match platform_version
        .drive
        .methods
        .document
        .insert_empty_private_document_store_operation
    {
        Some(0) => private_document_store_chunk_power_v0(stored_entry_size, platform_version),
        Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
            method: "private_document_store_chunk_power".to_string(),
            known_versions: vec![0],
            received: version,
        })),
        None => Err(Error::Drive(DriveError::VersionNotActive {
            method: "private_document_store_chunk_power".to_string(),
            known_versions: vec![0],
        })),
    }
}

/// The operation creating the empty store of a private type under its document type tree,
/// `type_path`, sized from `platform_version`'s private document store limits. Only a protocol
/// version admitting private types selects a version of it.
#[cfg(feature = "server")]
pub fn insert_empty_private_document_store_operation(
    type_path: [&[u8]; 4],
    private_document: PrivateDocumentConfig,
    storage_flags: Option<&StorageFlags>,
    platform_version: &PlatformVersion,
) -> Result<LowLevelDriveOperation, Error> {
    match platform_version
        .drive
        .methods
        .document
        .insert_empty_private_document_store_operation
    {
        Some(0) => insert_empty_private_document_store_operation_v0(
            type_path,
            private_document,
            storage_flags,
            platform_version,
        ),
        Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
            method: "insert_empty_private_document_store_operation".to_string(),
            known_versions: vec![0],
            received: version,
        })),
        None => Err(Error::Drive(DriveError::VersionNotActive {
            method: "insert_empty_private_document_store_operation".to_string(),
            known_versions: vec![0],
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::{Drive, RootTree};
    use crate::error::query::QuerySyntaxError;
    use crate::query::DriveDocumentQuery;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
    use dpp::data_contract::DataContract;
    use dpp::fee::fee_result::FeeResult;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::platform_value::Identifier;
    use dpp::version::{PlatformVersion, PLATFORM_VERSIONS};
    use grovedb::Element;
    use grovedb_path::SubtreePath;
    use serde_json::json;
    use std::ops::Range;

    const CONTRACT_ID: [u8; 32] = [7; 32];

    /// A contract of `version` holding a public `note` type and, when `with_secret`, the private
    /// `secret` type of 585-byte entries.
    fn contract(version: u32, with_secret: bool) -> DataContract {
        contract_with_entry_size(version, with_secret, 585)
    }

    /// `contract`, with the private `secret` type's entries of `entry_size` bytes.
    fn contract_with_entry_size(version: u32, with_secret: bool, entry_size: u32) -> DataContract {
        let mut document_schemas = json!({
            "note": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 63, "position": 0 }
                },
                "additionalProperties": false
            }
        });
        if with_secret {
            document_schemas["secret"] = json!({
                "type": "object",
                "private": { "entrySize": entry_size },
                "documentsMutable": false,
                "canBeDeleted": false,
                "properties": {
                    "accountReference": {
                        "type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 0
                    }
                },
                "additionalProperties": false
            });
        }
        let contract = json!({
            "$formatVersion": "1",
            "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
            "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
            "version": version,
            "documentSchemas": document_schemas
        });
        DataContract::from_value(
            dpp::platform_value::to_value(contract).expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        )
        .expect("the contract registers")
    }

    /// The element at the key `[0]` of the document type `type_name`.
    fn element_at_key_zero(drive: &Drive, type_name: &str) -> Element {
        let type_path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            &CONTRACT_ID,
            &[1],
            type_name.as_bytes(),
        ];
        drive
            .grove
            .get_raw(
                SubtreePath::from(&type_path[..]),
                &[PRIVATE_DOCUMENT_STORE_KEY],
                None,
                &PlatformVersion::latest().drive.grove_version,
            )
            .unwrap()
            .expect("expected an element at the type's key [0]")
    }

    fn assert_holds_an_empty_store_of_585_byte_entries(drive: &Drive) {
        let element = element_at_key_zero(drive, "secret");
        assert!(
            matches!(element, Element::PrivateDocumentStore(0, 601, 9, _)),
            "expected an empty store of 16 + 585 byte entries at chunk power 9, got {element:?}"
        );
    }

    #[test]
    fn should_create_the_store_when_a_contract_is_registered() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        drive
            .apply_contract(
                &contract(1, true),
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");

        assert_holds_an_empty_store_of_585_byte_entries(&drive);
        // The public type of the same contract keeps its primary-key tree
        assert!(element_at_key_zero(&drive, "note").is_any_tree());
    }

    #[test]
    fn should_create_the_store_of_a_private_type_an_update_adds() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        drive
            .apply_contract(
                &contract(1, false),
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");
        drive
            .apply_contract(
                &contract(2, true),
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to update the contract");

        assert_holds_an_empty_store_of_585_byte_entries(&drive);
    }

    /// Registers `contract` on `drive`, or estimates its registration when `apply` is false.
    fn register(drive: &Drive, contract: &DataContract, apply: bool) -> FeeResult {
        drive
            .apply_contract(
                contract,
                BlockInfo::default(),
                apply,
                None,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to register the contract, or to estimate it")
    }

    fn assert_the_estimate_covers(estimated: &FeeResult, applied: &FeeResult) {
        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "estimated processing {} under the applied {}",
            estimated.processing_fee,
            applied.processing_fee
        );
        assert!(
            estimated.storage_fee >= applied.storage_fee,
            "estimated storage {} under the applied {}",
            estimated.storage_fee,
            applied.storage_fee
        );
    }

    /// The fee of registering a contract with a private type, or of an update adding one, is
    /// estimated without the store existing yet, as check_tx estimates it, and the estimate
    /// covers what creating the store costs.
    #[test]
    fn should_estimate_a_registration_creating_a_store_at_its_applied_fees_or_more() {
        let estimated = register(
            &setup_drive_with_initial_state_structure(None),
            &contract(1, true),
            false,
        );
        let applied = register(
            &setup_drive_with_initial_state_structure(None),
            &contract(1, true),
            true,
        );
        assert_the_estimate_covers(&estimated, &applied);
    }

    #[test]
    fn should_estimate_an_update_creating_a_store_at_its_applied_fees_or_more() {
        let estimating_drive = setup_drive_with_initial_state_structure(None);
        let applying_drive = setup_drive_with_initial_state_structure(None);
        register(&estimating_drive, &contract(1, false), true);
        register(&applying_drive, &contract(1, false), true);

        let estimated = register(&estimating_drive, &contract(2, true), false);
        let applied = register(&applying_drive, &contract(2, true), true);
        assert_the_estimate_covers(&estimated, &applied);
    }

    #[test]
    fn should_refuse_a_document_query_on_a_private_type() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = contract(1, true);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");

        let query = DriveDocumentQuery::from_sql_expr(
            "select * from secret",
            &contract,
            Some(&drive.config),
            platform_version,
        )
        .expect("expected the query to parse");
        let refusal = query
            .construct_path_query(None, platform_version)
            .expect_err("a private type has no documents to query");
        assert!(
            matches!(
                &refusal,
                Error::Query(QuerySyntaxError::Unsupported(message))
                    if message.contains("private document type")
            ),
            "{refusal:?}"
        );
        let refusal = drive
            .query_documents(
                query,
                None,
                false,
                None,
                Some(platform_version.protocol_version),
            )
            .expect_err("a private type has no documents to query");
        assert!(
            matches!(refusal, Error::Query(QuerySyntaxError::Unsupported(_))),
            "{refusal:?}"
        );
    }

    /// A private type keeps no documents and no history: Drive's history query refuses it,
    /// for a fetch, a proof and a proof's verification.
    #[test]
    fn should_refuse_the_document_history_of_a_private_type() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = contract(1, true);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");
        let refused = |result: Result<(), Error>| {
            matches!(
                result,
                Err(Error::Query(QuerySyntaxError::Unsupported(message)))
                    if message.contains("private document type")
            )
        };

        assert!(refused(
            drive
                .fetch_document_history(
                    CONTRACT_ID,
                    secret,
                    [1; 32],
                    None,
                    0,
                    Some(10),
                    None,
                    platform_version,
                )
                .map(|_| ())
        ));
        assert!(refused(
            drive
                .prove_document_history(
                    CONTRACT_ID,
                    secret,
                    [1; 32],
                    None,
                    0,
                    Some(10),
                    None,
                    platform_version,
                )
                .map(|_| ())
        ));
        assert!(refused(
            Drive::verify_document_history(
                &[],
                CONTRACT_ID,
                "secret",
                secret,
                [1; 32],
                0,
                Some(10),
                None,
                platform_version,
            )
            .map(|_| ())
        ));
    }

    /// A drive holding the contract with the private `secret` type of 4096-byte entries,
    /// whose store keeps 64 entries to a chunk, and `count` entries appended to it, entry `i`
    /// behind the header of block `100 + i` at time `1_000 * i` and filled with byte `i`.
    fn drive_with_private_entries(count: u8) -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = contract_with_entry_size(1, true, 4096);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");
        let type_path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            &CONTRACT_ID,
            &[1],
            b"secret",
        ];
        for i in 0..count {
            drive
                .grove
                .private_document_store_insert(
                    type_path.as_slice(),
                    &[PRIVATE_DOCUMENT_STORE_KEY],
                    encode_stored_private_document_entry(
                        100 + u64::from(i),
                        1_000 * u64::from(i),
                        &[i; 4096],
                    ),
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("expected to append an entry");
        }
        (drive, contract)
    }

    fn expected_entries(positions: Range<u8>) -> Vec<PrivateDocumentEntry> {
        positions
            .map(|i| PrivateDocumentEntry {
                position: u64::from(i),
                block_height: 100 + u64::from(i),
                block_time_ms: 1_000 * u64::from(i),
                data: vec![i; 4096],
            })
            .collect()
    }

    /// Reads, proves and verifies the page of `drive`'s store from `start` under
    /// `platform_version`, checks the read and the proof agree and the proof derives the
    /// drive's root hash, and returns the page.
    fn read_and_prove(
        drive: &Drive,
        contract: &DataContract,
        start: u64,
        platform_version: &PlatformVersion,
    ) -> PrivateDocumentEntries {
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");
        let page = drive
            .fetch_private_document_entries(CONTRACT_ID, secret, start, None, platform_version)
            .expect("expected to read the page");
        let proof = drive
            .prove_private_document_entries(CONTRACT_ID, secret, start, platform_version)
            .expect("expected to prove the page");
        let (proved_root_hash, proved_page) = Drive::verify_private_document_entries(
            &proof,
            CONTRACT_ID,
            secret,
            start,
            platform_version,
        )
        .expect("expected to verify the page");
        let root_hash = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("the root hash");
        assert_eq!(proved_root_hash, root_hash, "proved from {start}");
        assert_eq!(proved_page, page, "proved from {start}");
        page
    }

    /// Pages of a store of 140 entries, 64 to a chunk (two compacted chunks and 12 buffered
    /// entries): each reads from its start to the end of its chunk, in a compacted chunk, in
    /// the second one, in the buffered tail, and past the end; each read and each proof agree,
    /// and a proof derives the drive's root hash.
    #[test]
    fn should_read_and_prove_pages_of_a_private_type_store() {
        let (drive, contract) = drive_with_private_entries(140);
        let platform_version = PlatformVersion::latest();

        for (start, expected) in [
            (0u64, expected_entries(0..64)),
            (10, expected_entries(10..64)),
            (63, expected_entries(63..64)),
            (64, expected_entries(64..128)),
            (100, expected_entries(100..128)),
            (128, expected_entries(128..140)),
            (133, expected_entries(133..140)),
            (140, vec![]),
            (200, vec![]),
        ] {
            let page = read_and_prove(&drive, &contract, start, platform_version);
            assert_eq!(page.entries, expected, "read from {start}");
            assert_eq!(page.total_count, 140);
        }
    }

    /// An empty store, which every store is until entries are appended, reads and proves an
    /// empty page and a count of 0.
    #[test]
    fn should_read_and_prove_an_empty_store() {
        let (drive, contract) = drive_with_private_entries(0);
        let platform_version = PlatformVersion::latest();
        for start in [0, 5, 64] {
            let page = read_and_prove(&drive, &contract, start, platform_version);
            assert_eq!(page, PrivateDocumentEntries::default(), "from {start}");
        }
    }

    /// A store keeps the chunks its contract created it with. Under limits that would size a
    /// new store of the same entries at 128 entries to a chunk, a store of 64 to a chunk still
    /// pages to its own chunk ends, and a proof made under either limits verifies under the
    /// other.
    #[test]
    fn should_page_a_store_by_its_committed_chunk_power_under_changed_limits() {
        let (drive, contract) = drive_with_private_entries(70);
        let platform_version = PlatformVersion::latest();
        let mut changed_limits = platform_version.clone();
        changed_limits
            .system_limits
            .private_document_store_target_chunk_bytes = Some(1024 * 1024);
        // 16 + 4096 = 4112 bytes: 255 entries fit 1 MiB, 2^7 = 128
        assert_eq!(
            private_document_store_chunk_power(4112, &changed_limits).unwrap(),
            7
        );
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");

        for (start, expected) in [
            (0u64, expected_entries(0..64)),
            (32, expected_entries(32..64)),
            (64, expected_entries(64..70)),
        ] {
            let page = read_and_prove(&drive, &contract, start, &changed_limits);
            assert_eq!(page.entries, expected, "read from {start}");
            assert_eq!(page.total_count, 70);

            for (prover, verifier) in [
                (&changed_limits, platform_version),
                (platform_version, &changed_limits),
            ] {
                let proof = drive
                    .prove_private_document_entries(CONTRACT_ID, secret, start, prover)
                    .expect("expected to prove the page");
                let (_, proved_page) = Drive::verify_private_document_entries(
                    &proof,
                    CONTRACT_ID,
                    secret,
                    start,
                    verifier,
                )
                .expect("expected to verify the page");
                assert_eq!(proved_page, page, "proved from {start}");
            }
        }
    }

    /// Only a private type has a store.
    #[test]
    fn should_refuse_a_type_that_is_not_private() {
        let (drive, contract) = drive_with_private_entries(3);
        let platform_version = PlatformVersion::latest();
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");
        let note = contract
            .document_type_for_name("note")
            .expect("the note type");

        let refused = |result: Result<(), Error>| {
            assert!(
                matches!(result, Err(Error::Query(QuerySyntaxError::InvalidParameter(ref message))) if message.contains("not a private document type")),
                "expected the type to be refused, got {result:?}"
            );
        };
        refused(
            drive
                .fetch_private_document_entries(CONTRACT_ID, note, 0, None, platform_version)
                .map(drop),
        );
        refused(
            drive
                .prove_private_document_entries(CONTRACT_ID, note, 0, platform_version)
                .map(drop),
        );
        let proof = drive
            .prove_private_document_entries(CONTRACT_ID, secret, 0, platform_version)
            .expect("expected to prove the first page");
        refused(
            Drive::verify_private_document_entries(&proof, CONTRACT_ID, note, 0, platform_version)
                .map(drop),
        );
    }

    /// Protocol version 14 has no private type, and reads none.
    #[test]
    fn should_read_no_private_entries_before_protocol_version_15() {
        let (drive, contract) = drive_with_private_entries(1);
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");
        let protocol_version_14 = PlatformVersion::get(14).expect("protocol version 14");
        assert!(matches!(
            drive.fetch_private_document_entries(CONTRACT_ID, secret, 0, None, protocol_version_14),
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
        assert!(matches!(
            drive.prove_private_document_entries(CONTRACT_ID, secret, 0, protocol_version_14),
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
        assert!(matches!(
            Drive::verify_private_document_entries(
                &[],
                CONTRACT_ID,
                secret,
                0,
                protocol_version_14
            ),
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }

    /// An entry splits into Drive's header and the client's entry, keeping the stored bytes'
    /// allocation, and only stored bytes of the type's size split.
    #[test]
    fn should_split_a_stored_entry_into_the_header_and_the_entry() {
        let private_document = contract(1, true)
            .document_type_for_name("secret")
            .expect("the secret type")
            .private_document()
            .expect("a private type");
        let stored = encode_stored_private_document_entry(7, 9, &[5; 585]);
        assert_eq!(stored.len(), 16 + 585);
        let stored_allocation = stored.as_ptr();
        let entry = PrivateDocumentEntry::from_stored(3, stored.clone(), private_document)
            .expect("a stored entry of the type's size");
        assert_eq!(
            entry,
            PrivateDocumentEntry {
                position: 3,
                block_height: 7,
                block_time_ms: 9,
                data: vec![5; 585],
            }
        );
        let entry = PrivateDocumentEntry::from_stored(3, stored, private_document)
            .expect("a stored entry of the type's size");
        assert_eq!(entry.data.as_ptr(), stored_allocation);
        assert_eq!(
            PrivateDocumentEntry::from_stored(3, vec![5; 600], private_document),
            None
        );
    }

    /// A page reads at most u16::MAX entries, so every protocol version's store limits keep a
    /// whole chunk (2^max chunk power entries) within one page.
    #[test]
    fn should_keep_a_whole_chunk_within_one_page() {
        for platform_version in PLATFORM_VERSIONS {
            if let Some(max_chunk_power) = platform_version
                .system_limits
                .private_document_store_max_chunk_power
            {
                assert!(
                    1u64 << max_chunk_power <= u64::from(u16::MAX),
                    "protocol version {} allows chunks of 2^{max_chunk_power} entries",
                    platform_version.protocol_version
                );
                assert_eq!(
                    private_document_entries_page_limit(max_chunk_power, 0).unwrap(),
                    1u16 << max_chunk_power
                );
            }
        }
        assert!(private_document_entries_page_limit(16, 0).is_err());
        assert_eq!(
            private_document_entries_page_limit(16, 1).unwrap(),
            u16::MAX
        );
    }

    #[test]
    fn should_keep_a_compacted_chunk_near_the_target() {
        let platform_version = PlatformVersion::latest();
        // 16 + 585 = 601 bytes: 872 entries fit, 2^9 = 512 is the largest power below
        assert_eq!(
            private_document_store_chunk_power(601, platform_version).unwrap(),
            9
        );
        // 16 + 4096 = 4112 bytes: 127 entries fit, 2^6 = 64
        assert_eq!(
            private_document_store_chunk_power(4112, platform_version).unwrap(),
            6
        );
    }

    #[test]
    fn should_clamp_the_chunk_power() {
        let platform_version = PlatformVersion::latest();
        let limits = &platform_version.system_limits;
        let max_chunk_power = limits.private_document_store_max_chunk_power.unwrap();
        let min_chunk_power = limits.private_document_store_min_chunk_power.unwrap();
        assert_eq!(
            private_document_store_chunk_power(17, platform_version).unwrap(),
            max_chunk_power
        );
        assert_eq!(
            private_document_store_chunk_power(u16::MAX as u32, platform_version).unwrap(),
            min_chunk_power
        );
        assert_eq!(
            private_document_store_chunk_power(0, platform_version).unwrap(),
            max_chunk_power
        );
    }

    /// Only a protocol version admitting private types selects a version of the store's
    /// creation: protocol version 14 creates no store.
    #[test]
    fn should_create_a_store_only_at_a_protocol_version_admitting_private_types() {
        let protocol_version_14 = PlatformVersion::get(14).expect("protocol version 14");
        assert!(protocol_version_14
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .parse_private_document
            .is_none());
        let secret_type_path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            &CONTRACT_ID,
            &[1],
            b"secret",
        ];
        let private_document = contract(1, true)
            .document_type_for_name("secret")
            .expect("the secret type")
            .private_document()
            .expect("a private type");
        assert!(matches!(
            insert_empty_private_document_store_operation(
                secret_type_path,
                private_document,
                None,
                protocol_version_14,
            ),
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
        assert!(insert_empty_private_document_store_operation(
            secret_type_path,
            private_document,
            None,
            PlatformVersion::latest(),
        )
        .is_ok());
    }
}
