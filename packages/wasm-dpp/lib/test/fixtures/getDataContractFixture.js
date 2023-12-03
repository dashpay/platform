const crypto = require('crypto');
const generateRandomIdentifierAsync = require('../utils/generateRandomIdentifierAsync');
const { default: loadWasmDpp } = require('../../..');
const { DataContractFactory, getLatestProtocolVersion, Identifier } = require('../../..');

let randomOwnerId = null;

/**
 *
 * @param {Buffer} [ownerId]
 * @return {Promise<DataContract>}
 */
module.exports = async function getDataContractFixture(ownerId = randomOwnerId) {
  await loadWasmDpp();

  if (!randomOwnerId) {
    randomOwnerId = await generateRandomIdentifierAsync();
  }

  if (!ownerId) {
    // eslint-disable-next-line no-param-reassign
    ownerId = randomOwnerId;
  }

  const documents = {
    niceDocument: {
      type: 'object',
      properties: {
        name: {
          type: 'string',
          position: 0,
        },
      },
      required: ['$createdAt'],
      additionalProperties: false,
    },
    prettyDocument: {
      type: 'object',
      properties: {
        lastName: {
          type: 'string',
          position: 0,
        },
      },
      required: ['lastName', '$updatedAt'],
      additionalProperties: false,
    },
    indexedDocument: {
      type: 'object',
      indices: [
        {
          name: 'index1',
          properties: [
            { $ownerId: 'asc' },
            { firstName: 'asc' },
          ],
          unique: true,
        },
        {
          name: 'index2',
          properties: [
            { $ownerId: 'asc' },
            { lastName: 'asc' },
          ],
          unique: true,
        },
        {
          name: 'index3',
          properties: [
            { lastName: 'asc' },
          ],
          unique: false,
        },
        {
          name: 'index4',
          properties: [
            { $createdAt: 'asc' },
            { $updatedAt: 'asc' },
          ],
        },
        {
          name: 'index5',
          properties: [
            { $updatedAt: 'asc' },
          ],
        },
        {
          name: 'index6',
          properties: [
            { $createdAt: 'asc' },
          ],
        },
      ],
      properties: {
        firstName: {
          type: 'string',
          maxLength: 63,
          position: 0,
        },
        lastName: {
          type: 'string',
          maxLength: 63,
          position: 1,
        },
        otherProperty: {
          type: 'string',
          maxLength: 42,
          position: 2,
        },
      },
      required: ['firstName', '$createdAt', '$updatedAt', 'lastName'],
      additionalProperties: false,
    },
    // indexedArray: {
    //   type: 'object',
    //   indices: [
    //     {
    //       name: 'index1',
    //       properties: [
    //         { mentions: 'asc' },
    //       ],
    //     },
    //   ],
    //   properties: {
    //     mentions: {
    //       type: 'array',
    //       prefixItems: [
    //         {
    //           type: 'string',
    //           maxLength: 100,
    //         },
    //       ],
    //       minItems: 1,
    //       maxItems: 5,
    //       items: false,
    //     },
    //   },
    //   additionalProperties: false,
    // },
    noTimeDocument: {
      type: 'object',
      properties: {
        name: {
          type: 'string',
          position: 0,
        },
      },
      additionalProperties: false,
    },
    uniqueDates: {
      type: 'object',
      indices: [
        {
          name: 'index1',
          properties: [
            { $createdAt: 'asc' },
            { $updatedAt: 'asc' },
          ],
          unique: true,
        },
        {
          name: 'index2',
          properties: [
            { $updatedAt: 'asc' },
          ],
        },
      ],
      properties: {
        firstName: {
          type: 'string',
          position: 0,
        },
        lastName: {
          type: 'string',
          position: 1,
        },
      },
      required: ['firstName', '$createdAt', '$updatedAt'],
      additionalProperties: false,
    },
    withByteArrays: {
      type: 'object',
      indices: [
        {
          name: 'index1',
          properties: [
            { byteArrayField: 'asc' },
          ],
        },
      ],
      properties: {
        byteArrayField: {
          type: 'array',
          byteArray: true,
          maxItems: 16,
          position: 0,
        },
        identifierField: {
          type: 'array',
          byteArray: true,
          contentMediaType: Identifier.MEDIA_TYPE,
          minItems: 32,
          maxItems: 32,
          position: 1,
        },
      },
      required: ['byteArrayField'],
      additionalProperties: false,
    },
    optionalUniqueIndexedDocument: {
      type: 'object',
      properties: {
        firstName: {
          type: 'string',
          maxLength: 63,
          position: 0,
        },
        lastName: {
          type: 'string',
          maxLength: 63,
          position: 1,
        },
        country: {
          type: 'string',
          maxLength: 63,
          position: 2,
        },
        city: {
          type: 'string',
          maxLength: 63,
          position: 3,
        },
      },
      indices: [
        {
          name: 'index1',
          properties: [
            { firstName: 'asc' },
          ],
          unique: true,
        },
        {
          name: 'index2',
          properties: [
            { $ownerId: 'asc' },
            { firstName: 'asc' },
            { lastName: 'asc' },
          ],
          unique: true,
        },
        {
          name: 'index3',
          properties: [
            { country: 'asc' },
            { city: 'asc' },
          ],
          unique: true,
        },
      ],
      required: ['firstName', 'lastName'],
      additionalProperties: false,
    },
  };

  const entropyGenerator = {
    generate() {
      return crypto.randomBytes(32);
    },
  };
  const factory = new DataContractFactory(
    getLatestProtocolVersion(),
    entropyGenerator,
  );

  const config = {
    canBeDeleted: false,
    readonly: false,
    keepsHistory: true,
    documentsKeepHistoryContractDefault: false,
    documentsMutableContractDefault: true,
  };

  return factory.create(ownerId, documents, config);
};
