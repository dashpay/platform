class AbstractBenchmark {
  /**
   * @type {Object}
   */
  config;

  /**
   * @type {Match[]}
   */
  matches = [];

  /**
   * @type {Object}
   */
  runnerOptions;

  /**
   * @param {Object} config
   * @param {Object} runnerOptions
   */
  constructor(config, runnerOptions) {
    this.config = config;
    this.runnerOptions = runnerOptions;
  }

  /**
   * @returns {number}
   */
  getRequiredCredits() {
    return this.config.requiredCredits;
  }

  /**
   * @returns {Match[]}
   */
  getMetricMatches() {
    return this.matches;
  }

  /**
   * @abstract
   * @param {Context} context
   * @param {Client} context.dash
   * @param {Identity} context.identity
   * @returns {Mocha.Suite}
   */
  // eslint-disable-next-line no-unused-vars
  createMochaTestSuite(context) {

  }

  /**
   * @abstract
   */
  printResults() {

  }
}

module.exports = AbstractBenchmark;
