use super::*;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::OwnedDocumentInfo;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::DocumentV0Getters;
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_json_value;
use serde_json::json;

#[test]
fn should_estimate_composite_terminal_width_in_preallocated_member_layers() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let mut schema = json_document_to_json_value(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-preallocated-contract.json",
    )
    .expect("read contract fixture");
    let like = &mut schema["documentSchemas"]["like"];
    like["properties"]["nonce"] = json!({
        "type": "array", "byteArray": true, "minItems": 33, "maxItems": 33, "position": 2
    });
    like["required"]
        .as_array_mut()
        .expect("required fields")
        .push(json!("nonce"));
    for index in like["indices"].as_array_mut().expect("indices") {
        if index["preallocated"] == true {
            index["terminal"] = json!(["nonce", "$ownerId"]);
        }
    }
    let contract = DataContract::try_from_platform_versioned(
        serde_json::from_value(schema).expect("contract serialization format"),
        false,
        &mut vec![],
        platform_version,
    )
    .expect("parse composite-terminal contract");
    let post_type = contract.document_type_for_name("post").expect("post type");
    let post = post_type
        .random_document(Some(1), platform_version)
        .expect("post");
    let mut layers = Some(HashMap::new());
    drive
        .add_preallocated_index_tree_operations_for_referring_types(
            &DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&post, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type: post_type,
            },
            &mut None,
            &mut layers,
            None,
            &mut vec![],
            platform_version,
        )
        .expect("estimate preallocation");

    let mut member_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "like");
    member_path.extend([b"postId".to_vec(), post.id().to_vec(), vec![0]]);
    let layers = layers.expect("estimated layers");
    let member_layer = layers
        .get(&KeyInfoPath::from_known_owned_path(member_path))
        .expect("byPost member layer must be estimated");
    // A 33-byte nonce followed by the 32-byte owner identifier keys this bucket.
    assert!(
        matches!(member_layer.estimated_layer_sizes, AllItems(65, _, _)),
        "the estimate must cover every terminal component: {member_layer:?}",
    );
}
