const { Suite, Test } = require('mocha');

const { Table } = require('console-table-printer');

const mathjs = require('mathjs');

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

    suite.beforeAll('Publish Data Contract', async () => {
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
    });

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
    const overall = [];
    const validateBasic = [];
    const validateFee = [];
    const validateSignature = [];
    const validateState = [];
    const apply = [];

    this.#metrics.forEach((metric) => {
      overall.push(metric.overall);
      validateBasic.push(metric.validateBasic);
      validateFee.push(metric.validateFee);
      validateSignature.push(metric.validateSignature);
      validateState.push(metric.validateState);
      apply.push(metric.apply);
    });

    // eslint-disable-next-line no-console
    console.log(`\n\n${this.config.title}`);

    const table = new Table({
      columns: [
        { name: 'overall' },
        { name: 'validateBasic' },
        { name: 'validateFee' },
        { name: 'validateSignature' },
        { name: 'validateState' },
        { name: 'apply' },
      ],
    });

    table.addRows(this.#metrics);

    const avgFunction = mathjs[this.config.avgFunction];

    table.addRow({
      overall: avgFunction(overall).toFixed(3),
      validateBasic: avgFunction(validateBasic).toFixed(3),
      validateFee: avgFunction(validateFee).toFixed(3),
      validateSignature: avgFunction(validateSignature).toFixed(3),
      validateState: avgFunction(validateState).toFixed(3),
      apply: avgFunction(apply).toFixed(3),
    }, { color: 'white_bold', separator: true });

    table.printTable();
  }
}

module.exports = DocumentsBenchmark;
