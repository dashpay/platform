const crypto = require('crypto');

const dpnsDocumentTypes = require('@dashevo/dpns-contract/schema/v1/dpns-contract-documents.json');

const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

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
  documentTypes: {
    domain: dpnsDocumentTypes.domain,
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
     * Calls for document type "domain"
     *
     * @param {number} i - Call index
     * @param {string} type - Document type
     * @returns {Object}
     */
    async domain() {
      const label = crypto.randomBytes(10).toString('hex');

      return {
        label,
        normalizedLabel: label.toLowerCase(),
        normalizedParentDomainName: 'dash',
        preorderSalt: crypto.randomBytes(32),
        records: {
          dashUniqueIdentityId: await generateRandomIdentifier(),
        },
        subdomainRules: {
          allowSubdomains: false,
        },
      };
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
