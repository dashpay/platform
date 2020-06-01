/* eslint-disable no-console */
const { Worker } = require('../../src/plugins');

class HelloWorldWorker extends Worker {
  constructor() {
    // noinspection PointlessArithmeticExpressionJS
    super({
      executeOnStart: true,
      firstExecutionRequired: true,
      workerIntervalTime: 1 * 60 * 1000,
    });
  }

  // eslint-disable-next-line class-methods-use-this
  execute() {
    console.log('HELLO WORLD');
  }
}
module.exports = HelloWorldWorker;
