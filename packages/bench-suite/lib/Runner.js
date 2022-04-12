const Mocha = require('mocha');

const setupContext = require('./setupContext');

const DriveMetricsCollector = require('./metrics/drive/DriveMetricsCollector');

const BENCHMARKS = require('./benchmarks');

class Runner {
  /**
   * @type {Mocha}
   */
  #mocha;

  /**
   * @type {Object}
   */
  #options;

  /**
   * @type {DriveMetricsCollector}
   */
  #driveMetricsCollector;

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

    this.#driveMetricsCollector = new DriveMetricsCollector(options.driveLogPath);
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

      const benchmark = new BenchmarkClass(benchmarkConfig, this.#driveMetricsCollector);

      this.#mocha.suite.addSuite(
        benchmark.createMochaTestSuite(this.#mocha.suite.ctx),
      );

      this.#benchmarks.push(benchmark);
    }
  }

  /**
   * Run benchmarks
   */
  run() {
    setupContext(this.#mocha, this.#benchmarks, this.#options);

    this.#mocha.run(async (failures) => {
      if (failures) {
        process.exitCode = 1;

        return;
      }

      // Collect metrics from Drive logs
      this.#benchmarks.forEach((benchmark) => {
        if (benchmark.getMetricMatches) {
          this.#driveMetricsCollector.addMatches(benchmark.getMetricMatches());
        }
      });

      await this.#driveMetricsCollector.collect();

      // Print results
      this.#benchmarks.forEach((benchmark) => {
        benchmark.printResults();
      });
    });
  }
}

module.exports = Runner;
