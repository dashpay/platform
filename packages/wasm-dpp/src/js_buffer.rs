use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct JsBuffer {
    pub data: Vec<u8>,
}
