use crate::consensus::basic::data_contract::{
    DuplicateIndexError, SystemPropertyIndexAlreadyPresentError, UniqueIndicesLimitReachedError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::Index;
use crate::version::PlatformVersion;
use crate::{NonConsensusError, ProtocolError};
use std::collections::BTreeMap;
use std::convert::TryFrom;

pub const NOT_ALLOWED_SYSTEM_PROPERTIES: [&str; 1] = ["$id"];

#[derive(Debug, PartialEq, Default, Clone)]
pub struct IndexLevel {
    /// the lower index levels from this level
    pub sub_index_levels: BTreeMap<String, IndexLevel>,
    /// did an index terminate at this level
    pub has_index_with_uniqueness: Option<bool>,
    /// unique level identifier
    pub level_identifier: u64,
}

const UNIQUE_INDEX_LIMIT_V0: usize = 16;

impl IndexLevel {
    pub fn try_from_indices(
        indices: &[Index],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .index_versions
            .index_levels_from_indices
        {
            0 => Self::try_from_indices_v0(indices, document_type_name),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IndexLevel::try_from_indices".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn try_from_indices_v0(
        indices: &[Index],
        document_type_name: &str,
    ) -> Result<Self, ProtocolError> {
        let mut index_level = IndexLevel::default();
        let mut unique_index_counter = 0;
        let mut counter: u64 = 0;
        for index in indices {
            let mut current_level = &mut index_level;
            let mut properties_iter = index.properties.iter().peekable();
            while let Some(index_part) = properties_iter.next() {
                if NOT_ALLOWED_SYSTEM_PROPERTIES.contains(&index_part.name.as_str()) {
                    return Err(ConsensusError::BasicError(
                        BasicError::SystemPropertyIndexAlreadyPresentError(
                            SystemPropertyIndexAlreadyPresentError::new(
                                document_type_name.to_owned(),
                                index.name.to_owned(),
                                index_part.name.to_owned(),
                            ),
                        ),
                    )
                    .into());
                }
                current_level = current_level
                    .sub_index_levels
                    .entry(index_part.name.clone())
                    .or_insert_with(|| {
                        counter += 1;
                        IndexLevel {
                            level_identifier: counter,
                            ..Default::default()
                        }
                    });
                if properties_iter.peek().is_none() {
                    if current_level.has_index_with_uniqueness.is_some() {
                        // an index already exists return error
                        return Err(ConsensusError::BasicError(BasicError::DuplicateIndexError(
                            DuplicateIndexError::new(
                                document_type_name.to_owned(),
                                index.name.to_owned(),
                            ),
                        ))
                        .into());
                    }
                    current_level.has_index_with_uniqueness = Some(index.unique);
                    if index.unique {
                        unique_index_counter += 1;
                        if unique_index_counter > UNIQUE_INDEX_LIMIT_V0 {
                            return Err(ConsensusError::BasicError(
                                BasicError::UniqueIndicesLimitReachedError(
                                    UniqueIndicesLimitReachedError::new(
                                        document_type_name.to_owned(),
                                        UNIQUE_INDEX_LIMIT_V0,
                                    ),
                                ),
                            )
                            .into());
                        }
                    }
                }
            }
        }

        Ok(index_level)
    }
}
