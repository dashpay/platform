#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::DataContractInvalidIndexDefinitionUpdateError;
use crate::consensus::basic::data_contract::DuplicateIndexError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::index_level::IndexType::{
    ContestedResourceIndex, NonUniqueIndex, UniqueIndex,
};
use crate::data_contract::document_type::Index;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use std::borrow::Borrow;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum IndexType {
    /// A normal non unique index
    NonUniqueIndex,
    /// A unique index, that means that the values for this index are unique
    /// As long as one of the values is not nil
    UniqueIndex,
    /// A contested resource: This is a unique index but that can be contested through a resolution
    /// The simplest to understand resolution is a masternode votes, but could also be something
    /// like a bidding war.
    /// For example the path/name in the dpns contract must be unique but it is a contested potentially
    /// valuable resource.
    ContestedResourceIndex,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct IndexLevelTypeInfo {
    /// should we insert if all fields up to here are null
    pub should_insert_with_all_null: bool,
    /// The index type
    pub index_type: IndexType,
}

impl IndexType {
    pub fn is_unique(&self) -> bool {
        match self {
            NonUniqueIndex => false,
            UniqueIndex => true,
            ContestedResourceIndex => true,
        }
    }
}

pub type ShouldInsertWithAllNull = bool;

#[derive(Debug, PartialEq, Clone)]
pub struct IndexLevel {
    /// the lower index levels from this level
    sub_index_levels: BTreeMap<String, IndexLevel>,
    /// did an index terminate at this level
    has_index_with_type: Option<IndexLevelTypeInfo>,
    /// unique level identifier
    level_identifier: u64,
}

impl IndexLevel {
    pub fn identifier(&self) -> u64 {
        self.level_identifier
    }

    pub fn sub_levels(&self) -> &BTreeMap<String, IndexLevel> {
        &self.sub_index_levels
    }

    pub fn has_index_with_type(&self) -> Option<IndexLevelTypeInfo> {
        self.has_index_with_type
    }

    /// Checks whether the given `rhs` IndexLevel is a subset of the current IndexLevel (`self`).
    ///
    /// A level is considered a subset if:
    /// - The `level_identifier` of both IndexLevels matches.
    /// - Each sub_index_level in `rhs` is also a subset of the corresponding sub_index_level in `self`.
    ///
    /// # Parameters
    /// - `self`: The current IndexLevel to compare with.
    /// - `rhs`: The IndexLevel to check if it's a subset of `self`.
    ///
    /// # Returns
    /// Returns `true` if `rhs` is a subset of `self`, otherwise `false`.
    pub fn contains_subset(&self, rhs: &IndexLevel) -> bool {
        self.contains_subset_first_non_subset_path(rhs).is_none()
    }

    /// Checks whether the given `rhs` IndexLevel is a subset of the current IndexLevel (`self`).
    /// If `rhs` is a subset, returns `None`. Otherwise, returns the invalid path as an `Option<String>`.
    ///
    /// A level is considered a subset if:
    /// - The `level_identifier` of both IndexLevels matches.
    /// - Each sub_index_level in `rhs` is also a subset of the corresponding sub_index_level in `self`.
    ///
    /// # Parameters
    /// - `self`: The current IndexLevel to compare with.
    /// - `rhs`: The IndexLevel to check if it's a subset of `self`.
    ///
    /// # Returns
    /// Returns `None` if `rhs` is a subset of `self`, otherwise returns `Some(String)` containing the invalid path.
    /// The invalid path is constructed by joining the keys that lead to the first mismatching sub_index_level.
    pub fn contains_subset_first_non_subset_path(&self, rhs: &IndexLevel) -> Option<String> {
        // If the rhs level's identifier doesn't match, it cannot be a subset.
        if self.level_identifier != rhs.level_identifier {
            return Some("Invalid path".to_string());
        }

        // Check if each sub_index_level in the rhs is a subset of self.
        for (key, rhs_sub_index) in &rhs.sub_index_levels {
            match self.sub_index_levels.get(key) {
                Some(self_sub_index) => {
                    // If the rhs sub_index is not a subset of the corresponding self sub_index, return the invalid path.
                    if let Some(invalid_path) =
                        self_sub_index.contains_subset_first_non_subset_path(rhs_sub_index)
                    {
                        return Some(format!("{} -> {}", key, invalid_path));
                    }
                }
                None => return Some(key.to_string()), // Key in rhs not found in self, return the invalid path.
            }
        }

        // If all checks pass, the rhs is a subset of self (return None for no invalid path).
        None
    }

    pub fn try_from_indices<I, T>(
        indices: I,
        document_type_name: &str, // TODO: We shouldn't pass document type, it's only for errors
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        I: IntoIterator<Item = T>, // T is the type of elements in the collection
        T: Borrow<Index>,          // Assuming Index is the type stored in the collection
    {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
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

    fn try_from_indices_v0<I, T>(
        indices: I,
        document_type_name: &str,
    ) -> Result<Self, ProtocolError>
    where
        I: IntoIterator<Item = T>, // T is the type of elements in the collection
        T: Borrow<Index>,          // Assuming Index is the type stored in the collection
    {
        let mut index_level = IndexLevel {
            sub_index_levels: Default::default(),
            has_index_with_type: None,
            level_identifier: 0,
        };

        let mut counter: u64 = 0;

        for index_to_borrow in indices {
            let index = index_to_borrow.borrow();
            let mut current_level = &mut index_level;
            let mut properties_iter = index.properties.iter().peekable();

            while let Some(index_part) = properties_iter.next() {
                current_level = current_level
                    .sub_index_levels
                    .entry(index_part.name.clone())
                    .or_insert_with(|| {
                        counter += 1;
                        IndexLevel {
                            level_identifier: counter,
                            sub_index_levels: Default::default(),
                            has_index_with_type: None,
                        }
                    });

                // The last property
                if properties_iter.peek().is_none() {
                    // This level already has been initialized.
                    // It means there are two indices with the same combination of properties.

                    // We might need to take into account the sorting order when we have it
                    if current_level.has_index_with_type.is_some() {
                        // an index already exists return error
                        return Err(ConsensusError::BasicError(BasicError::DuplicateIndexError(
                            DuplicateIndexError::new(
                                document_type_name.to_owned(),
                                index.name.clone(),
                            ),
                        ))
                        .into());
                    }

                    let index_type = if index.unique {
                        UniqueIndex
                    } else {
                        NonUniqueIndex
                    };

                    // if things are null searchable that means we should insert with all null

                    current_level.has_index_with_type = Some(IndexLevelTypeInfo {
                        should_insert_with_all_null: index.null_searchable,
                        index_type,
                    });
                }
            }
        }

        Ok(index_level)
    }

    #[cfg(feature = "validation")]
    pub fn validate_update(
        &self,
        document_type_name: &str,
        new_indices: &Self,
    ) -> SimpleConsensusValidationResult {
        // There is no changes. All good
        if self == new_indices {
            return SimpleConsensusValidationResult::new();
        }

        // We do not allow any index modifications now, but we want to figure out
        // what changed, so we compare one way then the other

        // If the new contract document type doesn't contain all previous indexes
        if let Some(non_subset_path) = new_indices.contains_subset_first_non_subset_path(self) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    non_subset_path,
                )
                .into(),
            );
        }

        // If the old contract document type doesn't contain all new indexes
        if let Some(non_subset_path) = self.contains_subset_first_non_subset_path(new_indices) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    non_subset_path,
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::IndexProperty;
    use assert_matches::assert_matches;

    #[test]
    fn should_pass_if_indices_are_the_same() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure = old_index_structure.clone();

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_if_new_index_with_only_new_field_is_add() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
        }];

        let new_indices = vec![
            Index {
                name: "test".to_string(),
                properties: vec![IndexProperty {
                    name: "test".to_string(),
                    ascending: false,
                }],
                unique: false,
                null_searchable: true,
                contested_index: None,
            },
            Index {
                name: "test2".to_string(),
                properties: vec![IndexProperty {
                    name: "test2".to_string(),
                    ascending: false,
                }],
                unique: false,
                null_searchable: true,
                contested_index: None,
            },
        ];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test2"
        );
    }

    #[test]
    fn should_return_invalid_result_if_some_indices_are_removed() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![
            Index {
                name: "test".to_string(),
                properties: vec![IndexProperty {
                    name: "test".to_string(),
                    ascending: false,
                }],
                unique: false,
                null_searchable: true,
                contested_index: None,
            },
            Index {
                name: "test2".to_string(),
                properties: vec![IndexProperty {
                    name: "test2".to_string(),
                    ascending: false,
                }],
                unique: false,
                null_searchable: true,
                contested_index: None,
            },
        ];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test2"
        );
    }

    #[test]
    fn should_return_invalid_result_if_additional_property_is_added_to_existing_index() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
        }];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![
                IndexProperty {
                    name: "test".to_string(),
                    ascending: false,
                },
                IndexProperty {
                    name: "test2".to_string(),
                    ascending: false,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test -> test2"
        );
    }

    #[test]
    fn should_return_invalid_result_if_property_is_removed_to_existing_index() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![
                IndexProperty {
                    name: "test".to_string(),
                    ascending: false,
                },
                IndexProperty {
                    name: "test2".to_string(),
                    ascending: false,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
        }];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test -> test2"
        );
    }
}
