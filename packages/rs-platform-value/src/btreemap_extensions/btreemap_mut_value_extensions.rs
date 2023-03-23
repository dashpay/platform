use crate::{Error, Value};
use std::borrow::BorrowMut;
use std::collections::BTreeMap;

pub trait BTreeMutValueMapHelper {
    fn get_optional_inner_map_in_array_mut<
        'a,
        M: FromIterator<(String, &'a mut Value)>,
        I: FromIterator<M>,
    >(
        &'a mut self,
        key: &str,
    ) -> Result<Option<I>, Error>;
    fn get_inner_map_in_array_mut<
        'a,
        M: FromIterator<(String, &'a mut Value)>,
        I: FromIterator<M>,
    >(
        &'a mut self,
        key: &str,
    ) -> Result<I, Error>;
}

impl<V> BTreeMutValueMapHelper for BTreeMap<String, V>
where
    V: BorrowMut<Value>,
{
    fn get_optional_inner_map_in_array_mut<
        'a,
        M: FromIterator<(String, &'a mut Value)>,
        I: FromIterator<M>,
    >(
        &'a mut self,
        key: &str,
    ) -> Result<Option<I>, Error> {
        self.get_mut(key)
            .map(|v| {
                v.borrow_mut()
                    .as_array_mut()
                    .map(|vec| {
                        vec.iter_mut()
                            .map(|v| v.to_ref_string_map_mut::<M>())
                            .collect::<Result<I, Error>>()
                    })
                    .ok_or_else(|| Error::StructureError(format!("{key} must be a an array")))
            })
            .transpose()?
            .transpose()
    }

    fn get_inner_map_in_array_mut<
        'a,
        M: FromIterator<(String, &'a mut Value)>,
        I: FromIterator<M>,
    >(
        &'a mut self,
        key: &str,
    ) -> Result<I, Error> {
        self.get_optional_inner_map_in_array_mut(key)?
            .ok_or_else(|| {
                Error::StructureError(format!("unable to get inner value array property {key}"))
            })
    }
}
