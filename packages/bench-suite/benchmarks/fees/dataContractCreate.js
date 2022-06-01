const { signStateTransition } = require('dash/build/src/SDK/Client/Platform/signStateTransition');

const TYPES = require('../../lib/benchmarks/types');
const createIndices = require('../../lib/util/createIndices');
const createProperties = require('../../lib/util/createProperties');

module.exports = {
  title: 'Fees',

  type: TYPES.STATE_TRANSITIONS,

  /**
   * Number of state transitions to broadcast
   *
   * @type {number}
   */
  stateTransitionsCount: 10,

  /**
   * Define document types
   *
   * It can be function or object
   *
   * @type {Object|Function}
   */
  stateTransitions: {
    /**
     * @param {Context} context - Test context
     * @param {number} i - Call index
     * @returns {AbstractStateTransition}
     */
    'Publish Data Contract': async (context) => {
      const { platform } = context.dash;

      const dataContract = await platform.contracts.create(
        {
          indices: {
            type: 'object',
            indices: createIndices(100),
            properties: createProperties(100, {
              type: 'string',
              maxLength: 63,
            }),
            additionalProperties: false,
          },
          uniqueIndices: {
            type: 'object',
            indices: createIndices(100, true),
            properties: createProperties(100, {
              type: 'string',
              maxLength: 63,
            }),
            additionalProperties: false,
          },
        },
        context.identity,
      );

      const stateTransition = platform.dpp.dataContract.createDataContractCreateTransition(
        dataContract,
      );

      await signStateTransition(platform, stateTransition, context.identity);

      return stateTransition;
    },
  },

  /**
   * How many credits this benchmark requires to run
   *
   * @type {number}
   */
  requiredCredits: 2000000000,

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
