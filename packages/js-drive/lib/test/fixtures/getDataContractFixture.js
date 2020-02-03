const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');
const generateRandomId = require('@dashevo/dpp/lib/test/utils/generateRandomId');

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
    generateRandomId(),
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
