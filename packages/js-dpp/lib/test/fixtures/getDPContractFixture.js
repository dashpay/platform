const DPContract = require('../../contract/DPContract');

/**
 * @return DPContract
 */
module.exports = function getDPContractFixture() {
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

  const dpContract = new DPContract('lovelyContract', documents);

  dpContract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return dpContract;
};
