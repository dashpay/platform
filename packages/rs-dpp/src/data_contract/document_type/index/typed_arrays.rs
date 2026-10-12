//! The typed arrays an index names (protocol version 14): an index keys a
//! document once by each distinct element of the one typed array it may name.

use crate::data_contract::document_type::property::array::TypedArrayProperty;
use crate::data_contract::document_type::{typed_array_property, DocumentProperty, Index};
use indexmap::IndexMap;

impl Index {
    /// The typed array properties this index names, each with its position,
    /// among the document type's `flattened_properties`. The document type
    /// parser admits at most one per index, so on a parsed document type this
    /// yields at most one; the parser reads a second to refuse it.
    pub fn typed_arrays<'a>(
        &'a self,
        flattened_properties: &'a IndexMap<String, DocumentProperty>,
    ) -> impl Iterator<Item = (usize, &'a str, &'a TypedArrayProperty)> + 'a {
        self.properties
            .iter()
            .enumerate()
            .filter_map(move |(position, property)| {
                typed_array_property(flattened_properties, &property.name)
                    .map(|typed_array| (position, property.name.as_str(), typed_array))
            })
    }

    /// Where the typed array this index keys its documents by
    /// ([`Index::indexed_array`], stamped by the parser on every parse) sits
    /// in the index, and its `maxItems`: the levels from that position on
    /// fan a document out over at most that many elements.
    pub fn indexed_array_fan_out(
        &self,
        flattened_properties: &IndexMap<String, DocumentProperty>,
    ) -> Option<(usize, u16)> {
        let array = self.indexed_array.as_deref()?;
        let position = self
            .properties
            .iter()
            .position(|property| property.name == array)?;
        typed_array_property(flattened_properties, array)
            .map(|typed_array| (position, typed_array.max_items))
    }
}
