const { Worker } = require('../../src/plugins');

class HelloWorldWorker extends Worker {
  constructor() {
    super({
      executeOnStart: true,
      firstExecutionRequired: true,
      workerIntervalTime: 1 * 60 * 1000,
    });
  }

  execute() {
    console.log('HELLO WORLD');
  }
}
module.exports = HelloWorldWorker;
