const crypto = require('crypto');

const dpnsDocumentTypes = require('@dashevo/dpns-contract/schema/dpns-contract-documents');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const TYPES = require('../../lib/benchmarks/types');

module.exports = {
  title: 'DPNS data contract',

  type: TYPES.DOCUMENTS,

  /**
   * Define document types
   *
   * It can be function or object
   *
   * @type {Object|Function}
   */
  documentTypes: dpnsDocumentTypes,

  /**
   * Return documents to create
   *
   * this function calling for each document type
   *
   * @param {string} type
   * @returns {Object[]}
   */
  // eslint-disable-next-line no-unused-vars
  documents: (type) => {
    if (type !== 'domain') {
      return [];
    }

    return new Array(35).fill(null).map(() => {
      const label = crypto.randomBytes(10).toString('hex');

      return {
        label,
        normalizedLabel: label.toLowerCase(),
        normalizedParentDomainName: 'dash',
        preorderSalt: crypto.randomBytes(32),
        records: {
          dashUniqueIdentityId: generateRandomIdentifier(),
        },
        subdomainRules: {
          allowSubdomains: false,
        },
      };
    });
  },

  /**
   * How many credits this benchmark requires to run
   *
   * @type {number}
   */
  requiredCredits: 100000,

  /**
   * Statistical function
   *
   * Available functions: https://mathjs.org/docs/reference/functions.html#statistics-functions
   *
   * @type {string}
   */
  avgFunction: 'mean',

  /**
   * Show all or only statistic result
   *
   * @type {boolean}
   */
  avgOnly: true,
};
