const Mocha = require('mocha');

const { convertCreditsToSatoshi } = require('@dashevo/dpp/lib/identity/creditsConverter');

const BENCHMARKS = require('./benchmarks');

const createClientWithFundedWallet = require('./client/createClientWithFundedWallet');

class Runner {
  /**
   * @type {Mocha}
   */
  #mocha;

  /**
   * @type {number}
   */
  #requiredCredits = 0;

  /**
   * @type {Object}
   */
  #options;

  /**
   * @param {Object} [options]
   */
  constructor(options = {}) {
    this.#options = options;

    this.#mocha = new Mocha({
      reporter: options.verbose ? 'spec' : 'nyan',
      timeout: 650000,
      bail: true,
      exit: true,
    });
  }

  /**
   * @param {string} filePath
   */
  loadBenchmarks(filePath) {
    // eslint-disable-next-line global-require,import/no-dynamic-require
    const benchmarks = require(filePath);

    for (const benchmarkConfig of benchmarks) {
      const BenchmarkClass = BENCHMARKS[benchmarkConfig.type];

      if (!BenchmarkClass) {
        throw new Error(`Invalid benchmark type ${benchmarkConfig.type}`);
      }

      const benchmark = new BenchmarkClass(benchmarkConfig);

      this.#mocha.suite.addSuite(
        benchmark.createMochaTestSuite(this.#mocha.suite.ctx),
      );

      this.#requiredCredits += benchmark.getRequiredCredits();
    }
  }

  /**
   * Run benchmarks
   */
  run() {
    this.#initializeContext();

    this.#mocha.run();
  }

  /**
   * @returns {void}
   */
  #initializeContext() {
    const context = this.#mocha.suite.ctx;

    let satoshis = convertCreditsToSatoshi(this.#requiredCredits);

    if (satoshis < 10000) {
      satoshis = 10000;
    }

    this.#mocha.suite.beforeAll('Create and connect client', async () => {
      context.dash = await createClientWithFundedWallet(satoshis + 5000);
    });

    this.#mocha.suite.beforeAll('Create identity', async () => {
      context.identity = await context.dash.platform.identities.register(satoshis);
    });

    this.#mocha.suite.afterAll('Disconnect client', async () => {
      if (context.dash) {
        await context.dash.disconnect();
      }
    });
  }
}

module.exports = Runner;
