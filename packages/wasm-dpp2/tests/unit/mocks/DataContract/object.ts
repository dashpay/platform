import bs58 from 'bs58';

// Object format uses Uint8Array for identifiers and BigInt for integers
export default {
  $format_version: '0',
  id: bs58.decode('4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6'),
  config: {
    $format_version: '0',
    canBeDeleted: false,
    readonly: false,
    keepsHistory: false,
    documentsKeepHistoryContractDefault: false,
    documentsMutableContractDefault: true,
    documentsCanBeDeletedContractDefault: true,
    requiresIdentityEncryptionBoundedKey: undefined,
    requiresIdentityDecryptionBoundedKey: undefined,
  },
  version: 1n,
  ownerId: bs58.decode('11111111111111111111111111111111'),
  schemaDefs: null,
  documentSchemas: {
    withdrawal: {
      description:
        'Withdrawal document to track underlying withdrawal transactions. Withdrawals should be created with IdentityWithdrawalTransition',
      creationRestrictionMode: 2n,
      type: 'object',
      indices: [
        {
          name: 'identityStatus',
          properties: [{ $ownerId: 'asc' }, { status: 'asc' }, { $createdAt: 'asc' }],
          unique: false,
        },
        {
          name: 'identityRecent',
          properties: [{ $ownerId: 'asc' }, { $updatedAt: 'asc' }, { status: 'asc' }],
          unique: false,
        },
        {
          name: 'pooling',
          properties: [
            { status: 'asc' },
            { pooling: 'asc' },
            { coreFeePerByte: 'asc' },
            { $updatedAt: 'asc' },
          ],
          unique: false,
        },
        {
          name: 'transaction',
          properties: [{ status: 'asc' }, { transactionIndex: 'asc' }],
          unique: false,
        },
      ],
      properties: {
        transactionIndex: {
          type: 'integer',
          description:
            'Sequential index of asset unlock (withdrawal) transaction. Populated when a withdrawal pooled into withdrawal transaction',
          minimum: 1n,
          position: 0n,
        },
        transactionSignHeight: {
          type: 'integer',
          description: 'The Core height on which transaction was signed',
          minimum: 1n,
          position: 1n,
        },
        amount: {
          type: 'integer',
          description: 'The amount to be withdrawn',
          minimum: 1000n,
          position: 2n,
        },
        coreFeePerByte: {
          type: 'integer',
          description: 'This is the fee that you are willing to spend for this transaction in Duffs/Byte',
          minimum: 1n,
          maximum: 4294967295n,
          position: 3n,
        },
        pooling: {
          type: 'integer',
          description: 'This indicated the level at which Platform should try to pool this transaction',
          enum: [0n, 1n, 2n],
          position: 4n,
        },
        outputScript: {
          type: 'array',
          byteArray: true,
          minItems: 23n,
          maxItems: 25n,
          position: 5n,
        },
        status: {
          type: 'integer',
          enum: [0n, 1n, 2n, 3n, 4n],
          description: '0 - Pending, 1 - Signed, 2 - Broadcasted, 3 - Complete, 4 - Expired',
          position: 6n,
        },
      },
      additionalProperties: false,
      required: [
        '$createdAt',
        '$updatedAt',
        'amount',
        'coreFeePerByte',
        'pooling',
        'outputScript',
        'status',
      ],
    },
  },
};
