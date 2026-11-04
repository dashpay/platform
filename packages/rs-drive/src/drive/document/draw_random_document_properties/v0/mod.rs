use crate::drive::document::draw_random_document_properties::RandomDocumentRequest;
use crate::drive::document::expiration::pricing::{
    document_remaining_lifetime_ms, document_ttl_pricing,
};
use crate::drive::document::paths::contract_documents_primary_key_path;
use crate::drive::random_beacon::{draw_from_beacon_v0, push_with_length, DOCUMENT_PROPERTY_DRAW};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::{DocumentOwnedInfo, DocumentRefInfo};
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    indexes_a_random_property, integer_value_for_type, DocumentPropertyType, DocumentTypeRef,
    Index, RandomRange, SystemFunction,
};
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::btreemap_extensions::BTreeValueMapInsertionPathHelper;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use std::borrow::Cow;

impl Drive {
    #[inline(always)]
    pub(super) fn draw_random_document_properties_v0(
        &self,
        beacon: &[u8],
        requests: &[RandomDocumentRequest],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u32, Error> {
        let mut drawn = 0u32;
        for request in requests {
            if self.draw_random_document_v0(
                beacon,
                request,
                block_info,
                transaction,
                platform_version,
            )? {
                drawn = drawn.saturating_add(1);
            }
        }
        Ok(drawn)
    }

    /// Generation 0 of [`Self::prepay_random_document_index_entries`]: the entries of the
    /// document in the indexes of `document_type` holding a `sys.random` property
    /// ([`Self::prepay_drawn_index_entries_v0`]), carrying the storage flags the create writes
    /// them with.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn prepay_random_document_index_entries_v0(
        &self,
        document: &Document,
        owner_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        // The flags the create writes the document's elements with
        let storage_flags =
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id.to_buffer()));
        self.prepay_drawn_index_entries_v0(
            document,
            owner_id,
            Some(storage_flags),
            contract,
            document_type,
            |index| indexes_a_random_property(document_type, index),
            block_info,
            transaction,
            platform_version,
        )
    }

    /// What Drive's fee estimation gives, without reading state, for inserting the entries
    /// `document`, of `document_type` in `contract`, written by `owner_id` with
    /// `storage_flags`, takes in the indexes of its type that `drawn_into` selects: the indexes
    /// holding a value a block's random beacon draws once the block is accepted, whose rewrite
    /// moves the document's entries and may create the branches they move into, unbilled. The
    /// selected indexes stand alone in a copy of the type (`with_only_indexes`), the entries
    /// are gathered as the insert gathers a document's index entries in a dry run, with the
    /// layers above the type estimated as for any document insert (the average layer
    /// information, as if the branches had to be created), and priced as the dry run's
    /// operations are. As the insert, the dry run first completes the document with the values
    /// of the derived index properties (`with_derived_index_values`), which a create's document
    /// does not carry yet, estimated: nothing is read, and the reads are priced by their worst
    /// case. Zero when `drawn_into` selects no index of the type.
    ///
    /// Shared by the prepayments of generation 0 of both draws: a `sys.random` document
    /// property's ([`Self::prepay_random_document_index_entries`]) and the amount a random
    /// token transfer or mint writes into its history document
    /// (`Drive::prepay_random_token_history_entries`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepay_drawn_index_entries_v0(
        &self,
        document: &Document,
        owner_id: Identifier,
        storage_flags: Option<StorageFlags>,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        drawn_into: impl Fn(&Index) -> bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        if !document_type.indexes().values().any(&drawn_into) {
            return Ok(FeeResult::default());
        }
        let indexes_drawn_into = document_type.with_only_indexes(&drawn_into, platform_version)?;
        let document_and_contract_info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, storage_flags.map(Cow::Owned))),
                owner_id: Some(owner_id.to_buffer()),
            },
            contract,
            document_type: indexes_drawn_into.as_ref(),
        }
        .without_storage_flags_if_expiring();

        let mut estimated_layers = EstimatedLayers::new();
        Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
            contract,
            &mut estimated_layers,
            &platform_version.drive,
        )?;
        let mut estimated_costs_only_with_layer_info = Some(estimated_layers);
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        let document_and_contract_info = self.with_derived_index_values(
            document_and_contract_info,
            true,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;
        self.add_indices_for_top_index_level_for_contract_operations(
            &document_and_contract_info,
            &mut None,
            &mut estimated_costs_only_with_layer_info,
            block_info.time_ms,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;
        // A type whose documents expire prices the entries by the lifetime the document has
        // left, as its insert prices its own (`add_document_ttl_operations`): the entries the
        // draw moves live as long as the document
        if let Some(ttl_seconds) = document_type.documents_ttl_seconds() {
            let pricing = document_ttl_pricing(
                document_remaining_lifetime_ms(
                    document.created_at(),
                    ttl_seconds,
                    block_info.time_ms,
                )?,
                self.config.epoch_time_length_s,
                self.config.epochs_per_era,
                &platform_version.fee_version,
            )?;
            batch_operations = batch_operations
                .into_iter()
                .map(|operation| operation.retag_document_ttl(pricing))
                .collect();
        }
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// Draws the `sys.random` properties of the document `request` names and rewrites it;
    /// `false` when there is nothing to draw: its type declares no such property, or a later
    /// transition of the block deleted it.
    fn draw_random_document_v0(
        &self,
        beacon: &[u8],
        request: &RandomDocumentRequest,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        // Unbilled, like the rest of the draw: the creation that recorded the request paid for
        // its write.
        let (_, Some(contract)) = self.get_contract_with_fetch_info_and_fee(
            request.contract_id.to_buffer(),
            None,
            true,
            transaction,
            platform_version,
        )?
        else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "the contract of a document the block created exists",
            )));
        };
        let document_type = contract
            .contract
            .document_type_for_name(&request.document_type_name)
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "the type of a document the block created exists",
                ))
            })?;
        let draws: Vec<(String, RandomRange, DocumentPropertyType)> = document_type
            .generated_from_fields()
            .iter()
            .filter_map(|(path, generated_from)| {
                let (SystemFunction::Random(_), Some(range)) =
                    (generated_from.function, generated_from.range)
                else {
                    return None;
                };
                let property_type = document_type
                    .flattened_properties()
                    .get(path)?
                    .property_type
                    .clone();
                Some((path.clone(), range, property_type))
            })
            .collect();
        if draws.is_empty() {
            return Ok(false);
        }

        let primary_key_path = contract_documents_primary_key_path(
            contract.contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );
        let element = self.grove_get_raw_optional(
            (&primary_key_path).into(),
            request.document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        let (serialized, element_flags) = match element {
            Some(Element::Item(serialized, flags))
            | Some(Element::ItemWithSumItem(serialized, _, flags)) => (serialized, flags),
            // A later transition of the block deleted it
            None => return Ok(false),
            Some(_) => {
                return Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                    "a document the block created is stored as an item",
                )))
            }
        };
        let mut document = Document::from_bytes(&serialized, document_type, platform_version)?;
        let storage_flags = StorageFlags::map_some_element_flags_ref(&element_flags)?;

        for (path, range, property_type) in &draws {
            let drawn = draw_v0(beacon, request, path, *range).ok_or(Error::Drive(
                DriveError::CorruptedCodeExecution("a range holds every offset below its size"),
            ))?;
            let value = integer_value_for_type(property_type, drawn).ok_or(Error::Drive(
                DriveError::CorruptedCodeExecution(
                    "registration admits sys.random only on an integer type holding its range",
                ),
            ))?;
            // The document holds the placeholder at the path, so the objects on the way are
            // maps
            document
                .properties_mut()
                .insert_at_path(path, value)
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "a document holding a placeholder holds the objects on its path",
                    ))
                })?;
        }

        let owner_id = document.owner_id().to_buffer();
        let operations = self.update_stored_document_operations_v1(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentOwnedInfo((document, storage_flags.map(Cow::Owned))),
                    owner_id: Some(owner_id),
                },
                contract: &contract.contract,
                document_type,
            },
            block_info,
            &mut None,
            &mut None,
            transaction,
            platform_version,
        )?;
        // No fee: the creation paid for the placeholder's write, which takes the same storage
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            operations,
            &mut vec![],
            &platform_version.drive,
        )?;
        Ok(true)
    }
}

