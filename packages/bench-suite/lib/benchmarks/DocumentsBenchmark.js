const { Suite, Test } = require('mocha');

const AbstractBenchmark = require('./AbstractBenchmark');
const printMetrics = require('../metrics/drive/printMetrics');
const createStateTransitionMatch = require('../metrics/drive/createStateTransitionMatch');

class DocumentsBenchmark extends AbstractBenchmark {
  /**
   * @type {Object}
   */
  #metrics = {};

  /**
   * @param {Context} context
   * @param {Client} context.dash
   * @param {Identity} context.identity
   * @returns {Mocha.Suite}
   */
  createMochaTestSuite(context) {
    const suite = new Suite(this.config.title, context);

    suite.timeout(650000);

    const documentTypes = typeof this.config.documentTypes === 'function'
      ? this.config.documentTypes()
      : this.config.documentTypes;

    suite.beforeAll('Publish Data Contract', async () => {
      const dataContract = await context.dash.platform.contracts.create(
        documentTypes,
        context.identity,
      );

      if (this.runnerOptions.verbose) {
        // eslint-disable-next-line no-console
        console.dir(context.identity.toJSON(), { depth: Infinity });

        // eslint-disable-next-line no-console
        console.dir(dataContract.toJSON(), { depth: Infinity });
      }

      await context.dash.platform.contracts.publish(
        dataContract,
        context.identity,
      );

      context.dash.getApps().set(this.config.title, {
        contractId: dataContract.getId(),
        contract: dataContract,
      });
    });

    for (const documentType of Object.keys(documentTypes)) {
      const documentTypeSuite = new Suite(documentType, suite.ctx);

      let documentDataFunction = this.config.documentsData[documentType];
      if (!documentDataFunction) {
        documentDataFunction = this.config.documentsData.$all;
      }

      for (let i = 0; i < this.config.documentsCount; i++) {
        suite.addTest(new Test(`Create document ${documentType}`, async () => {
          const documentProperties = await documentDataFunction(i, documentType);

          const document = await context.dash.platform.documents.create(
            `${this.config.title}.${documentType}`,
            context.identity,
            documentProperties,
          );

          if (this.runnerOptions.verbose) {
            // eslint-disable-next-line no-console
            console.dir(document.toJSON(), { depth: Infinity });
          }

          const stateTransition = await context.dash.platform.documents.broadcast({
            create: [document],
          }, context.identity);

          const match = createStateTransitionMatch(
            stateTransition,
            documentType,
            this.#metrics,
          );

          this.matches.push(match);
        }));
      }

      suite.addSuite(documentTypeSuite);
    }

    return suite;
  }

  /**
   * Print metrics
   */
  printResults() {
    // eslint-disable-next-line no-console
    console.log(`\n\n${this.config.title}\n${'-'.repeat(this.config.title.length)}`);

    Object.entries(this.#metrics).forEach(([documentType, metrics]) => {
      printMetrics(documentType, metrics, this.config);
    });
  }

  /**
   * @returns {number}
   */
  getRequiredCredits() {
    return this.config.requiredCredits;
  }
}

module.exports = DocumentsBenchmark;
