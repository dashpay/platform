#[cfg(feature = "server")]
use crate::drive::document::paths::contract_document_type_path_vec;
#[cfg(feature = "server")]
use crate::drive::document::private_document_store::{
    encode_stored_private_document_entry, private_document_stored_entry_size,
    PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE, PRIVATE_DOCUMENT_STORE_KEY,
};
#[cfg(feature = "server")]
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::util::estimated_layers::EstimatedLayers;
#[cfg(feature = "server")]
use crate::util::storage_flags::StorageFlags;
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
#[cfg(feature = "server")]
use dpp::data_contract::accessors::v0::DataContractV0Getters;
#[cfg(feature = "server")]
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::PrivateDocumentConfig;
#[cfg(feature = "server")]
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
#[cfg(feature = "server")]
use grovedb::batch::KeyInfoPath;
#[cfg(feature = "server")]
use grovedb::batch::QualifiedGroveDbOp;
#[cfg(feature = "server")]
use grovedb::Element;
#[cfg(feature = "server")]
use grovedb::EstimatedLayerCount::EstimatedLevel;
#[cfg(feature = "server")]
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
#[cfg(feature = "server")]
use grovedb::EstimatedSumTrees::NoSumTrees;
#[cfg(feature = "server")]
use grovedb::{EstimatedLayerInformation, TreeType};

/// The chunk power of a store whose stored entries take `stored_entry_size` bytes: the largest
/// that keeps a compacted chunk near the target of `platform_version`'s system limits,
/// `clamp(floor(log2(target / stored_entry_size)), min, max)`. A platform rule, not a contract
/// choice, committed into the store when the store is created: a store keeps the rule of the
/// protocol version that created it, and new numbers are a new limits table.
pub(super) fn private_document_store_chunk_power_v0(
    stored_entry_size: u32,
    platform_version: &PlatformVersion,
) -> Result<u8, Error> {
    let limits = &platform_version.system_limits;
    let (Some(target_chunk_bytes), Some(min_chunk_power), Some(max_chunk_power)) = (
        limits.private_document_store_target_chunk_bytes,
        limits.private_document_store_min_chunk_power,
        limits.private_document_store_max_chunk_power,
    ) else {
        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
            "a private document store is sized only at a protocol version setting the private \
             document store limits",
        )));
    };
    let entries_per_chunk = target_chunk_bytes / stored_entry_size.max(1);
    // At most 31 for a u32
    let power = u8::try_from(entries_per_chunk.checked_ilog2().unwrap_or(0)).unwrap_or(u8::MAX);
    Ok(power.clamp(min_chunk_power, max_chunk_power))
}

/// The store is a single element, inserted directly: grovedb's empty-tree helpers take a tree
/// type, which cannot carry the store's entry size. It carries the contract's storage flags,
/// like the type's other trees.
#[cfg(feature = "server")]
pub(super) fn insert_empty_private_document_store_operation_v0(
    type_path: [&[u8]; 4],
    private_document: PrivateDocumentConfig,
    storage_flags: Option<&StorageFlags>,
    platform_version: &PlatformVersion,
) -> Result<LowLevelDriveOperation, Error> {
    let stored_entry_size = private_document_stored_entry_size(private_document)?;
    let store = Element::empty_private_document_store_with_flags(
        stored_entry_size,
        private_document_store_chunk_power_v0(stored_entry_size, platform_version)?,
        storage_flags.map(StorageFlags::to_element_flags),
    )?;
    Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
        type_path.iter().map(|segment| segment.to_vec()).collect(),
        vec![PRIVATE_DOCUMENT_STORE_KEY],
        store,
    ))
}

/// One GroveDB append per entry, into the store at the type's key `[0]`, each entry after Drive's
/// header. The entries land at consecutive positions in this order. An estimated batch gets the
/// layers down to the contract's document types (`add_estimation_costs_for_levels_up_to_contract_document_type_excluded`),
/// the type's tree, which holds the store alone, and the store's own layer at its committed
/// chunk power, which GroveDB's estimator requires.
#[cfg(feature = "server")]
pub(super) fn append_private_document_entries_operations_v0(
    contract: &DataContract,
    document_type_name: &str,
    entries: &[Vec<u8>],
    chunk_power: u8,
    block_info: &BlockInfo,
    estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
    platform_version: &PlatformVersion,
) -> Result<Vec<LowLevelDriveOperation>, Error> {
    let contract_id = contract.id().to_buffer();
    if let Some(estimated_costs) = estimated_costs_only_with_layer_info {
        Drive::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
            contract,
            estimated_costs,
            &platform_version.drive,
        )?;
        // The store carries the contract's storage flags, as the type's tree does.
        let storage_flags = contract
            .config()
            .can_be_deleted()
            .then(|| StorageFlags::approximate_size(true, None));
        estimated_costs.insert(
            KeyInfoPath::from_known_owned_path(contract_document_type_path_vec(
                &contract_id,
                document_type_name,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                // A private type's tree holds its store and nothing else.
                estimated_layer_count: EstimatedLevel(0, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, storage_flags),
            },
        );
        let stored_entry_size = entries
            .first()
            .map(|entry| entry.len() as u32)
            .unwrap_or(0)
            .saturating_add(PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE);
        let mut store_path = contract_document_type_path_vec(&contract_id, document_type_name);
        store_path.push(vec![PRIVATE_DOCUMENT_STORE_KEY]);
        estimated_costs.insert(
            KeyInfoPath::from_known_owned_path(store_path),
            EstimatedLayerInformation {
                tree_type: TreeType::PrivateDocumentStore(chunk_power),
                estimated_layer_count: EstimatedLevel(0, false),
                estimated_layer_sizes: AllItems(8, stored_entry_size, None),
            },
        );
    }
    let mut store_path = contract_document_type_path_vec(&contract_id, document_type_name);
    store_path.push(vec![PRIVATE_DOCUMENT_STORE_KEY]);
    Ok(entries
        .iter()
        .map(|entry| {
            LowLevelDriveOperation::GroveOperation(
                QualifiedGroveDbOp::private_document_store_insert_op(
                    store_path.clone(),
                    encode_stored_private_document_entry(
                        block_info.height,
                        block_info.time_ms,
                        entry,
                    ),
                ),
            )
        })
        .collect())
}
