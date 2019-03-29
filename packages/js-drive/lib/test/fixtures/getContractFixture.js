const Contract = require('@dashevo/dpp/lib/contract/Contract');

/**
 * @return Contract
 */
function getContractFixture() {
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
  };

  const contract = new Contract('Contract', documents);

  contract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return contract;
}

module.exports = getContractFixture;
