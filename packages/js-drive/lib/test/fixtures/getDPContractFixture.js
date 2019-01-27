const DPContract = require('@dashevo/dpp/lib/contract/DPContract');

/**
 * @return DPContract
 */
function getDPContractFixture() {
  const dpObjectsDefinition = {
    niceObject: {
      properties: {
        name: {
          type: 'string',
        },
      },
      additionalProperties: false,
    },
    prettyObject: {
      properties: {
        lastName: {
          $ref: '#/definitions/lastName',
        },
      },
      required: ['lastName'],
      additionalProperties: false,
    },
  };

  const dpContract = new DPContract('Contract', dpObjectsDefinition);

  dpContract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return dpContract;
}

module.exports = getDPContractFixture;
