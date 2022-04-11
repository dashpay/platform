const { Suite, Test } = require('mocha');

class DocumentsBenchmark {
  /**
   * @type {Object}
   */
  #config;

  /**
   * @param {Object} config
   */
  constructor(config) {
    this.#config = config;
  }

  /**
   * @param {Context} context
   * @param {Client} context.dash
   * @param {Identity} context.identity
   * @returns {Mocha.Suite}
   */
  createMochaTestSuite(context) {
    const suite = new Suite(this.#config.title, context);

    suite.timeout(650000);

    const documentTypes = this.#config.documentTypes();

    suite.addTest(new Test('Publish Data Contract', async () => {
      const dataContract = await context.dash.platform.contracts.create(
        documentTypes,
        context.identity,
      );

      await context.dash.platform.contracts.publish(
        dataContract,
        context.identity,
      );

      context.dash.getApps().set(this.#config.title, {
        contractId: dataContract.getId(),
        contract: dataContract,
      });
    }));

    for (const documentType of Object.keys(documentTypes)) {
      const documentTypeSuite = new Suite(documentType, suite.ctx);

      for (const documentProperties of this.#config.documents(documentType)) {
        suite.addTest(new Test(`Create document ${documentType}`, async () => {
          const document = await context.dash.platform.documents.create(
            `${this.#config.title}.${documentType}`,
            context.identity,
            documentProperties,
          );

          await context.dash.platform.documents.broadcast({
            create: [document],
          }, context.identity);
        }));
      }

      suite.addSuite(documentTypeSuite);
    }

    return suite;
  }

  getRequiredCredits() {
    return this.#config.requiredCredits;
  }
}

module.exports = DocumentsBenchmark;
