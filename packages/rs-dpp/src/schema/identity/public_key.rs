use serde_json::Value;

pub const PUBLIC_KEY_JSON_STRING: &'static str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "id": {
      "type": "integer",
      "minimum": 0,
      "description": "Public key ID",
      "$comment": "Must be unique for the identity. It can’t be changed after adding a key. Included when signing state transitions to indicate which identity key was used to sign."
    },
    "type": {
      "type": "integer",
      "enum": [
        0,
        1,
        2
      ],
      "description": "Public key type. 0 - ECDSA Secp256k1, 1 - BLS 12-381, 2 - ECDSA Secp256k1 Hash160",
      "$comment": "It can't be changed after adding a key"
    },
    "purpose": {
      "type": "integer",
      "enum": [
        0,
        1,
        2
      ],
      "description": "Public key purpose. 0 - Authentication, 1 - Encryption, 2 - Decryption",
      "$comment": "It can't be changed after adding a key"
    },
    "securityLevel": {
      "type": "integer",
      "enum": [
        0,
        1,
        2,
        3
      ],
      "description": "Public key security level. 0 - Master, 1 - Critical, 2 - High, 3 - Medium",
      "$comment": "It can't be changed after adding a key"
    },
    "data": true,
    "readOnly": {
      "type": "boolean",
      "description": "Read only",
      "$comment": "Identity public key can't be modified with readOnly set to true. It can’t be changed after adding a key"
    }
  },
  "allOf": [
    {
      "if": {
        "properties": {
          "type": {
            "const": 0
          }
        }
      },
      "then": {
        "properties": {
          "data": {
            "type": "array",
            "byteArray": true,
            "minItems": 33,
            "maxItems": 33,
            "description": "Raw ECDSA public key",
            "$comment": "It must be a valid key of the specified type and unique for the identity. It can’t be changed after adding a key"
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "type": {
            "const": 1
          }
        }
      },
      "then": {
        "properties": {
          "data": {
            "type": "array",
            "byteArray": true,
            "minItems": 48,
            "maxItems": 48,
            "description": "Raw BLS public key",
            "$comment": "It must be a valid key of the specified type and unique for the identity. It can’t be changed after adding a key"
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "type": {
            "const": 2
          }
        }
      },
      "then": {
        "properties": {
          "data": {
            "type": "array",
            "byteArray": true,
            "minItems": 20,
            "maxItems": 20,
            "description": "ECDSA Secp256k1 public key Hash160",
            "$comment": "It must be a valid key hash of the specified type and unique for the identity. It can’t be changed after adding a key"
          }
        }
      }
    }
  ],
  "required": [
    "id",
    "type",
    "data",
    "purpose",
    "securityLevel"
  ],
  "additionalProperties": false
}
"#;

pub fn public_key_json() -> Result<Value, serde_json::Error> {
    serde_json::from_str(PUBLIC_KEY_JSON_STRING)
}