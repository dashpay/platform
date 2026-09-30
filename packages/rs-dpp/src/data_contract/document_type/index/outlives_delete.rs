//! What an index whose entries outlive a delete of their document
//! (`outlivesDelete`, see [`super::OUTLIVES_DELETE`]) changes about an
//! indexOnly row.

use crate::data_contract::document_type::Index;
use crate::document::property_names::CREATED_AT;
use std::collections::BTreeSet;

/// Whether an indexOnly row commits to its `$createdAt`, so that a delete of
/// it must carry the value: when the type requires `$createdAt`, unless every
/// index involving it outlives a delete (at least one does). Such indexes are
/// the only ones keyed by the timestamp, and a delete leaves their entries, so
/// neither the delete nor the commitment binding the entries it removes needs
/// the value. A type that requires `$createdAt` but indexes it nowhere commits
/// to it, as every type did before `outlivesDelete`.
///
/// Shared by the parser, the delete transition's construction and its
/// validation, and Drive's row commitment, so the four agree.
pub fn index_only_row_commits_created_at<'a>(
    required_fields: &BTreeSet<String>,
    indexes: impl IntoIterator<Item = &'a Index>,
) -> bool {
    if !required_fields.contains(CREATED_AT) {
        return false;
    }
    let mut involved = false;
    for index in indexes {
        if index.involves(CREATED_AT) {
            if !index.outlives_delete {
                return true;
            }
            involved = true;
        }
    }
    !involved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::IndexProperty;

    fn index(properties: &[&str], outlives_delete: bool) -> Index {
        Index {
            name: properties.join("_"),
            properties: properties
                .iter()
                .map(|name| IndexProperty {
                    name: name.to_string(),
                    ascending: true,
                })
                .collect(),
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: Default::default(),
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_countable_at: vec![],
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: Some(vec!["$ownerId".to_string()]),
            preallocated: false,
            outlives_delete,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
        }
    }

    #[test]
    fn should_commit_created_at_unless_only_outliving_indexes_involve_it() {
        let required: BTreeSet<String> = ["$createdAt".to_string()].into();
        let by_post = index(&["postId"], false);
        let trend = index(&["$createdAt", "postId"], true);
        let by_time = index(&["postId", "$createdAt"], false);

        assert!(!index_only_row_commits_created_at(
            &required,
            [&by_post, &trend]
        ));
        assert!(index_only_row_commits_created_at(
            &required,
            [&by_post, &trend, &by_time]
        ));
        // Required but indexed nowhere: committed, as before the keyword
        assert!(index_only_row_commits_created_at(&required, [&by_post]));
        // Not required: never
        assert!(!index_only_row_commits_created_at(
            &BTreeSet::new(),
            [&by_time]
        ));
    }
}
