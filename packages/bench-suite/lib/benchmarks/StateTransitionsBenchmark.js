const {
  Suite,
  Test,
} = require('mocha');

const AbstractBenchmark = require('./AbstractBenchmark');
const printMetrics = require('../metrics/drive/printMetrics');
const createStateTransitionMatch = require('../metrics/drive/createStateTransitionMatch');

class StateTransitionsBenchmark extends AbstractBenchmark {
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

    for (const stateTransitionTitle of Object.keys(this.config.stateTransitions)) {
      const stateTransitionSuite = new Suite(stateTransitionTitle, suite.ctx);

      for (let i = 0; i < this.config.stateTransitionsCount; i++) {
        suite.addTest(new Test(`Broadcast state transition "${i + 1}"`, async () => {
          const stateTransition = await this.config.stateTransitions[stateTransitionTitle](
            context,
            i,
          );

          if (this.runnerOptions.verbose) {
            // eslint-disable-next-line no-console
            console.dir(stateTransition.toJSON(), { depth: Infinity });
          }

          await context.dash.platform.broadcastStateTransition(stateTransition);

          const match = createStateTransitionMatch(
            stateTransition,
            stateTransitionTitle,
            this.#metrics,
          );

          this.matches.push(match);
        }));
      }

      suite.addSuite(stateTransitionSuite);
    }

    return suite;
  }

  /**
   * Print metrics
   */
  printResults() {
    // eslint-disable-next-line no-console
    console.log(`\n\n${this.config.title}\n${'-'.repeat(this.config.title.length)}`);

    Object.entries(this.#metrics)
      .forEach(([documentType, metrics]) => {
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

module.exports = StateTransitionsBenchmark;
