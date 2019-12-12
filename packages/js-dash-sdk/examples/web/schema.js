const schema = {
  "contact": {
    "indices": [
      {
        "unique": true,
        "properties": [
          {
            "$userId": "asc"
          },
          {
            "toUserId": "asc"
          }
        ]
      }
    ],
    "required": [
      "toUserId",
      "publicKey"
    ],
    "properties": {
      "toUserId": {
        "type": "string"
      },
      "publicKey": {
        "type": "string"
      }
    },
    "additionalProperties": false
  },
  "profile": {
    "indices": [
      {
        "unique": true,
        "properties": [
          {
            "$userId": "asc"
          }
        ]
      }
    ],
    "required": [
      "avatarUrl",
      "about"
    ],
    "properties": {
      "about": {
        "type": "string"
      },
      "avatarUrl": {
        "type": "string",
        "format": "url"
      }
    },
    "additionalProperties": false
  }
}
