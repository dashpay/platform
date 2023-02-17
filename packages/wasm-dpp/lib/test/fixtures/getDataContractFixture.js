const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const generateRandomIdentifierAsync = require('../utils/generateRandomIdentifierAsync');
const { default: loadWasmDpp } = require('../../..');
let { DataContractFactory, Identifier, DataContractValidator } = require('../../..');

let randomOwnerId = null;

/**
 *
 * @param {Buffer} [ownerId]
 * @return {Promise<DataContract>}
 */
module.exports = async function getDataContractFixture(ownerId = randomOwnerId) {
  ({ DataContractFactory, Identifier, DataContractValidator } = await loadWasmDpp());

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
        },
        lastName: {
          type: 'string',
          maxLength: 63,
        },
        otherProperty: {
          type: 'string',
          maxLength: 42,
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
    optionalUniqueIndexedDocument: {
      type: 'object',
      properties: {
        firstName: {
          type: 'string',
          maxLength: 63,
        },
        lastName: {
          type: 'string',
          maxLength: 63,
        },
        country: {
          type: 'string',
          maxLength: 63,
        },
        city: {
          type: 'string',
          maxLength: 63,
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

  const dataContractValidator = new DataContractValidator();
  const factory = new DataContractFactory(
    protocolVersion.latestProtocolVersion,
    dataContractValidator,
  );

  const dataContract = factory.create(ownerId, documents);

  // dataContract.setDefinitions({
  //   lastName: {
  //     type: 'string',
  //   },
  // });

  return dataContract;
};
