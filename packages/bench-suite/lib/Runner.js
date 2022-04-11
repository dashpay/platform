const Mocha = require('mocha');

const { convertCreditsToSatoshi } = require('@dashevo/dpp/lib/identity/creditsConverter');

const MetricsCollector = require('./metrics/MetricsCollector');

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
   * @type {MetricsCollector}
   */
  #metricsCollector;

  /**
   * @type {AbstractBenchmark[]}
   */
  #benchmarks = [];

  /**
   * @param {Object} options
   * @param {string} options.driveLogPath
   * @param {boolean} [options.verbose=false]
   */
  constructor(options = {}) {
    this.#options = options;

    this.#mocha = new Mocha({
      reporter: options.verbose ? 'spec' : 'nyan',
      timeout: 650000,
      bail: true,
    });

    this.#metricsCollector = new MetricsCollector(options.driveLogPath);
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

      const benchmark = new BenchmarkClass(benchmarkConfig, this.#metricsCollector);

      this.#mocha.suite.addSuite(
        benchmark.createMochaTestSuite(this.#mocha.suite.ctx),
      );

      this.#requiredCredits += benchmark.getRequiredCredits();

      this.#benchmarks.push(benchmark);
    }
  }

  /**
   * Run benchmarks
   */
  run() {
    this.#initializeContext();

    this.#mocha.run(async (failures) => {
      if (failures) {
        process.exitCode = 1;

        return;
      }

      // Print metrics
      this.#benchmarks.forEach((benchmark) => {
        this.#metricsCollector.addMatches(benchmark.getMetricMatches());
      });

      await this.#metricsCollector.collect();

      this.#benchmarks.forEach((benchmark) => {
        benchmark.printMetrics();
      });
    });
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
