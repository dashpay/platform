//! Explicit adapters for foreign Serde types in Platform's binary wire format.

/// Txid's binary Serde visitor accepts exactly 32 bytes. The transparent wrapper
/// preserves its length-prefixed wire encoding while opting this specific graph
/// into the guarded Serde adapter; it does not opt in arbitrary Dash Core types.
#[derive(serde::Deserialize)]
#[serde(transparent)]
struct UntrustedTxid(dashcore::Txid);

impl<'de> bincode::serde::DeserializeUntrusted<'de> for UntrustedTxid {}

pub(crate) fn decode_txid<D: bincode::de::UntrustedDecoder>(
    decoder: &mut D,
) -> Result<dashcore::Txid, bincode::error::DecodeError> {
    let bincode::serde::Compat(UntrustedTxid(txid)) =
        bincode::DecodeUntrusted::decode_untrusted(decoder)?;
    Ok(txid)
}
