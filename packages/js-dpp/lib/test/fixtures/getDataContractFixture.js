const generateRandomId = require('../utils/generateRandomId');

const DataContractFactory = require('../../dataContract/DataContractFactory');

const randomOwnerId = generateRandomId();

/**
 *
 * @param {Buffer} [ownerId]
 * @return {DataContract}
 */
module.exports = function getDataContractFixture(ownerId = randomOwnerId) {
  const documents = {
    niceDocument: {
      properties: {
        name: {
          type: 'string',
        },
      },
      required: ['$createdAt'],
      additionalProperties: false,
    },
    prettyDocument: {
      properties: {
        lastName: {
          $ref: '#/definitions/lastName',
        },
      },
      required: ['lastName', '$updatedAt'],
      additionalProperties: false,
    },
    indexedDocument: {
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
    noTimeDocument: {
      properties: {
        name: {
          type: 'string',
        },
      },
      additionalProperties: false,
    },
    uniqueDates: {
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
    withContentEncoding: {
      properties: {
        base64Field: {
          type: 'string',
          contentEncoding: 'base64',
          maxLength: 16,
          pattern: '^([A-Za-z0-9+/])+$',
        },
        base58Field: {
          type: 'string',
          contentEncoding: 'base58',
          maxLength: 16,
          pattern: '^[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]+$',
        },
      },
      required: ['base64Field', 'base58Field'],
      additionalProperties: false,
    },
    optionalUniqueIndexedDocument: {
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

  const factory = new DataContractFactory(
    () => {},
  );

  const dataContract = factory.create(ownerId.toBuffer(), documents);

  dataContract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return dataContract;
};
