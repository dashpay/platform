const DPContract = require('../../contract/DPContract');

/**
 * @return DPContract
 */
module.exports = function getDPContractFixture() {
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

  const dpContract = new DPContract('lovelyContract', dpObjectsDefinition);

  dpContract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return dpContract;
};
