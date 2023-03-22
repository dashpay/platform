use crate::{Error, Value};

impl Value {
    pub fn push(&mut self, value: Value) -> Result<(), Error> {
        self.to_array_mut().map(|array| array.push(value))
    }
}
