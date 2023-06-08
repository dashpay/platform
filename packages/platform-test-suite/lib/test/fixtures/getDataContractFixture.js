const Dash = require('dash');

const crypto = require('crypto');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const {
  Platform,
} = Dash;

let randomOwnerId = null;

/**
 *
 * @param {Identifier} [ownerId]
 * @return {Promise<DataContract>}
 */
module.exports = async function getDataContractFixture(
  ownerId = randomOwnerId,
) {
  const { DataContractFactory, DataContractValidator, Identifier } = await Platform
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
        },
        identifierField: {
          type: 'array',
          byteArray: true,
          contentMediaType: Identifier.MEDIA_TYPE,
          minItems: 32,
          maxItems: 32,
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
        },
        lastName: {
          type: 'string',
          maxLength: 63,
        },
      },
      required: ['firstName', '$createdAt', '$updatedAt', 'lastName'],
      additionalProperties: false,
    },
  };

  const dataContractValidator = new DataContractValidator();
  const entropyGenerator = {
    generate() {
      return crypto.randomBytes(32);
    },
  };
  const factory = new DataContractFactory(
    protocolVersion.latestVersion,
    dataContractValidator,
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
