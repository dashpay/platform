const { Suite, Test } = require('mocha');

class DocumentsBenchmark {
  /**
   * @type {Object}
   */
  #options;

  /**
   * @param {Object} options
   */
  constructor(options) {
    this.#options = options;
  }

  /**
   * @param {Context} context
   * @param {Client} context.dash
   * @param {Identity} context.identity
   * @returns {Suite}
   */
  createMochaTestSuite(context) {
    const suite = new Suite(this.#options.title, context);

    suite.addTest(new Test('Publish Data Contract', async () => {
      const dataContract = await context.dash.platform.contracts.create(
        this.#options.documentTypes,
        context.identity,
      );

      await context.dash.platform.contracts.publish(
        dataContract,
        context.identity,
      );
    }));

    return suite;
  }
}

module.exports = DocumentsBenchmark;
