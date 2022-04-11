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
   * @param {Object} config
   */
  constructor(config) {
    this.config = config;
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
  printMetrics() {

  }
}

module.exports = AbstractBenchmark;
