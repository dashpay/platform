use std::collections::BTreeMap;

use platform_value::Identifier;
use platform_value::Value;

use crate::data_contract::{DefinitionName, DocumentName};

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;

use crate::metadata::Metadata;

/// `DataContractV0` represents a data contract in a decentralized platform.
///
/// It contains information about the contract, such as its protocol version, unique identifier,
/// schema, version, and owner identifier. The struct also includes details about the document
/// types, metadata, configuration, and document schemas associated with the contract.
///
/// Additionally, `DataContractV0` holds definitions for JSON schemas, entropy, and binary properties
/// of the documents.
#[derive(Debug, Clone, PartialEq)]
pub struct DataContractV0 {
    /// A unique identifier for the data contract.
    /// This field must always present in all versions.
    pub(crate) id: Identifier,

    /// The version of this data contract.
    pub(crate) version: u32,

    /// The identifier of the contract owner.
    pub(crate) owner_id: Identifier,

    /// A mapping of document names to their corresponding document types.
    pub document_types: BTreeMap<DocumentName, DocumentType>,

    // TODO: Move metadata from here
    /// Optional metadata associated with the contract.
    pub(crate) metadata: Option<Metadata>,

    /// Internal configuration for the contract.
    pub(crate) config: DataContractConfig,

    /// Shared subschemas to reuse across documents (see $defs)
    pub(crate) schema_defs: Option<BTreeMap<DefinitionName, Value>>,
}

