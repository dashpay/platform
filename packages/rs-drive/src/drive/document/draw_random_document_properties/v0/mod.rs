use crate::drive::document::draw_random_document_properties::RandomDocumentRequest;
use crate::drive::document::paths::contract_documents_primary_key_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{
    integer_value_for_type, DocumentPropertyType, RandomRange, SystemFunction,
};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::btreemap_extensions::BTreeValueMapInsertionPathHelper;
use dpp::util::hash::hash_single;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use std::borrow::Cow;

/// The domain of a draw's seed, so a beacon never seeds anything else the same way.
const DRAW_DOMAIN: &[u8] = b"dash-platform-random-v1";

/// What a seed draws for: a document property (the only kind in this version).
const DOCUMENT_PROPERTY_DRAW: u8 = 0;

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

/// The value the property at `path` of the document `request` names draws from `beacon`:
/// SHA-256 of the domain, the beacon and what the draw is for (a document property, its
/// contract, type, document and path, each variable part prefixed by its length), whose first
/// 16 bytes, big-endian, reduced modulo the range's size, offset the range's minimum. A range
/// holds at most 2^64 values, so the reduction favours no value by more than 2^-64. `None`
/// only for a part longer than a length prefix holds, which registration rules out.
fn draw_v0(
    beacon: &[u8],
    request: &RandomDocumentRequest,
    path: &str,
    range: RandomRange,
) -> Option<i128> {
    let mut preimage = Vec::with_capacity(
        DRAW_DOMAIN.len()
            + 2
            + beacon.len()
            + 1
            + 32
            + 2
            + request.document_type_name.len()
            + 32
            + 2
            + path.len(),
    );
    preimage.extend_from_slice(DRAW_DOMAIN);
    push_with_length(&mut preimage, beacon)?;
    preimage.push(DOCUMENT_PROPERTY_DRAW);
    preimage.extend_from_slice(request.contract_id.as_slice());
    push_with_length(&mut preimage, request.document_type_name.as_bytes())?;
    preimage.extend_from_slice(request.document_id.as_slice());
    push_with_length(&mut preimage, path.as_bytes())?;
    let seed = hash_single(&preimage);
    let mut bits = [0u8; 16];
    bits.copy_from_slice(&seed[..16]);
    range.value_at(u128::from_be_bytes(bits) % range.size())
}

/// Appends `part` after its length as a big-endian `u16`.
fn push_with_length(preimage: &mut Vec<u8>, part: &[u8]) -> Option<()> {
    let length = u16::try_from(part.len()).ok()?;
    preimage.extend_from_slice(&length.to_be_bytes());
    preimage.extend_from_slice(part);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DriveConfig;
    use crate::query::DriveDocumentQuery;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::DocumentV0;
    use dpp::platform_value::{platform_value, Identifier, Value};
    use dpp::prelude::DataContract;
    use std::collections::BTreeMap;

    const OWNER: [u8; 32] = [42; 32];
    const BEACON: [u8; 96] = [7; 96];

    /// A contract with an immutable `card` type, its `rarity` drawn from 1 to 100 and indexed,
    /// and a `plain` type drawing nothing.
    fn contract() -> DataContract {
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(
                Identifier::from(OWNER),
                0,
                platform_value!({
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
                        "required": ["name"],
                        "additionalProperties": false
                    }
                }),
                None,
                None,
            )
            .expect("contract")
            .data_contract_owned()
    }

    fn setup() -> (Drive, DataContract) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = contract();
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

    /// Inserts a `card` holding the placeholder, the way a create transition writes it.
    fn insert_card(drive: &Drive, contract: &DataContract, marker: u8) -> Identifier {
        let id = Identifier::from([marker; 32]);
        let document = Document::V0(DocumentV0 {
            id,
            owner_id: Identifier::from(OWNER),
            properties: BTreeMap::from([
                ("name".to_string(), Value::Text("Ace".to_string())),
                ("rarity".to_string(), Value::U8(1)),
            ]),
            ..Default::default()
        });
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
                PlatformVersion::latest(),
                None,
            )
            .expect("the card inserts");
        id
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
    fn cards_with_rarity(drive: &Drive, contract: &DataContract, rarity: i128) -> Vec<Document> {
        let platform_version = PlatformVersion::latest();
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

    const RARITY: RandomRange = RandomRange {
        minimum: 1,
        maximum: 100,
    };

    #[test]
    fn should_draw_into_an_immutable_document_and_move_its_index_entry() {
        let (drive, contract) = setup();
        let id = insert_card(&drive, &contract, 1);
        let request = request(contract.id(), "card", id);
        let expected = draw_v0(&BEACON, &request, "rarity", RARITY).expect("draws");
        assert!((1..=100).contains(&expected));

        let drawn = drive
            .draw_random_document_properties(
                &BEACON,
                &[request],
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
            )
            .expect("the draw runs");
        assert_eq!(drawn, 1);

        let found = cards_with_rarity(&drive, &contract, expected);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id(), id);
        assert_eq!(
            found[0]
                .get("rarity")
                .and_then(|rarity| rarity.to_integer::<i128>().ok()),
            Some(expected)
        );
        if expected != 1 {
            assert!(cards_with_rarity(&drive, &contract, 1).is_empty());
        }
    }

    #[test]
    fn should_draw_each_document_and_beacon_independently_and_reproducibly() {
        let widest = RandomRange {
            minimum: 0,
            maximum: u64::MAX.into(),
        };
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
            draw_v0(&BEACON, &first, "rarity", RARITY),
            Some(RARITY_VECTOR)
        );
    }

    const DRAW_VECTOR: i128 = 16_905_443_507_565_715_185;
    const RARITY_VECTOR: i128 = 22;

    #[test]
    fn should_skip_a_deleted_document_and_a_type_drawing_nothing() {
        let (drive, contract) = setup();
        let drawn = drive
            .draw_random_document_properties(
                &BEACON,
                &[
                    request(contract.id(), "card", Identifier::from([3; 32])),
                    request(contract.id(), "plain", Identifier::from([4; 32])),
                ],
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
            )
            .expect("the draw runs");
        assert_eq!(drawn, 0);
    }

    #[test]
    fn should_not_draw_before_protocol_version_15() {
        let (drive, _contract) = setup();
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
}
