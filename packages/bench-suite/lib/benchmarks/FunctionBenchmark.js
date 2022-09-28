const {
  Suite,
  Test,
} = require('mocha');

const { Table } = require('console-table-printer');

const { performance, PerformanceObserver } = require('perf_hooks');

const mathjs = require('mathjs');

const AbstractBenchmark = require('./AbstractBenchmark');

class FunctionBenchmark extends AbstractBenchmark {
  /**
   * @type {Object<string, PerformanceMeasure[][]>}
   */
  #perfMeasures = {};

  /**
   * @param {Context} context
   * @param {Client} context.dash
   * @param {Identity} context.identity
   * @returns {Mocha.Suite}
   */
  createMochaTestSuite(context) {
    const benchmarkSuite = new Suite(this.config.title, context);

    benchmarkSuite.timeout(this.config.timeout);

    if (this.config.beforeAll) {
      benchmarkSuite.beforeAll(this.config.beforeAll.bind(benchmarkSuite.ctx, benchmarkSuite.ctx));
    }

    if (this.config.afterAll) {
      benchmarkSuite.beforeAll(this.config.beforeAll.bind(benchmarkSuite.ctx, benchmarkSuite.ctx));
    }

    for (const [title, functions] of Object.entries(this.config.tests)) {
      const testSuite = new Suite(title, benchmarkSuite.ctx);

      this.#perfMeasures[title] = [];

      const perfObserver = new PerformanceObserver((list) => {
        this.#perfMeasures[title].push(list.getEntries());
      });

      testSuite.beforeAll('Start performance observer', () => {
        perfObserver.observe({ entryTypes: ['measure', 'function'] });
      });

      if (functions.beforeAll) {
        testSuite.beforeAll(functions.beforeAll.bind(testSuite.ctx, testSuite.ctx));
      }

      if (functions.beforeEach) {
        testSuite.beforeEach(functions.beforeEach.bind(testSuite.ctx, testSuite.ctx));
      }

      if (functions.afterAll) {
        testSuite.afterAll(functions.afterAll.bind(testSuite.ctx, testSuite.ctx));
      }

      if (functions.afterEach) {
        testSuite.afterEach(functions.afterEach.bind(testSuite.ctx, testSuite.ctx));
      }

      testSuite.afterAll('Stop performance observer', () => {
        performance.clearMeasures();
        performance.clearMarks();
        perfObserver.disconnect();
      });

      const measureName = 'overall';

      for (let i = 0; i < this.config.repeats; i++) {
        testSuite.addTest(new Test(`Test ${i}`, async () => {
          const startMarkTitle = `${measureName}-start`;
          const endMarkTitle = `${measureName}-end`;

          performance.mark(startMarkTitle);

          await functions.test(testSuite.ctx, i);

          performance.mark(endMarkTitle);

          performance.measure(measureName, startMarkTitle, endMarkTitle);
        }));
      }

      benchmarkSuite.addSuite(testSuite);
    }

    return benchmarkSuite;
  }

  printResults() {
    // eslint-disable-next-line no-console
    console.log(`\n\n${this.config.title}\n${'-'.repeat(this.config.title.length)}`);

    for (const [title] of Object.entries(this.config.tests)) {
      this.#printTestMeasures(title, this.#perfMeasures[title]);
    }
  }

  /**
   * @private
   * @param {string} title
   * @param {PerformanceMeasure[][]} measures
   */
  #printTestMeasures(title, measures) {
    const avgs = {};

    const rows = measures.map((testMeasures) => (
      testMeasures.reduce((row, measure) => {
        const duration = Number(measure.duration.toFixed(3));

        // eslint-disable-next-line no-param-reassign
        row[measure.name] = duration;

        if (!avgs[measure.name]) {
          avgs[measure.name] = [];
        }

        avgs[measure.name].push(duration);

        return row;
      }, {})
    ));

    const table = new Table();

    const keys = Object.keys(rows[0]);

    if (this.config.avgOnly) {
      const avgRow = {};

      // eslint-disable-next-line array-callback-return
      keys.map((key) => {
        avgRow[key] = '...';
      });

      table.addRow(avgRow);
    } else {
      table.addRows(rows);
    }

    const avgFunction = mathjs[this.config.avgFunction];

    const avgRow = {};

    keys.forEach((key) => {
      avgRow[key] = avgFunction(avgs[key]).toFixed(3);
    });

    table.addRow(avgRow, {
      color: 'white_bold',
      separator: true,
    });

    // eslint-disable-next-line no-console
    console.log(`\n\n${title} tests ran ${this.config.repeats} times:`);

    table.printTable();
  }
}

module.exports = FunctionBenchmark;
