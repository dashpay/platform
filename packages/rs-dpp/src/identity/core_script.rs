use std::ops::Deref;

use dashcore::Script as DashcoreScript;
use serde::{Deserialize, Serialize};

use crate::{
    util::string_encoding::{self, Encoding},
    ProtocolError,
};

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct CoreScript(DashcoreScript);

impl CoreScript {
    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.0.to_bytes(), encoding)
    }

    pub fn from_string(encoded_value: &str, encoding: Encoding) -> Result<Self, ProtocolError> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Ok(Self(DashcoreScript::from(vec)))
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(DashcoreScript::from(bytes))
    }
}

impl Deref for CoreScript {
    type Target = DashcoreScript;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for CoreScript {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string(Encoding::Base64))
    }
}

impl<'de> Deserialize<'de> for CoreScript {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data: String = Deserialize::deserialize(deserializer)?;

        Self::from_string(&data, Encoding::Base64)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

impl std::fmt::Display for CoreScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string(Encoding::Base64))
    }
}
