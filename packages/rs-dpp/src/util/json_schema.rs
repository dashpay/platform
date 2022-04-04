use crate::{data_contract::JsonSchema, errors::ProtocolError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Indices documentation:  https://dashplatform.readme.io/docs/reference-data-contracts#document-indices
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Index {
    pub name: String,
    pub properties: BTreeMap<String, OrderBy>,
    #[serde(default)]
    pub unique: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone)]
pub enum OrderBy {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

// TODO this should be implemented as method for JsonSchema data type. Additionally, the
// TODO function shouldn't return an error. The validation of struct should be performed while creating the JsonSchema type
pub fn get_indices_from_json_schema(
    document_schema: &JsonSchema,
) -> Result<Vec<Index>, ProtocolError> {
    match document_schema.get("indices") {
        Some(raw_indices) => Ok(serde_json::from_value(raw_indices.to_owned())?),
        None => Ok(vec![]),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_indices() {
        let input = json!({
            "properties" : {
                "field_one" : {
                    "type" : "string"
                },
                "field_two" : {
                    "type" : "string"
                }
            },
            "indices" : [
                {
                    "name" : "first_index",
                    "properties" : {
                        "field_one" : "asc",
                        "field_two" : "desc"
                    },
                    "unique" : true

                },
                {
                    "name" : "second_index",
                    "properties" : {
                        "field_two" : "desc",
                    }
                }
             ]
        });

        let indices_result = get_indices_from_json_schema(&input);
        let indices = indices_result.unwrap();
        assert_eq!(indices.len(), 2);
        assert_eq!(indices[0].name, "first_index");
        assert_eq!(indices[0].properties.len(), 2);
        assert_eq!(indices[0].properties["field_one"], OrderBy::Asc);
        assert_eq!(indices[0].properties["field_two"], OrderBy::Desc);
        assert!(indices[0].unique);

        assert_eq!(indices[1].name, "second_index");
        assert_eq!(indices[1].properties.len(), 1);
        assert!(!indices[1].unique);
    }
}
