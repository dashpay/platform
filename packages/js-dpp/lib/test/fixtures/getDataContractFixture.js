const DataContract = require('../../dataContract/DataContract');

/**
 * @return {DataContract}
 */
module.exports = function getDataContractFixture() {
  const contractId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';
  const documents = {
    niceDocument: {
      properties: {
        name: {
          type: 'string',
        },
      },
      additionalProperties: false,
    },
    prettyDocument: {
      properties: {
        lastName: {
          $ref: '#/definitions/lastName',
        },
      },
      required: ['lastName'],
      additionalProperties: false,
    },
    indexedDocument: {
      indices: [
        {
          properties: [
            { $userId: 'asc' },
            { firstName: 'desc' },
          ],
          unique: true,
        },
        {
          properties: [
            { $userId: 'asc' },
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
      ],
      properties: {
        firstName: {
          type: 'string',
        },
        lastName: {
          type: 'string',
        },
      },
      required: ['firstName'],
      additionalProperties: false,
    },
  };

  const dataContract = new DataContract(contractId, documents);

  dataContract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return dataContract;
};
