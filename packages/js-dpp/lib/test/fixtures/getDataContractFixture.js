const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const DataContractFactory = require('../../dataContract/DataContractFactory');

const randomOwnerId = generateRandomIdentifier();

const Identifier = require('../../identifier/Identifier');
const createDPPMock = require('../mocks/createDPPMock');

/**
 *
 * @param {Buffer} [ownerId]
 * @return {DataContract}
 */
module.exports = function getDataContractFixture(ownerId = randomOwnerId) {
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
    prettyDocument: {
      type: 'object',
      properties: {
        lastName: {
          $ref: '#/$defs/lastName',
        },
      },
      required: ['lastName', '$updatedAt'],
      additionalProperties: false,
    },
    indexedDocument: {
      type: 'object',
      indices: [
        {
          properties: [
            { $ownerId: 'asc' },
            { firstName: 'desc' },
          ],
          unique: true,
        },
        {
          properties: [
            { $ownerId: 'asc' },
            { lastName: 'desc' },
          ],
          unique: true,
        },
        {
          properties: [
            { $id: 'asc' },
            { lastName: 'asc' },
          ],
        },
        {
          properties: [
            { $createdAt: 'asc' },
            { $updatedAt: 'asc' },
          ],
        },
        {
          properties: [
            { $updatedAt: 'asc' },
          ],
        },
      ],
      properties: {
        firstName: {
          type: 'string',
          maxLength: 256,
        },
        lastName: {
          type: 'string',
          maxLength: 256,
        },
      },
      required: ['firstName', '$createdAt', '$updatedAt', 'lastName'],
      additionalProperties: false,
    },
    indexedArray: {
      type: 'object',
      indices: [
        {
          properties: [
            { mentions: 'asc' },
          ],
        },
      ],
      properties: {
        mentions: {
          type: 'array',
          prefixItems: [
            {
              type: 'string',
            },
          ],
          minItems: 1,
          maxItems: 5,
          items: false,
        },
      },
      additionalProperties: false,
    },
    noTimeDocument: {
      type: 'object',
      properties: {
        name: {
          type: 'string',
        },
      },
      additionalProperties: false,
    },
    uniqueDates: {
      type: 'object',
      indices: [
        {
          properties: [
            { $createdAt: 'asc' },
            { $updatedAt: 'asc' },
          ],
          unique: true,
        },
        {
          properties: [
            { $updatedAt: 'asc' },
          ],
        },
      ],
      properties: {
        firstName: {
          type: 'string',
        },
        lastName: {
          type: 'string',
        },
      },
      required: ['firstName', '$createdAt', '$updatedAt'],
      additionalProperties: false,
    },
    withByteArrays: {
      type: 'object',
      indices: [
        {
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
    optionalUniqueIndexedDocument: {
      type: 'object',
      properties: {
        firstName: {
          type: 'string',
          maxLength: 256,
        },
        lastName: {
          type: 'string',
          maxLength: 256,
        },
        country: {
          type: 'string',
          maxLength: 256,
        },
        city: {
          type: 'string',
          maxLength: 256,
        },
      },
      indices: [
        {
          properties: [
            { firstName: 'desc' },
          ],
          unique: true,
        },
        {
          properties: [
            { $id: 'asc' },
            { $ownerId: 'asc' },
            { firstName: 'asc' },
            { lastName: 'asc' },
          ],
          unique: true,
        },
        {
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

  const factory = new DataContractFactory(createDPPMock(), () => {});

  const dataContract = factory.create(ownerId, documents);

  dataContract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return dataContract;
};
