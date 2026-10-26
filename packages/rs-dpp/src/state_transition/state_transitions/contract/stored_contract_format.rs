//! Format version 2 of a contract, the stored format, is the form only state holds: a node
//! compiles it when it stores the contract. A contract transition carries its contract as its
//! signer wrote it, in format version 0 or 1, so the contract transitions decode their contract
//! as a [`TransitionContractFormat`], which refuses the stored format at its tag, before any of
//! it is read, whatever decoder reads the transition and at every protocol version. Software
//! without the format refuses it there the same way, as a variant it does not know.

#[cfg(doc)]
use crate::data_contract::serialized_version::TransitionContractFormat;

/// Gives `$transition` the four decoders bincode derives, each reading `$decoded`, its copy
/// with the contract read as a [`TransitionContractFormat`], and converting it with `From`.
macro_rules! decode_through {
    ($transition:ty, $decoded:ty) => {
        impl<Context> bincode::Decode<Context> for $transition {
            fn decode<D: bincode::de::Decoder<Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                <$decoded as bincode::Decode<Context>>::decode(decoder).map(Self::from)
            }
        }

        impl<'de, Context> bincode::BorrowDecode<'de, Context> for $transition {
            fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                <$decoded as bincode::BorrowDecode<'de, Context>>::borrow_decode(decoder)
                    .map(Self::from)
            }
        }

        impl<Context> bincode::DecodeUntrusted<Context> for $transition {
            fn decode_untrusted<D: bincode::de::UntrustedDecoder<Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                <$decoded as bincode::DecodeUntrusted<Context>>::decode_untrusted(decoder)
                    .map(Self::from)
            }
        }

        impl<'de, Context> bincode::BorrowDecodeUntrusted<'de, Context> for $transition {
            fn borrow_decode_untrusted<
                D: bincode::de::BorrowUntrustedDecoder<'de, Context = Context>,
            >(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                <$decoded as bincode::BorrowDecodeUntrusted<'de, Context>>::borrow_decode_untrusted(
                    decoder,
                )
                .map(Self::from)
            }
        }
    };
}

pub(super) use decode_through;
