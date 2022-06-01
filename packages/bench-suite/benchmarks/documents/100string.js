const crypto = require('crypto');

const TYPES = require('../../lib/benchmarks/types');

const createProperties = require('../../lib/util/createProperties');

module.exports = {
  title: '100 Strings',

  type: TYPES.DOCUMENTS,

  /**
   * Define document types
   *
   * It can be function or object
   *
   * @type {Object|Function}
   */
  documentTypes: {
    plain: {
      type: 'object',
      properties: createProperties(100, {
        type: 'string',
      }),
      additionalProperties: false,
    },
    regexps100: {
      type: 'object',
      properties: createProperties(100, {
        type: 'string',
        pattern: '^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$',
        maxLength: 63,
      }),
      additionalProperties: false,
    },
  },

  /**
   * Number of documents to create for each type
   *
   * We get 35x3 results running against local network
   * since metrics are gathering from all 3 nodes
   *
   * @type {number}
   */
  documentsCount: 10,

  /**
   * Return document data for specific document type to create
   *
   * Functions will be called "documentsCount" times
   */
  documentsData: {
    /**
     * Calls if specific document type function is not created
     *
     * @param {number} i - Call index
     * @param {string} type - Document type
     * @returns {Object}
     */
    $all() {
      const document = {};

      for (let i = 0; i < 100; i++) {
        const name = `property${i}`;

        document[name] = crypto.randomBytes(20)
          .toString('hex');
      }

      return document;
    },
  },

  /**
   * How many credits this benchmark requires to run
   *
   * @type {number}
   */
  requiredCredits: 1000000,

  /**
   * Statistical function
   *
   * Available functions: https://mathjs.org/docs/reference/functions.html#statistics-functions
   *
   * @type {string}
   */
  avgFunction: 'median',

  /**
   * Show all or only statistic result
   *
   * @type {boolean}
   */
  avgOnly: false,
};
