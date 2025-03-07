const Dash = require('dash');

const crypto = require('crypto');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const {
  Platform,
} = Dash;

let randomOwnerId = null;

/**
 * @param {string|number|bigint} identityNonce
 * @param {Identifier} [ownerId]
 * @return {Promise<DataContract>}
 */
module.exports = async function getDataContractFixture(
  identityNonce,
  ownerId = randomOwnerId,
) {
  const { DataContractFactory, Identifier, getLatestProtocolVersion } = await Platform
    .initializeDppModule();

  if (!randomOwnerId) {
    randomOwnerId = await generateRandomIdentifier();
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
      },
      required: ['firstName', '$createdAt', '$updatedAt', 'lastName'],
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

  return factory.create(ownerId, BigInt(identityNonce), documents, config);
};
