use bincode::{Decode, Encode};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[repr(u8)]
#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug, Encode, Decode, Default,
)]
#[ferment_macro::export]
pub enum Pooling {
    #[default]
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}
