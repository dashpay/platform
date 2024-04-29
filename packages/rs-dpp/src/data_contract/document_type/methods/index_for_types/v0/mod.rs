use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::v0::DocumentTypeV0;

// If another document type ever were to exist we would need to implement index_for_types_v0 again

impl DocumentTypeV0 {
    pub(in crate::data_contract::document_type) fn index_for_types_v0(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
    ) -> Option<(&Index, u16)> {
        let mut best_index: Option<(&Index, u16)> = None;
        let mut best_difference = u16::MAX;
        for (_, index) in self.indices.iter() {
            let difference_option = index.matches(index_names, in_field_name, order_by);
            if let Some(difference) = difference_option {
                if difference == 0 {
                    return Some((index, 0));
                } else if difference < best_difference {
                    best_difference = difference;
                    best_index = Some((index, best_difference));
                }
            }
        }
        best_index
    }
}
