const Dash = require('dash');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const {
  PlatformProtocol: {
    DataContractFactory,
    Identifier,
  },
} = Dash;

const randomOwnerId = generateRandomIdentifier();

/**
 *
 * @param {Identifier} [ownerId]
 * @return {DataContract}
 */
module.exports = function getDataContractFixture(
  ownerId = randomOwnerId,
) {
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
            { firstName: 'desc' },
          ],
          unique: true,
        },
        {
          name: 'index2',
          properties: [
            { $ownerId: 'asc' },
            { lastName: 'desc' },
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

  const dpp = {
    getProtocolVersion: () => 1,
  };

  const factory = new DataContractFactory(dpp, () => {});

  return factory.create(ownerId, documents);
};
