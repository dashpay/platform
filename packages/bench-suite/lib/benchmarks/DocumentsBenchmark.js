const { Suite, Test } = require('mocha');

const { Table } = require('console-table-printer');

const mathjs = require('mathjs');

const crypto = require('crypto');

const AbstractBenchmark = require('./AbstractBenchmark');
const Match = require('../metrics/Match');

class DocumentsBenchmark extends AbstractBenchmark {
  /**
   * @type {Object}
   */
  #metrics = {};

  /**
   * @type {Object}
   */
  #documentCounts = {}

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

      for (const documentProperties of this.config.documents(documentType)) {
        suite.addTest(new Test(`Create document ${documentType}`, async () => {
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
            if (!this.#metrics[documentType]) {
              this.#metrics[documentType] = [];
            }

            this.#metrics[documentType].push({
              timings: data.timings,
              fees: data.fees,
            });
          });

          this.matches.push(match);
        }));

        if (!this.#documentCounts[documentType]) {
          this.#documentCounts[documentType] = 0;
        }

        this.#documentCounts[documentType] += 1;
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
      this.#printDocumentTypeMetrics(documentType, metrics);
    });
  }

  /**
   * @returns {number}
   */
  getRequiredCredits() {
    return this.config.requiredCredits;
  }

  /**
   * @private
   * @param {string} documentType
   * @param {{fees: Object, timings: Object}[]} metrics
   */
  #printDocumentTypeMetrics(documentType, metrics) {
    const overall = [];
    const validateBasic = [];
    const validateFee = [];
    const validateSignature = [];
    const validateState = [];
    const apply = [];

    metrics.forEach((metric) => {
      overall.push(metric.timings.overall);
      validateBasic.push(metric.timings.validateBasic);
      validateFee.push(metric.timings.validateFee);
      validateSignature.push(metric.timings.validateSignature);
      validateState.push(metric.timings.validateState);
      apply.push(metric.timings.apply);
    });

    // eslint-disable-next-line no-console
    console.log(`\n\n${this.#documentCounts[documentType]} "${documentType}" documents published and ${metrics.length} metrics collected:`);

    const timingTable = new Table({
      columns: [
        { name: 'overall' },
        { name: 'validateBasic' },
        { name: 'validateFee' },
        { name: 'validateSignature' },
        { name: 'validateState' },
        { name: 'apply' },
      ],
    });

    if (this.config.avgOnly) {
      timingTable.addRow({
        overall: '...',
        validateBasic: '...',
        validateFee: '...',
        validateSignature: '...',
        validateState: '...',
        apply: '...',
      });
    } else {
      timingTable.addRows(
        metrics.map((metric) => metric.timings),
      );
    }

    const avgFunction = mathjs[this.config.avgFunction];

    timingTable.addRow({
      overall: avgFunction(overall).toFixed(3),
      validateBasic: avgFunction(validateBasic).toFixed(3),
      validateFee: avgFunction(validateFee).toFixed(3),
      validateSignature: avgFunction(validateSignature).toFixed(3),
      validateState: avgFunction(validateState).toFixed(3),
      apply: avgFunction(apply).toFixed(3),
    }, { color: 'white_bold', separator: true });

    timingTable.printTable();

    // eslint-disable-next-line no-console
    console.log(`\n\n"${documentType}" fees:`);

    const feeTable = new Table({
      columns: [
        { name: 'storage' },
        { name: 'processing' },
        { name: 'final' },
      ],
    });

    feeTable.addRow({
      storage: metrics[0].fees.storage,
      processing: metrics[0].fees.processing,
      final: metrics[0].fees.final,
    });

    feeTable.printTable();

    // eslint-disable-next-line no-console
    console.log(`\n\n"${documentType}" fee operations:\n`);

    metrics[0].fees.operations.forEach((operation) => {
      // eslint-disable-next-line no-console
      console.log(operation);
    });
  }
}

module.exports = DocumentsBenchmark;
