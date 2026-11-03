use dpp::data_contract::document_type::property_constraints::{AggregateRead, StoredSystemValues};
use std::collections::BTreeMap;

/// What the `propertyConstraints` rules judging a document write read beyond the document it
/// writes, read once its action is built: the `countOf` and `sumOf` totals, and the system
/// values of the stored version the write replaces. An action holds it boxed, and only while it
/// holds something, since few writes do and the action is one variant of a large enum.
#[derive(Debug, Clone, Default)]
pub struct PropertyConstraintReads {
    /// The totals the rules judging the write read, each as it will be once the write is done.
    pub aggregates: BTreeMap<AggregateRead, i128>,
    /// The owner, times, heights and revision of the stored version the write replaces, which
    /// a rule or a condition reads through `$old.`: `None` for a create, and when nothing the
    /// document type declares reads them.
    pub stored_system_values: Option<StoredSystemValues>,
}

/// What [`PropertyConstraintReads::aggregates_of`] gives for an action holding none.
static NO_AGGREGATES: BTreeMap<AggregateRead, i128> = BTreeMap::new();

impl PropertyConstraintReads {
    /// The totals an action holding `reads` holds, none when it holds nothing.
    pub fn aggregates_of(reads: &Option<Box<Self>>) -> &BTreeMap<AggregateRead, i128> {
        reads
            .as_deref()
            .map_or(&NO_AGGREGATES, |reads| &reads.aggregates)
    }

    /// The stored system values an action holding `reads` holds.
    pub fn stored_system_values_of(reads: &Option<Box<Self>>) -> Option<&StoredSystemValues> {
        reads
            .as_deref()
            .and_then(|reads| reads.stored_system_values.as_ref())
    }

    /// Sets the totals an action holding `reads` holds. An action holding
    /// nothing, given no totals, is left without a box.
    pub fn set_aggregates(
        reads: &mut Option<Box<Self>>,
        aggregates: BTreeMap<AggregateRead, i128>,
    ) {
        if reads.is_none() && aggregates.is_empty() {
            return;
        }
        Self::update(reads, |held| held.aggregates = aggregates);
    }

    /// Sets the stored system values an action holding `reads` holds.
    pub fn set_stored_system_values(reads: &mut Option<Box<Self>>, values: StoredSystemValues) {
        Self::update(reads, |held| held.stored_system_values = Some(values));
    }

    /// Applies `change` to what `reads` holds, leaving no box once it holds nothing.
    fn update(reads: &mut Option<Box<Self>>, change: impl FnOnce(&mut Self)) {
        let held = reads.get_or_insert_with(Box::default);
        change(held);
        if held.aggregates.is_empty() && held.stored_system_values.is_none() {
            *reads = None;
        }
    }
}
