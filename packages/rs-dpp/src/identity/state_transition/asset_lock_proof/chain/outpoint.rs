pub mod serde {
    use dashcore::consensus::{Decodable, Encodable};
    use dashcore::OutPoint;
    use serde::de::Error as DeError;
    use serde::de::Visitor;
    use serde::ser::Error as SerError;
    use serde::{self, Deserializer, Serializer};
    use std::fmt::Formatter;

    struct OutPointVisitor;

    impl<'v> Visitor<'v> for OutPointVisitor {
        type Value = OutPoint;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("a outpoint bytes of length 36")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            let mut cursor = std::io::Cursor::new(v);

            let outpoint =
                OutPoint::consensus_decode(&mut cursor).map_err(|e| E::custom(e.to_string()))?;

            Ok(outpoint)
        }
    }

    pub fn serialize<S>(value: &OutPoint, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buffer = Vec::<u8>::new();

        value
            .consensus_encode(&mut buffer)
            .map_err(|e| S::Error::custom(e.to_string()))?;

        serializer.serialize_bytes(&buffer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OutPoint, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(OutPointVisitor)
    }
}
