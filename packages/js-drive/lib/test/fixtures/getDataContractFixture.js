const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

/**
 * @return {DataContract}
 */
function getDataContractFixture() {
  const documents = {
    niceDocument: {
      indices: [{
        properties: [
          { name: 'asc' },
        ],
      }],
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
  };

  const dataContract = new DataContract(
    Buffer.alloc(32, 'abcFhdvD').toString('hex'),
    documents,
  );

  dataContract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return dataContract;
}

module.exports = getDataContractFixture;
