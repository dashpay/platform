use crate::{Error, Value};
use std::collections::BTreeMap;

pub trait BTreeValueRemoveInnerValueFromMapHelper {
    fn remove_optional_inner_value_array<I: FromIterator<Value>>(
        &mut self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn remove_inner_value_array<I: FromIterator<Value>>(&mut self, key: &str) -> Result<I, Error>;
}

impl BTreeValueRemoveInnerValueFromMapHelper for BTreeMap<String, Value> {
    fn remove_optional_inner_value_array<I: FromIterator<Value>>(
        &mut self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.remove(key)
            .map(|v| v.into_array().map(|vec| vec.into_iter().collect()))
            .transpose()
    }

    fn remove_inner_value_array<I: FromIterator<Value>>(&mut self, key: &str) -> Result<I, Error> {
        self.remove_optional_inner_value_array(key)?
            .ok_or_else(|| Error::StructureError(format!("unable to remove float property {key}")))
    }
}
