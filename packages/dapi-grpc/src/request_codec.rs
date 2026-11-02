//! Classify protobuf payload failures at the server's request boundary.

use prost::Message;
use std::marker::PhantomData;
use tonic::codec::{BufferSettings, Codec, DecodeBuf, Decoder};
use tonic::Status;
use tonic_prost::{ProstCodec, ProstEncoder};

pub(crate) struct RequestCodec<T, U>(PhantomData<(T, U)>);

impl<T, U> Default for RequestCodec<T, U> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, U> Codec for RequestCodec<T, U>
where
    T: Message + Send + 'static,
    U: Message + Default + Send + 'static,
{
    type Encode = T;
    type Decode = U;
    type Encoder = ProstEncoder<T>;
    type Decoder = RequestDecoder<U>;

    fn encoder(&mut self) -> Self::Encoder {
        ProstCodec::<T, U>::raw_encoder(BufferSettings::default())
    }

    fn decoder(&mut self) -> Self::Decoder {
        RequestDecoder(PhantomData)
    }
}

pub(crate) struct RequestDecoder<U>(PhantomData<U>);

impl<U: Message + Default> Decoder for RequestDecoder<U> {
    type Item = U;
    type Error = Status;

    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> Result<Option<U>, Status> {
        // The generated server uses this decoder only for the caller's protobuf payload.
        // Client response decoding and transport framing errors keep their own statuses.
        Message::decode(buf)
            .map(Some)
            .map_err(|error| Status::invalid_argument(error.to_string()))
    }
}
