const Worker = require('../src/plugins/Worker');

class DummyWorker extends Worker {
  constructor() {
    super({
      name: 'DummyWorker',
      dependencies: [],
      executeOnStart: true,
      workerIntervalTime: 50 * 1000,
    });
  }

  // eslint-disable-next-line class-methods-use-this
  execute() {
    console.log('Dummy worker successfully did nothing');
  }
}
module.exports = DummyWorker;
