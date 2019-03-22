const Contract = require('../../contract/Contract');

/**
 * @return {Contract}
 */
module.exports = function getContractFixture() {
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

  const contract = new Contract('lovelyContract', documents);

  contract.setDefinitions({
    lastName: {
      type: 'string',
    },
  });

  return contract;
};