/// The value the property at `path` of the document `request` names draws from `beacon`
/// (`draw_from_beacon_v0`), under the request key of a document property: what the draw is
/// for, the contract, the type, the document and the path, each variable part prefixed by its
/// length. `None` only for a part longer than a length prefix holds, which registration rules
/// out.
fn draw_v0(
    beacon: &[u8],
    request: &RandomDocumentRequest,
    path: &str,
    range: RandomRange,
) -> Option<i128> {
    let mut request_key =
        Vec::with_capacity(1 + 32 + 2 + request.document_type_name.len() + 32 + 2 + path.len());
    request_key.push(DOCUMENT_PROPERTY_DRAW);
    request_key.extend_from_slice(request.contract_id.as_slice());
    push_with_length(&mut request_key, request.document_type_name.as_bytes())?;
    request_key.extend_from_slice(request.document_id.as_slice());
    push_with_length(&mut request_key, path.as_bytes())?;
    draw_from_beacon_v0(beacon, &request_key, range)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DriveConfig;
    use crate::query::DriveDocumentQuery;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::document::{DocumentV0, DocumentV0Setters};
    use dpp::platform_value::{platform_value, Value};
    use std::collections::BTreeMap;

    const OWNER: [u8; 32] = [42; 32];
    const BEACON: [u8; 96] = [7; 96];

    /// The latest platform version with `sys.random.uniform` admitted, which no protocol
    /// version does until block execution draws the values.
    fn drawing_platform_version() -> PlatformVersion {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .random_generated_from = Some(0);
        platform_version
    }

    /// A contract with an immutable `card` type, its `rarity` drawn from 1 to 100 and indexed,
    /// and a `plain` type drawing nothing, registered at `platform_version`.
    fn contract(platform_version: &PlatformVersion) -> DataContract {
        DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::from([9; 32]),
                "ownerId": Identifier::from(OWNER),
                "version": 1u32,
                "documentSchemas": {
                    "card": {
                        "type": "object",
                        "documentsMutable": false,
                        "properties": {
                            "name": { "type": "string", "maxLength": 32, "position": 0 },
                            "rarity": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 100,
                                "generatedFrom": { "function": "sys.random.uniform" },
                                "position": 1
                            }
                        },
                        "indices": [{ "name": "byRarity", "properties": [{ "rarity": "asc" }] }],
                        "required": ["name", "rarity"],
                        "additionalProperties": false
                    },
                    "plain": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "maxLength": 32, "position": 0 }
                        },
                        "indices": [{ "name": "byName", "properties": [{ "name": "asc" }] }],
                        "required": ["name"],
                        "additionalProperties": false
                    },
                    "fleetingCard": {
                        "type": "object",
                        "documentsMutable": false,
                        "ttl": 3600,
                        "properties": {
                            "name": { "type": "string", "maxLength": 32, "position": 0 },
                            "rarity": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 100,
                                "generatedFrom": { "function": "sys.random.uniform" },
                                "position": 1
                            }
                        },
                        "indices": [{ "name": "byRarity", "properties": [{ "rarity": "asc" }] }],
                        "required": ["name", "rarity", "$createdAt"],
                        "additionalProperties": false
                    }
                }
            }),
            true,
            platform_version,
        )
        .expect("contract")
    }

    fn setup(platform_version: &PlatformVersion) -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = contract(platform_version);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("contract applies");
        (drive, contract)
    }

    /// A `card` holding the placeholder, the way a create transition writes it.
    fn card(marker: u8) -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from([marker; 32]),
            owner_id: Identifier::from(OWNER),
            properties: BTreeMap::from([
                ("name".to_string(), Value::Text("Ace".to_string())),
                ("rarity".to_string(), Value::U8(1)),
            ]),
            ..Default::default()
        })
    }

    /// Inserts a `card` holding the placeholder, the way a create transition writes it.
    fn insert_card(
        drive: &Drive,
        contract: &DataContract,
        marker: u8,
        platform_version: &PlatformVersion,
    ) -> Identifier {
        let document = card(marker);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(OWNER)))),
                        )),
                        owner_id: Some(OWNER),
                    },
                    contract,
                    document_type: contract.document_type_for_name("card").expect("card"),
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("the card inserts");
        document.id()
    }

    fn request(
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
    ) -> RandomDocumentRequest {
        RandomDocumentRequest {
            contract_id,
            document_type_name: document_type_name.to_string(),
            document_id,
        }
    }

    /// The cards whose rarity is `rarity`, through the `byRarity` index.
    fn cards_with_rarity(
        drive: &Drive,
        contract: &DataContract,
        rarity: i128,
        platform_version: &PlatformVersion,
    ) -> Vec<Document> {
        let query = DriveDocumentQuery::from_sql_expr(
            &format!("select * from card where rarity = {rarity}"),
            contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("the query builds");
        let (results, _, _) = query
            .execute_raw_results_no_proof(drive, None, None, platform_version)
            .expect("the query runs");
        let card = contract.document_type_for_name("card").expect("card");
        results
            .iter()
            .map(|bytes| Document::from_bytes(bytes, card, platform_version).expect("decodes"))
            .collect()
    }

    fn rarity_range() -> RandomRange {
        RandomRange::new(1, 100).expect("a range")
    }

    #[test]
    fn should_draw_into_an_immutable_document_and_move_its_index_entry() {
        let platform_version = drawing_platform_version();
        let (drive, contract) = setup(&platform_version);
        let id = insert_card(&drive, &contract, 1, &platform_version);
        let request = request(contract.id(), "card", id);
        let expected = draw_v0(&BEACON, &request, "rarity", rarity_range()).expect("draws");
        assert!((1..=100).contains(&expected));

        let drawn = drive
            .draw_random_document_properties(
                &BEACON,
                &[request],
                &BlockInfo::default(),
                None,
                &platform_version,
            )
            .expect("the draw runs");
        assert_eq!(drawn, 1);

        let found = cards_with_rarity(&drive, &contract, expected, &platform_version);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id(), id);
        assert_eq!(
            found[0]
                .get("rarity")
                .and_then(|rarity| rarity.to_integer::<i128>().ok()),
            Some(expected)
        );
        if expected != 1 {
            assert!(cards_with_rarity(&drive, &contract, 1, &platform_version).is_empty());
        }
    }

    #[test]
    fn should_draw_each_document_and_beacon_independently_and_reproducibly() {
        let widest = RandomRange::new(0, u64::MAX.into()).expect("2^64 values");
        let contract_id = Identifier::from([9; 32]);
        let first = request(contract_id, "card", Identifier::from([1; 32]));
        let second = request(contract_id, "card", Identifier::from([2; 32]));
        let draw = |beacon: &[u8], request: &RandomDocumentRequest, path: &str| {
            draw_v0(beacon, request, path, widest).expect("draws")
        };
        assert_eq!(
            draw(&BEACON, &first, "rarity"),
            draw(&BEACON, &first, "rarity")
        );
        assert_ne!(
            draw(&BEACON, &first, "rarity"),
            draw(&BEACON, &second, "rarity")
        );
        assert_ne!(
            draw(&BEACON, &first, "rarity"),
            draw(&[8; 96], &first, "rarity")
        );
        assert_ne!(
            draw(&BEACON, &first, "rarity"),
            draw(&BEACON, &first, "power")
        );
        // The draw is consensus: every node must compute these values forever
        assert_eq!(draw(&BEACON, &first, "rarity"), DRAW_VECTOR);
        assert_eq!(
            draw_v0(&BEACON, &first, "rarity", rarity_range()),
            Some(RARITY_VECTOR)
        );
    }

    const DRAW_VECTOR: i128 = 16_905_443_507_565_715_185;
    const RARITY_VECTOR: i128 = 22;

    #[test]
    fn should_skip_a_deleted_document_and_a_type_drawing_nothing() {
        let platform_version = drawing_platform_version();
        let (drive, contract) = setup(&platform_version);
        let drawn = drive
            .draw_random_document_properties(
                &BEACON,
                &[
                    request(contract.id(), "card", Identifier::from([3; 32])),
                    request(contract.id(), "plain", Identifier::from([4; 32])),
                ],
                &BlockInfo::default(),
                None,
                &platform_version,
            )
            .expect("the draw runs");
        assert_eq!(drawn, 0);
    }

    #[test]
    fn should_not_draw_before_protocol_version_15() {
        let (drive, _contract) = setup(&drawing_platform_version());
        let result = drive.draw_random_document_properties(
            &BEACON,
            &[],
            &BlockInfo::default(),
            None,
            PlatformVersion::get(14).expect("protocol version 14"),
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }

    #[test]
    fn should_prepay_the_draw_once_with_the_insert_of_the_document() {
        // The insert of a card, and of one that expires in an hour, is charged the prepayment
        // on top of what it costs at a version whose draw method version is unset, which
        // prepays nothing
        let platform_version = drawing_platform_version();
        let mut without_draw = platform_version.clone();
        without_draw
            .drive
            .methods
            .document
            .draw_random_document_properties = None;
        let mut fleeting_card = card(2);
        fleeting_card.set_created_at(Some(0));
        for (document_type_name, document) in [("card", card(1)), ("fleetingCard", fleeting_card)] {
            let insert_fee = |platform_version: &PlatformVersion| {
                let (drive, contract) = setup(platform_version);
                drive
                    .add_document_for_contract(
                        DocumentAndContractInfo {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentRefInfo((
                                    &document,
                                    Some(Cow::Owned(StorageFlags::new_single_epoch(
                                        0,
                                        Some(OWNER),
                                    ))),
                                )),
                                owner_id: Some(OWNER),
                            },
                            contract: &contract,
                            document_type: contract
                                .document_type_for_name(document_type_name)
                                .expect("the type"),
                        },
                        false,
                        BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                        None,
                    )
                    .expect("the document inserts")
            };
            let (drive, contract) = setup(&platform_version);
            let prepayment = drive
                .prepay_random_document_index_entries(
                    &document,
                    Identifier::from(OWNER),
                    &contract,
                    contract
                        .document_type_for_name(document_type_name)
                        .expect("the type"),
                    &BlockInfo::default(),
                    None,
                    &platform_version,
                )
                .expect("the prepayment is priced");
            assert_ne!(prepayment, FeeResult::default(), "{document_type_name}");

            let with = insert_fee(&platform_version);
            let without = insert_fee(&without_draw);
            assert_eq!(
                with.storage_fee,
                without.storage_fee + prepayment.storage_fee,
                "{document_type_name}"
            );
            assert_eq!(
                with.processing_fee,
                without.processing_fee + prepayment.processing_fee,
                "{document_type_name}"
            );
        }
    }

    #[test]
    fn should_prepay_the_draw_of_an_expiring_document_for_the_lifetime_it_has_left() {
        // A card that expires in an hour prepays its `byRarity` entry for that hour, where a
        // card that never expires pays for it for good
        let platform_version = drawing_platform_version();
        let (drive, contract) = setup(&platform_version);
        let prepay = |document_type_name: &str, document: &Document| {
            drive
                .prepay_random_document_index_entries(
                    document,
                    Identifier::from(OWNER),
                    &contract,
                    contract
                        .document_type_for_name(document_type_name)
                        .expect("the type"),
                    &BlockInfo::default(),
                    None,
                    &platform_version,
                )
                .expect("the prepayment is priced")
        };
        let lasting = prepay("card", &card(1));
        let mut fleeting_card = card(2);
        fleeting_card.set_created_at(Some(0));
        let fleeting = prepay("fleetingCard", &fleeting_card);

        assert!(lasting.lifetime_storage_fees.is_empty());
        assert!(!fleeting.lifetime_storage_fees.is_empty());
        assert!(
            fleeting.storage_fee < lasting.storage_fee,
            "an hour of storage costs less than storage for good: {} against {}",
            fleeting.storage_fee,
            lasting.storage_fee
        );
    }

    #[test]
    fn should_prepay_the_entries_of_the_indexes_holding_a_random_property_only() {
        let platform_version = drawing_platform_version();
        let (drive, contract) = setup(&platform_version);
        let prepay = |document: &Document, document_type_name: &str| {
            drive
                .prepay_random_document_index_entries(
                    document,
                    Identifier::from(OWNER),
                    &contract,
                    contract
                        .document_type_for_name(document_type_name)
                        .expect("the type"),
                    &BlockInfo::default(),
                    None,
                    &platform_version,
                )
                .expect("the prepayment is priced")
        };
        let card_prepayment = prepay(&card(5), "card");
        assert!(card_prepayment.storage_fee > 0);
        assert!(card_prepayment.processing_fee > 0);

        // Indexed, but by nothing drawn
        let plain = Document::V0(DocumentV0 {
            id: Identifier::from([6; 32]),
            owner_id: Identifier::from(OWNER),
            properties: BTreeMap::from([("name".to_string(), Value::Text("Ace".to_string()))]),
            ..Default::default()
        });
        assert_eq!(prepay(&plain, "plain"), FeeResult::default());
    }
}
