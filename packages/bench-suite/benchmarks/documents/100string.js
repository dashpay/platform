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

    // you will get x3 results running against local network
    // since metrics are gathering from all 3 nodes
    return new Array(35).map(() => {
      const properties = {};

      for (let i = 0; i <= 100; i++) {
        const name = `property${i}`;

        properties[name] = crypto.randomBytes(20).toString('hex');
      }

      return properties;
    });
  },

  requiredCredits: 50000,

  // https://mathjs.org/docs/reference/functions.html#statistics-functions
  avgFunction: 'mean',
};
