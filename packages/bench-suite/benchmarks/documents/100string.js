const crypto = require('crypto');

const TYPES = require('../../lib/benchmarks/types');

module.exports = {
  title: '100 Strings',
  type: TYPES.DOCUMENTS,
  documentTypes: () => {
    const properties = {};

    for (let i = 0; i < 100; i++) {
      const name = `property${i}`;

      properties[name] = {
        type: 'string',
      };
    }

    return {
      test: {
        type: 'object',
        properties,
        additionalProperties: false,
      },
    };
  },
  documents: (type) => {
    if (type !== 'test') {
      return [];
    }

    return new Array(100).map(() => {
      const properties = {};

      for (let i = 0; i <= 100; i++) {
        const name = `property${i}`;

        properties[name] = crypto.randomBytes(20).toString('hex');
      }

      return properties;
    });
  },
  requiredCredits: 100000,
};
