use bincode::{config, Decode, Encode};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::identity::SecurityLevel::{CRITICAL, HIGH, MEDIUM};

#[repr(u8)]
#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug, Encode, Decode, Default,
)]
pub enum Pooling {
    #[default]
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}