#[cfg(test)]
mod test {

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }
    //
    // #[test]
    // #[cfg(feature = "cbor")]
    // fn conversion_to_cbor_buffer_from_cbor_buffer() {
    //     init();
    //     let data_contract = data_contract_fixture(None).data_contract;
    //
    //     let data_contract_bytes = data_contract
    //         .to_cbor_buffer()
    //         .expect("data contract should be converted into the bytes");
    //     let data_contract_restored = DataContractV0::from_cbor_buffer(data_contract_bytes)
    //         .expect("data contract should be created from bytes");
    //
    //     assert_eq!(
    //         data_contract.data_contract_protocol_version,
    //         data_contract_restored.data_contract_protocol_version
    //     );
    //     assert_eq!(data_contract.schema, data_contract_restored.schema);
    //     assert_eq!(data_contract.version, data_contract_restored.version);
    //     assert_eq!(data_contract.id(), data_contract_restored.id);
    //     assert_eq!(data_contract.owner_id(), data_contract_restored.owner_id);
    //     assert_eq!(
    //         data_contract.binary_properties,
    //         data_contract_restored.binary_properties
    //     );
    //     assert_eq!(data_contract.documents, data_contract_restored.documents);
    //     assert_eq!(
    //         data_contract.document_types,
    //         data_contract_restored.document_types
    //     );
    // }
    //
    // #[test]
    // #[cfg(feature = "cbor")]
    // fn conversion_to_cbor_buffer_from_cbor_buffer_high_version() {
    //     init();
    //     let mut data_contract = get_data_contract_fixture(None).data_contract;
    //     data_contract.data_contract_protocol_version = 10000;
    //
    //     let data_contract_bytes = data_contract
    //         .to_cbor_buffer()
    //         .expect("data contract should be converted into the bytes");
    //
    //     let data_contract_restored = DataContractV0::from_cbor_buffer(data_contract_bytes)
    //         .expect("data contract should be created from bytes");
    //
    //     assert_eq!(
    //         data_contract.data_contract_protocol_version,
    //         data_contract_restored.data_contract_protocol_version
    //     );
    //     assert_eq!(data_contract.schema, data_contract_restored.schema);
    //     assert_eq!(data_contract.version, data_contract_restored.version);
    //     assert_eq!(data_contract.id(), data_contract_restored.id);
    //     assert_eq!(data_contract.owner_id(), data_contract_restored.owner_id);
    //     assert_eq!(
    //         data_contract.binary_properties,
    //         data_contract_restored.binary_properties
    //     );
    //     assert_eq!(data_contract.documents, data_contract_restored.documents);
    //     assert_eq!(
    //         data_contract.document_types,
    //         data_contract_restored.document_types
    //     );
    // }
    //
    // #[test]
    // fn conversion_to_cbor_buffer_from_cbor_buffer_too_high_version() {
    //     init();
    //     let data_contract = get_data_contract_fixture(None).data_contract;
    //
    //     let data_contract_bytes = data_contract
    //         .to_cbor_buffer()
    //         .expect("data contract should be converted into the bytes");
    //
    //     let mut high_protocol_version_bytes = u64::MAX.encode_var_vec();
    //
    //     let (_, offset) = u32::decode_var(&data_contract_bytes)
    //         .ok_or(ProtocolError::DecodingError(
    //             "contract cbor could not decode protocol version".to_string(),
    //         ))
    //         .expect("expected to decode protocol version");
    //     let (_, contract_cbor_bytes) = data_contract_bytes.split_at(offset);
    //
    //     high_protocol_version_bytes.extend_from_slice(contract_cbor_bytes);
    //
    //     let data_contract_restored = DataContractV0::from_cbor_buffer(&high_protocol_version_bytes)
    //         .expect("data contract should be created from bytes");
    //
    //     assert_eq!(
    //         u32::MAX,
    //         data_contract_restored.data_contract_protocol_version
    //     );
    //     assert_eq!(data_contract.schema, data_contract_restored.schema);
    //     assert_eq!(data_contract.version, data_contract_restored.version);
    //     assert_eq!(data_contract.id(), data_contract_restored.id);
    //     assert_eq!(data_contract.owner_id(), data_contract_restored.owner_id);
    //     assert_eq!(
    //         data_contract.binary_properties,
    //         data_contract_restored.binary_properties
    //     );
    //     assert_eq!(data_contract.documents, data_contract_restored.documents);
    //     assert_eq!(
    //         data_contract.document_types,
    //         data_contract_restored.document_types
    //     );
    // }
    //
    // #[test]
    // fn conversion_from_json() -> Result<()> {
    //     init();
    //
    //     let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
    //     let contract = DataContractV0::try_from(string_contract.as_str())?;
    //     assert_eq!(contract.data_contract_protocol_version, 0);
    //     assert_eq!(
    //         contract.schema,
    //         "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
    //     );
    //     assert_eq!(contract.version, 5);
    //     assert_eq!(
    //         contract.id.to_string(Encoding::Base58),
    //         "AoDzJxWSb1gUi2dSmvFeUFpSsjZQRJaqCpn7vCLkwwJj"
    //     );
    //     assert_eq!(
    //         contract.documents["note"]["properties"]["message"]["type"],
    //         "string"
    //     );
    //     assert!(contract.is_document_defined("note"));
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn conversion_to_json() -> Result<()> {
    //     init();
    //
    //     let mut string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
    //     string_contract.retain(|c| !c.is_whitespace());
    //
    //     let contract = DataContractV0::try_from(string_contract.as_str())?;
    //     let serialized_contract = serde_json::to_string(&contract.to_json()?)?;
    //
    //     // they will be out of order so won't be exactly the same
    //     assert_eq!(serialized_contract, string_contract);
    //     Ok(())
    // }
    //
    // #[test]
    // fn conversion_to_object() -> Result<()> {
    //     let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
    //     let data_contract: DataContractV0 = serde_json::from_str(&string_contract)?;
    //
    //     let raw_data_contract = data_contract.to_json_object()?;
    //     for path in DATA_CONTRACT_IDENTIFIER_FIELDS_V0 {
    //         assert!(raw_data_contract
    //             .get(path)
    //             .expect("the path should exist")
    //             .is_array())
    //     }
    //     Ok(())
    // }
    //
    // #[test]
    // fn conversion_from_object() -> Result<()> {
    //     init();
    //
    //     let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
    //     let raw_contract: JsonValue = serde_json::from_str(&string_contract)?;
    //
    //     for path in DATA_CONTRACT_IDENTIFIER_FIELDS_V0 {
    //         raw_contract.get(path).expect("the path should exist");
    //     }
    //
    //     let data_contract_from_raw = DataContractV0::try_from(raw_contract)?;
    //     assert_eq!(data_contract_from_raw.data_contract_protocol_version, 0);
    //     assert_eq!(
    //         data_contract_from_raw.schema,
    //         "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
    //     );
    //     assert_eq!(data_contract_from_raw.version, 5);
    //     assert_eq!(
    //         data_contract_from_raw.id.to_string(Encoding::Base58),
    //         "AoDzJxWSb1gUi2dSmvFeUFpSsjZQRJaqCpn7vCLkwwJj"
    //     );
    //     assert_eq!(
    //         data_contract_from_raw.documents["note"]["properties"]["message"]["type"],
    //         "string"
    //     );
    //
    //     Ok(())
    // }
    //
    // fn get_data_contract_cbor_bytes() -> Vec<u8> {
    //     let data_contract_cbor_hex = "01a56324696458208efef7338c0d34b2e408411b9473d724cbf9b675ca72b3126f7f8e7deb42ae516724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820962088aa3812bb3386d0c9130edbde51e4be17bb2d10031d4147c8597facee256776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e677468183f67636f756e747279a2647479706566737472696e67696d61784c656e677468183f686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4";
    //     hex::decode(data_contract_cbor_hex).unwrap()
    // }
    //
    // #[test]
    // fn deserialize_dpp_cbor() {
    //     let data_contract_cbor = get_data_contract_cbor_bytes();
    //
    //     let data_contract = DataContractV0::from_cbor_buffer(data_contract_cbor).unwrap();
    //
    //     assert_eq!(data_contract.version, 1);
    //     assert_eq!(data_contract.data_contract_protocol_version, 1);
    //     assert_eq!(
    //         data_contract.schema,
    //         "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
    //     );
    //     assert_eq!(
    //         data_contract.owner_id(),
    //         Identifier::new([
    //             150, 32, 136, 170, 56, 18, 187, 51, 134, 208, 201, 19, 14, 219, 222, 81, 228, 190,
    //             23, 187, 45, 16, 3, 29, 65, 71, 200, 89, 127, 172, 238, 37
    //         ])
    //     );
    //     assert_eq!(
    //         data_contract.id(),
    //         Identifier::new([
    //             142, 254, 247, 51, 140, 13, 52, 178, 228, 8, 65, 27, 148, 115, 215, 36, 203, 249,
    //             182, 117, 202, 114, 179, 18, 111, 127, 142, 125, 235, 66, 174, 81
    //         ])
    //     );
    // }
    //
    // #[test]
    // fn serialize_deterministically_serialize_to_cbor() {
    //     let data_contract_cbor = get_data_contract_cbor_bytes();
    //
    //     let data_contract = DataContractV0::from_cbor_buffer(&data_contract_cbor).unwrap();
    //
    //     let serialized = data_contract.to_cbor_buffer().unwrap();
    //
    //     assert_eq!(hex::encode(data_contract_cbor), hex::encode(serialized));
    // }
    // #[test]
    // fn serialize_deterministically_serialize_to_bincode() {
    //     let data_contract_cbor = get_data_contract_cbor_bytes();
    //
    //     let data_contract = DataContract::from_cbor_buffer(&data_contract_cbor).unwrap();
    //
    //     let serialized = data_contract.to_cbor_buffer().unwrap();
    //
    //     assert_eq!(hex::encode(data_contract_cbor), hex::encode(serialized));
    // }
}
