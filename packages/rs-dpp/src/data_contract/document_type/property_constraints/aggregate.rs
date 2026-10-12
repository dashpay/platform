//! Where the total an [`AggregateRead`] reads is kept, and what the document
//! being written adds to it. Registration checks with these that a tree keeps
//! every total a rule reads; consensus builds its reads with them.

use super::{AggregateBinding, AggregateKind, AggregateRead};
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::property_names::OWNER_ID;
use crate::ProtocolError;
use platform_value::string_encoding::Encoding;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;

impl AggregateRead {
    /// Whether `counted`, the type the read totals, keeps the total over all
    /// of its documents: `documentsCountable` for a `countOf`, and
    /// `documentsSummable` naming the property for a `sumOf`. Only a read
    /// with an empty filter reads it.
    pub fn whole_type_kept(&self, counted: DocumentTypeRef) -> bool {
        match &self.kind {
            AggregateKind::Count => counted.documents_countable(),
            AggregateKind::Sum { property } => {
                counted.documents_summable() == Some(property.as_str())
            }
        }
    }

    /// The index of `counted`, the type the read totals, whose trees keep the
    /// total of the documents matching the filter: the first in name order
    /// whose properties are exactly the filter's keys, countable for a
    /// `countOf` and summing the property for a `sumOf`, and plain: not unique,
    /// contested, ranked, bucketed (a time or integer range) or an indexOnly
    /// index (one with a terminal, or a `summableOffCountIndex` index), whose
    /// trees keep their totals elsewhere or not at all. `None` for an
    /// empty filter, and when no index answers, which registration refuses.
    pub fn answering_index<'a>(&self, counted: &'a DocumentTypeRef) -> Option<&'a Index> {
        if self.filter.is_empty() {
            return None;
        }
        counted.indexes().values().find(|index| {
            let plain = !index.unique
                && index.contested_index.is_none()
                && !index.is_bucketed()
                && !index.is_index_only()
                && !index.declares_any_ranking();
            // An index lists a property once, so equal lengths and every property
            // among the keys make the two the same set
            let keyed_by_filter = index.properties.len() == self.filter.len()
                && index
                    .properties
                    .iter()
                    .all(|property| self.filter.contains_key(&property.name));
            let keeps_total = match &self.kind {
                AggregateKind::Count => index.countable.is_countable(),
                AggregateKind::Sum { property } => {
                    index.summable.as_deref() == Some(property.as_str())
                }
            };
            plain && keyed_by_filter && keeps_total
        })
    }

    /// The filter's keys with the values they must take, for a document being
    /// written with properties `data` and owner `owner_id`: each value as the
    /// key of `counted`, the type the read totals, holds it, which
    /// `counted.serialize_value_for_key` accepts. `None` when a value the
    /// document gives is missing or is one the key cannot hold: no document of
    /// `counted` can then match, and the total is 0. Registration makes every
    /// value always present and of the key's kind, but the read is built before
    /// the document's schema is validated, which refuses such a document
    /// anyway.
    pub fn filter_values(
        &self,
        counted: DocumentTypeRef,
        data: &Value,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<(String, Value)>>, ProtocolError> {
        let mut values = Vec::with_capacity(self.filter.len());
        for (key, binding) in &self.filter {
            let value = match binding {
                AggregateBinding::Owner => Value::Identifier(owner_id.to_buffer()),
                AggregateBinding::Integer(integer) => Value::I128(*integer),
                AggregateBinding::Property { path, .. } => {
                    match data.get_optional_value_at_path(path) {
                        Ok(Some(value)) if !value.is_null() => value.clone(),
                        _ => return Ok(None),
                    }
                }
                AggregateBinding::Constant(constant) => {
                    let identifier_key = key == OWNER_ID
                        || counted
                            .flattened_properties()
                            .get(key)
                            .is_some_and(|property| property.property_type.is_identifier());
                    if identifier_key {
                        match Identifier::from_string(constant, Encoding::Base58) {
                            Ok(identifier) => Value::Identifier(identifier.to_buffer()),
                            Err(_) => return Ok(None),
                        }
                    } else {
                        Value::Text(constant.clone())
                    }
                }
            };
            if counted
                .serialize_value_for_key(key, &value, platform_version)
                .is_err()
            {
                return Ok(None);
            }
            values.push((key.clone(), value));
        }
        Ok(Some(values))
    }

    /// What the document with properties `data` and owner `owner_id` adds to
    /// the total the read takes over `counted` with `filter_values`
    /// ([`Self::filter_values`]): 0 unless the read totals the writer's own
    /// type and the document matches every key, then 1 for a `countOf` and its
    /// value of the property for a `sumOf`. Consensus reads the stored total
    /// before the write, then takes off what the document added as it was
    /// stored and adds what it adds as it is written.
    pub fn contribution(
        &self,
        counted: DocumentTypeRef,
        filter_values: &[(String, Value)],
        data: &Value,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<i128, ProtocolError> {
        if !self.of_own_type {
            return Ok(0);
        }
        for (key, value) in filter_values {
            let held = if key == OWNER_ID {
                Value::Identifier(owner_id.to_buffer())
            } else {
                match data.get_optional_value_at_path(key) {
                    Ok(Some(held)) if !held.is_null() => held.clone(),
                    _ => return Ok(0),
                }
            };
            let Ok(held) = counted.serialize_value_for_key(key, &held, platform_version) else {
                return Ok(0);
            };
            if held != counted.serialize_value_for_key(key, value, platform_version)? {
                return Ok(0);
            }
        }
        match &self.kind {
            AggregateKind::Count => Ok(1),
            // A value that is no integer adds nothing: the read is built before the
            // document's schema is validated, which refuses such a document anyway
            AggregateKind::Sum { property } => match data.get_optional_value_at_path(property) {
                Ok(Some(value)) => Ok(value.to_integer::<i128>().unwrap_or(0)),
                _ => Ok(0),
            },
        }
    }
}
