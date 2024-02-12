use crate::Sdk;

/// State transition identifier
pub struct TxId([u8; 32]);
impl TxId {
    /// Checks if the state transition is confirmed
    pub fn is_confirmed(&self, _sdk: &Sdk) -> bool {
        todo!("Not implemented")
    }
}

impl From<TxId> for [u8; 32] {
    fn from(tx_id: TxId) -> Self {
        tx_id.0
    }
}

impl From<[u8; 32]> for TxId {
    fn from(value: [u8; 32]) -> Self {
        TxId(value)
    }
}
