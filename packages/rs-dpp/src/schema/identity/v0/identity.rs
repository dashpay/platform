use serde_json::Value;

pub const IDENTITY_JSON_STRING: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "protocolVersion": {
      "type": "integer",
      "minimum": 0,
      "$comment": "Maximum is the latest protocol version"
    },
    "id": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier"
    },
    "publicKeys": {
      "type": "array",
      "minItems": 1,
      "maxItems": 32,
      "uniqueItems": true
    },
    "balance": {
      "type": "integer",
      "minimum": 0
    },
    "revision": {
      "type": "integer",
      "minimum": 0,
      "description": "Identity update revision"
    }
  },
  "required": [
    "protocolVersion",
    "id",
    "publicKeys",
    "balance",
    "revision"
  ]
}"#;

pub fn identity_json() -> Result<Value, serde_json::Error> {
    serde_json::from_str(IDENTITY_JSON_STRING)
}
