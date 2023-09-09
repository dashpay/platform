use crate::consensus::basic::data_contract::DuplicateIndexError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::Index;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use std::collections::BTreeMap;
use crate::data_contract::document_type::index_level::IndexType::{ContestedResourceIndex, NonUniqueIndex, UniqueIndex};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum IndexType {
    /// A normal non unique index
    NonUniqueIndex,
    /// A unique index, that means that the values for this index are unique
    /// As long as one of the values is not nil
    UniqueIndex,
    /// A contested resource: This is a unique index but that can be contested through a resolution
    /// The simplest to understand resolution is a masternode vote, but could also be something
    /// like a bidding war.
    /// For example the path/name in the dpns contract must be unique but it is a contested potentially
    /// valuable resource.
    ContestedResourceIndex,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IndexLevel {
    /// the lower index levels from this level
    sub_index_levels: BTreeMap<String, IndexLevel>,
    /// did an index terminate at this level
    has_index_with_type: Option<IndexType>,
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

    pub fn has_index_with_type(&self) -> Option<IndexType> {
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

    pub fn try_from_indices(
        indices: &[Index],
        document_type_name: &str, // TODO: We shouldn't pass document type, it's only for errors
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
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

    fn try_from_indices_v0(
        indices: &[Index],
        document_type_name: &str,
    ) -> Result<Self, ProtocolError> {
        let mut index_level = IndexLevel {
            sub_index_levels: Default::default(),
            has_index_with_type: None,
            level_identifier: 0,
        };

        let mut counter: u64 = 0;

        for index in indices {
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

                    current_level.has_index_with_type = Some(index_type);
                }
            }

            if let Some(contested_index) = &index.contested_index {
                let mut current_level = &mut index_level;
                let mut properties_iter = index.properties.iter().peekable();

                while let Some(index_part) = properties_iter.next() {
                    let level_name = if contested_index.contested_field_name == index_part.name {
                        &contested_index.contested_field_temp_replacement_name
                    } else {
                        &index_part.name
                    };
                    current_level = current_level
                        .sub_index_levels
                        .entry(level_name.clone())
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
                            return Err(ConsensusError::BasicError(
                                BasicError::DuplicateIndexError(DuplicateIndexError::new(
                                    document_type_name.to_owned(),
                                    level_name.clone(),
                                )),
                            )
                            .into());
                        }

                        current_level.has_index_with_type = Some(ContestedResourceIndex);
                    }
                }
            }
        }

        Ok(index_level)
    }
}
