const Dash = require('dash');
const Mocha = require('mocha');

const BENCHMARKS = require('./benchmarks');

class Runner {
  /**
   * @typedef {Mocha}
   */
  #mocha;

  constructor() {
    this.#mocha = new Mocha({
      ui: 'tdd',
      reporter: 'spec',
    });



  }

  loadBenchmarks(filePath) {
    const benchmarks = require(filePath);

    for (let benchmarkOptions of benchmarks) {
      const BenchmarkClass = BENCHMARKS[benchmarkOptions.type];

      if (!BenchmarkClass) {
        throw new Error(`Invalid benchmark type ${benchmarkOptions.type}`);
      }

      const benchmark = new BenchmarkClass(benchmarkOptions);

      this.#mocha.suite.addSuite(
        benchmark.createMochaTestSuite(this.#mocha.suite.ctx),
      );
    }
  }

  /**
   * Run benchmarks
   */
  async run(options) {
    await this.#initializeContext(options);

    this.#mocha.run();
  }

  async #initializeContext(options) {
    const context = this.#mocha.suite.ctx;

    context.dash = new Dash.Client({
      seeds: []
    });
  }
}

module.exports = Runner;
