use crate::data_contract::document_type::DocumentTypeRef;

impl<'a> DocumentTypeRef<'a> {
    /// The property each summing declaration of the type names: `documentsSummable`, which
    /// `documentsAverageable` sets too, then the `summable` of each index, which `averageable`
    /// sets too. Drive reads each of them from a document it writes into the type's sums.
    /// Registration holds every declaration of a type to one property
    /// (`apply_doctype_aggregates`); each is listed all the same, so that what reads them
    /// covers whatever a declaration names.
    pub fn summed_properties(self) -> Vec<&'a str> {
        let (documents_summable, indices) = match self {
            DocumentTypeRef::V0(v0) => (None, &v0.indices),
            DocumentTypeRef::V1(v1) => (None, &v1.indices),
            DocumentTypeRef::V2(v2) => (v2.documents_summable.as_deref(), &v2.indices),
        };
        documents_summable
            .into_iter()
            .chain(
                indices
                    .values()
                    .filter_map(|index| index.summable.as_deref()),
            )
            .collect()
    }
}
