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
    ///
    /// # Examples
    /// ```
    /// use std::collections::BTreeMap;
    /// use dpp::data_contract::document_type::IndexLevel;
    ///
    /// let level1 = IndexLevel {
    ///     sub_index_levels: BTreeMap::new(),
    ///     has_index_with_uniqueness: Some(true),
    ///     level_identifier: 1,
    /// };
    ///
    /// let level2 = IndexLevel {
    ///     sub_index_levels: BTreeMap::new(),
    ///     has_index_with_uniqueness: Some(true),
    ///     level_identifier: 2,
    /// };
    ///
    /// let mut root_level = IndexLevel::default();
    /// root_level.sub_index_levels.insert("level1".to_string(), level1.clone());
    /// root_level.sub_index_levels.insert("level2".to_string(), level2.clone());
    ///
    /// // Test if level1 is a subset of root_level
    /// assert_eq!(root_level.contains_subset(&level1), true);
    ///
    /// // Test if level2 is a subset of root_level
    /// assert_eq!(root_level.contains_subset(&level2), true);
    ///
    /// // Test if root_level is a subset of level1 (should be false since the root has more sub-levels)
    /// assert_eq!(level1.contains_subset(&root_level), false);
    /// ```
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
    ///
    /// # Examples
    /// ```
    /// use std::collections::BTreeMap;
    /// use dpp::data_contract::document_type::IndexLevel;
    ///
    /// let level1 = IndexLevel {
    ///     sub_index_levels: BTreeMap::new(),
    ///     has_index_with_uniqueness: Some(true),
    ///     level_identifier: 1,
    /// };
    ///
    /// let level2 = IndexLevel {
    ///     sub_index_levels: BTreeMap::new(),
    ///     has_index_with_uniqueness: Some(true),
    ///     level_identifier: 2,
    /// };
    ///
    /// let mut root_level = IndexLevel::default();
    /// root_level.sub_index_levels.insert("level1".to_string(), level1.clone());
    /// root_level.sub_index_levels.insert("level2".to_string(), level2.clone());
    ///
    /// // Test contains_subset_and_return_invalid_path
    /// assert_eq!(root_level.contains_subset_first_non_subset_path(&level1), None); // level1 is a subset of root_level.
    /// assert_eq!(root_level.contains_subset_first_non_subset_path(&level2), None); // level2 is a subset of root_level.
    /// assert_eq!(level1.contains_subset_first_non_subset_path(&root_level), Some("level1".to_string())); // root_level is NOT a subset of level1. Invalid path: "level1".
    /// ```
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
