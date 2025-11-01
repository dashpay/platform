const contract = {
  $format_version: '0',
  id: '86LHvdC1Tqx5P97LQUSibGFqf2vnKFpB6VkqQ7oso86e',
  ownerId: '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU',
  version: 1,
  config: {
    $format_version: '0',
    canBeDeleted: false,
    readonly: false,
    keepsHistory: false,
    documentsKeepHistoryContractDefault: false,
    documentsMutableContractDefault: true,
    documentsCanBeDeletedContractDefault: true,
    requiresIdentityEncryptionBoundedKey: null,
    requiresIdentityDecryptionBoundedKey: null,
  },
  documentSchemas: {
    card: {
      type: 'object',
      documentsMutable: false,
      canBeDeleted: true,
      transferable: 1,
      tradeMode: 1,
      properties: {
        name: {
          type: 'string',
          description: 'Name of the card',
          maxLength: 63,
          position: 0,
        },
        description: {
          type: 'string',
          description: 'Description of the card',
          maxLength: 256,
          position: 1,
        },
        imageUrl: {
          type: 'string',
          description: 'URL of the image associated with the card',
          maxLength: 2048,
          format: 'uri',
          position: 2,
        },
        imageHash: {
          type: 'array',
          description: 'SHA256 hash of the bytes of the image specified by imageUrl',
          byteArray: true,
          minItems: 32,
          maxItems: 32,
          position: 3,
        },
        rarity: {
          type: 'string',
          description: 'Rarity level of the card',
          enum: [
            'common',
            'uncommon',
            'rare',
            'legendary',
          ],
          position: 4,
        },
      },
      required: [
        '$createdAt',
        '$updatedAt',
        'name',
        'description',
        'imageUrl',
        'imageHash',
        'rarity',
      ],
      additionalProperties: false,
      indices: [
        {
          name: 'name',
          properties: [
            {
              name: 'asc',
            },
          ],
          unique: true,
        },
        {
          name: 'rarity',
          properties: [
            {
              rarity: 'asc',
            },
          ],
          unique: false,
        },
      ],
    },
  },
};

export default contract;
