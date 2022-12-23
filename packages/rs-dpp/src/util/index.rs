use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;

use crate::data_contract::extra::IndexProperty;

// Indices documentation:  https://dashplatform.readme.io/docs/reference-data-contracts#document-indices
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Index {
    pub name: String,
    pub properties: Vec<IndexProperty>,
    #[serde(default)]
    pub unique: bool,
}

impl TryFrom<IndexWithRawProperties> for Index {
    type Error = anyhow::Error;

    fn try_from(raw_index: IndexWithRawProperties) -> Result<Self, Self::Error> {
        let properties = raw_index
            .properties
            .into_iter()
            .map(IndexProperty::try_from)
            .collect::<Result<Vec<IndexProperty>, anyhow::Error>>()?;

        Ok(Self {
            name: raw_index.name,
            unique: raw_index.unique,
            properties,
        })
    }
}

// The intermediate structure that holds the `BTreeMap<String, String>` instead of [`IndexProperty`]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct IndexWithRawProperties {
    pub name: String,
    pub properties: Vec<BTreeMap<String, String>>,
    #[serde(default)]
    pub unique: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Eq)]
pub enum OrderBy {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}
