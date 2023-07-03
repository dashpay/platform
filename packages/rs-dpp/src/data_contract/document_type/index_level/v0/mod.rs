use crate::data_contract::document_type::IndexV0;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct IndexLevelV0 {
    /// the lower index levels from this level
    pub sub_index_levels: BTreeMap<String, IndexLevelV0>,
    /// did an index terminate at this level
    pub has_index_with_uniqueness: Option<bool>,
    /// unique level identifier
    pub level_identifier: u64,
}

impl From<&[IndexV0]> for IndexLevelV0 {
    fn from(indices: &[IndexV0]) -> Self {
        let mut index_level = IndexLevelV0::default();
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
                        IndexLevelV0 {
                            level_identifier: counter,
                            ..Default::default()
                        }
                    });
                if properties_iter.peek().is_none() {
                    current_level.has_index_with_uniqueness = Some(index.unique);
                }
            }
        }

        index_level
    }
}
