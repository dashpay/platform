const DapContract = require('../../dapContract/DapContract');

/**
 * @return DapContract
 */
module.exports = function getLovelyDapContract() {
  const dapObjectsDefinition = {
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

  const dapContract = new DapContract('lovelyContract', dapObjectsDefinition);

  dapContract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return dapContract;
};
