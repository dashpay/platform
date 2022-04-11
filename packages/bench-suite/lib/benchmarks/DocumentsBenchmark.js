const { Suite, Test } = require('mocha');

const { printTable } = require('console-table-printer');

const crypto = require('crypto');

const AbstractBenchmark = require('./AbstractBenchmark');
const Match = require('../metrics/Match');

class DocumentsBenchmark extends AbstractBenchmark {
  /**
   * @type {Object[]}
   */
  #metrics = [];

  /**
   * @param {Context} context
   * @param {Client} context.dash
   * @param {Identity} context.identity
   * @returns {Mocha.Suite}
   */
  createMochaTestSuite(context) {
    const suite = new Suite(this.config.title, context);

    suite.timeout(650000);

    const documentTypes = this.config.documentTypes();

    suite.addTest(new Test('Publish Data Contract', async () => {
      const dataContract = await context.dash.platform.contracts.create(
        documentTypes,
        context.identity,
      );

      await context.dash.platform.contracts.publish(
        dataContract,
        context.identity,
      );

      context.dash.getApps().set(this.config.title, {
        contractId: dataContract.getId(),
        contract: dataContract,
      });
    }));

    for (const documentType of Object.keys(documentTypes)) {
      const documentTypeSuite = new Suite(documentType, suite.ctx);

      for (const documentProperties of this.config.documents(documentType)) {
        suite.addTest(new Test(`Create document ${documentType}`, async () => {
          const document = await context.dash.platform.documents.create(
            `${this.config.title}.${documentType}`,
            context.identity,
            documentProperties,
          );

          const stateTransition = await context.dash.platform.documents.broadcast({
            create: [document],
          }, context.identity);

          const stHash = crypto
            .createHash('sha256')
            .update(stateTransition.toBuffer())
            .digest()
            .toString('hex')
            .toUpperCase();

          const match = new Match({
            txId: stHash,
            txType: stateTransition.getType(),
            abciMethod: 'deliverTx',
          }, (data) => {
            this.#metrics.push(data.timings);
          });

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
  printMetrics() {
    // eslint-disable-next-line no-console
    console.log(`\n\n${this.config.title}`);

    printTable(this.#metrics);
  }
}

module.exports = DocumentsBenchmark;
